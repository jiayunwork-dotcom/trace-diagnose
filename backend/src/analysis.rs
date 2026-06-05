use crate::models::*;
use crate::db::DbPool;
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use chrono::{DateTime, Utc, Duration, Datelike};
use serde_json::json;
use sqlx::FromRow;

pub async fn rebuild_trace_topology(pool: &DbPool, trace_id: &str) -> Result<()> {
    let spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1 ORDER BY start_time",
        trace_id
    )
    .fetch_all(pool)
    .await?;

    if spans.is_empty() {
        return Ok(());
    }

    let root_span = spans.iter()
        .find(|s| s.parent_span_id.is_none() || s.parent_span_id.as_ref().map(|p| p.is_empty()).unwrap_or(true))
        .or_else(|| spans.first());

    let start_time = spans.iter().map(|s| s.start_time).min().unwrap();
    let end_time = spans.iter().map(|s| s.end_time).max().unwrap();
    let duration_ms = (end_time - start_time).num_milliseconds();
    let services: HashSet<_> = spans.iter().map(|s| s.service_name.as_str()).collect();
    let has_errors = spans.iter().any(|s| s.status_code >= 400 || s.is_anomaly);

    sqlx::query!(
        r#"
        INSERT INTO traces (
            trace_id, root_span_id, start_time, end_time, duration_ms,
            service_count, span_count, has_errors
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (trace_id) DO UPDATE SET
            root_span_id = EXCLUDED.root_span_id,
            start_time = EXCLUDED.start_time,
            end_time = EXCLUDED.end_time,
            duration_ms = EXCLUDED.duration_ms,
            service_count = EXCLUDED.service_count,
            span_count = EXCLUDED.span_count,
            has_errors = EXCLUDED.has_errors
        "#,
        trace_id,
        root_span.map(|s| s.span_id.clone()),
        start_time,
        end_time,
        duration_ms,
        services.len() as i32,
        spans.len() as i32,
        has_errors,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_service_dependencies(pool: &DbPool) -> Result<()> {
    let deps = sqlx::query_as!(
        ServiceDependency,
        r#"
        SELECT
            id,
            caller_service,
            callee_service,
            call_count,
            total_duration_ms,
            avg_duration_ms,
            error_count,
            last_updated
        FROM service_dependencies
        "#
    )
    .fetch_all(pool)
    .await?;

    let edges = sqlx::query!(
        r#"
        SELECT
            parent.service_name AS caller,
            child.service_name AS callee,
            COUNT(*) as call_count,
            SUM(child.duration_ms) as total_duration,
            COUNT(*) FILTER (WHERE child.status_code >= 400) as error_count
        FROM spans child
        JOIN spans parent ON child.parent_span_id = parent.span_id AND child.trace_id = parent.trace_id
        WHERE parent.service_name != child.service_name
        GROUP BY parent.service_name, child.service_name
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut tx = pool.begin().await?;

    for edge in edges {
        let avg_duration = if edge.call_count > 0 {
            edge.total_duration.unwrap_or(0) as f64 / edge.call_count as f64
        } else {
            0.0
        };

        sqlx::query!(
            r#"
            INSERT INTO service_dependencies (caller_service, callee_service, call_count, total_duration_ms, avg_duration_ms, error_count, last_updated)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (caller_service, callee_service) DO UPDATE SET
                call_count = EXCLUDED.call_count,
                total_duration_ms = EXCLUDED.total_duration_ms,
                avg_duration_ms = EXCLUDED.avg_duration_ms,
                error_count = EXCLUDED.error_count,
                last_updated = NOW()
            "#,
            edge.caller,
            edge.callee,
            edge.call_count,
            edge.total_duration.unwrap_or(0),
            avg_duration,
            edge.error_count.unwrap_or(0),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn get_topology(pool: &DbPool) -> Result<TopologyResponse> {
    let services = sqlx::query!(
        r#"
        SELECT
            service_name,
            COUNT(*) as call_count,
            AVG(duration_ms) as avg_duration,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count
        FROM spans
        GROUP BY service_name
        "#
    )
    .fetch_all(pool)
    .await?;

    let deps = sqlx::query_as!(
        ServiceDependency,
        r#"SELECT id, caller_service, callee_service, call_count, total_duration_ms, avg_duration_ms, error_count, last_updated FROM service_dependencies"#
    )
    .fetch_all(pool)
    .await?;

    let mut nodes = Vec::new();
    let now = Utc::now();
    let one_hour_ago = now - Duration::hours(1);

    for svc in services {
        let qps_data = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM spans
            WHERE service_name = $1 AND start_time >= $2
            "#,
            svc.service_name,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let qps = qps_data.count.unwrap_or(0) as f64 / 3600.0;
        let total = svc.call_count.unwrap_or(0);
        let error_rate = if total > 0 {
            svc.error_count.unwrap_or(0) as f64 / total as f64
        } else {
            0.0
        };

        nodes.push(TopologyNode {
            id: svc.service_name.clone(),
            service_name: svc.service_name.clone(),
            qps,
            error_rate,
            avg_duration_ms: svc.avg_duration.unwrap_or(0.0),
        });
    }

    let edges = deps
        .into_iter()
        .map(|d| {
            let error_rate = if d.call_count > 0 {
                d.error_count as f64 / d.call_count as f64
            } else {
                0.0
            };
            TopologyEdge {
                source: d.caller_service,
                target: d.callee_service,
                call_count: d.call_count,
                avg_duration_ms: d.avg_duration_ms,
                error_rate,
            }
        })
        .collect();

    Ok(TopologyResponse { nodes, edges })
}

pub async fn update_service_metrics_for_trace(pool: &DbPool, trace_id: &str) -> Result<()> {
    let spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1",
        trace_id
    )
    .fetch_all(pool)
    .await?;

    let mut buckets: HashMap<(String, String, DateTime<Utc>), Vec<i64>> = HashMap::new();

    for span in &spans {
        let bucket_time = span.start_time
            .with_minute(0)
            .unwrap_or(span.start_time)
            .with_second(0)
            .unwrap_or(span.start_time)
            .with_nanosecond(0)
            .unwrap_or(span.start_time);
        let key = (span.service_name.clone(), span.operation_name.clone(), bucket_time);
        buckets.entry(key).or_default().push(span.duration_ms);
    }

    let mut tx = pool.begin().await?;

    for ((service, operation, bucket), durations) in buckets {
        let call_count = durations.len() as i64;
        let total_duration: i64 = durations.iter().sum();
        let min_duration = *durations.iter().min().unwrap_or(&0);
        let max_duration = *durations.iter().max().unwrap_or(&0);
        let mut sorted = durations.clone();
        sorted.sort();

        let p50 = percentile(&sorted, 50.0);
        let p95 = percentile(&sorted, 95.0);
        let p99 = percentile(&sorted, 99.0);

        let histogram = build_histogram(&durations);

        sqlx::query!(
            r#"
            INSERT INTO service_metrics (
                service_name, operation_name, time_bucket, call_count, error_count,
                total_duration_ms, min_duration_ms, max_duration_ms, p50_ms, p95_ms, p99_ms,
                duration_histogram
            ) VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (service_name, operation_name, time_bucket) DO UPDATE SET
                call_count = service_metrics.call_count + EXCLUDED.call_count,
                total_duration_ms = service_metrics.total_duration_ms + EXCLUDED.total_duration_ms,
                min_duration_ms = LEAST(service_metrics.min_duration_ms, EXCLUDED.min_duration_ms),
                max_duration_ms = GREATEST(service_metrics.max_duration_ms, EXCLUDED.max_duration_ms)
            "#,
            service,
            operation,
            bucket,
            call_count,
            total_duration,
            min_duration,
            max_duration,
            p50,
            p95,
            p99,
            json!(histogram),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

fn percentile(sorted: &[i64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx] as f64
}

fn build_histogram(durations: &[i64]) -> HashMap<String, i64> {
    let bounds = vec![0, 1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];
    let mut histogram = HashMap::new();

    for d in durations {
        for window in bounds.windows(2) {
            if *d >= window[0] && *d < window[1] {
                let key = format!("{}-{}", window[0], window[1]);
                *histogram.entry(key).or_insert(0) += 1;
                break;
            }
        }
    }

    histogram
}

pub async fn get_latency_distribution(
    pool: &DbPool,
    service_name: Option<&str>,
    operation_name: Option<&str>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
) -> Result<LatencyDistribution> {
    let spans = match (service_name, operation_name) {
        (Some(svc), Some(op)) => {
            sqlx::query!(
                "SELECT duration_ms FROM spans WHERE service_name = $1 AND operation_name = $2",
                svc, op
            )
            .fetch_all(pool)
            .await?
        }
        (Some(svc), None) => {
            sqlx::query!(
                "SELECT duration_ms FROM spans WHERE service_name = $1",
                svc
            )
            .fetch_all(pool)
            .await?
        }
        (None, Some(op)) => {
            sqlx::query!(
                "SELECT duration_ms FROM spans WHERE operation_name = $1",
                op
            )
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query!("SELECT duration_ms FROM spans")
                .fetch_all(pool)
                .await?
        }
    };

    let mut durations: Vec<i64> = spans.iter()
        .map(|r| r.duration_ms.unwrap_or(0))
        .collect();
    durations.sort();

    let p50 = percentile(&durations, 50.0);
    let p95 = percentile(&durations, 95.0);
    let p99 = percentile(&durations, 99.0);

    let buckets = build_histogram_buckets(&durations);

    Ok(LatencyDistribution {
        buckets,
        p50,
        p95,
        p99,
    })
}

fn build_histogram_buckets(durations: &[i64]) -> Vec<LatencyBucket> {
    let bounds = vec![0, 1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, i64::MAX];
    let mut buckets = Vec::new();

    for window in bounds.windows(2) {
        let count = durations.iter().filter(|&&d| d >= window[0] && d < window[1]).count() as i64;
        buckets.push(LatencyBucket {
            min_ms: window[0],
            max_ms: if window[1] == i64::MAX { window[0] * 2 } else { window[1] },
            count,
        });
    }

    buckets
}

pub async fn detect_anomalies_in_trace(pool: &DbPool, trace_id: &str) -> Result<()> {
    let spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1",
        trace_id
    )
    .fetch_all(pool)
    .await?;

    let threshold_ms = 3000;
    let mut anomaly_spans = Vec::new();

    for span in &spans {
        let mut reasons = Vec::new();
        if span.duration_ms > threshold_ms {
            reasons.push(format!("High latency: {}ms > {}ms threshold", span.duration_ms, threshold_ms));
        }
        if span.status_code >= 400 {
            reasons.push(format!("Error status code: {}", span.status_code));
        }

        if !reasons.is_empty() {
            anomaly_spans.push((span, reasons.join("; ")));
        }
    }

    let children_map: HashMap<&str, Vec<&Span>> = spans
        .iter()
        .fold(HashMap::new(), |mut map, span| {
            if let Some(parent_id) = &span.parent_span_id {
                map.entry(parent_id.as_str()).or_default().push(span);
            }
            map
        });

    let is_span_anomalous = |span: &Span| -> bool {
        span.duration_ms > threshold_ms || span.status_code >= 400
    };

    for (span, reason) in &anomaly_spans {
        let root_cause = find_root_cause(span, &children_map, &is_span_anomalous);

        let severity = if span.duration_ms > threshold_ms * 3 || span.status_code >= 500 {
            "critical"
        } else if span.duration_ms > threshold_ms * 2 || span.status_code >= 400 {
            "warning"
        } else {
            "info"
        };

        let affected = trace_affected_services(&spans, span);

        sqlx::query!(
            r#"
            INSERT INTO anomaly_traces (
                trace_id, anomaly_type, severity, root_service, root_operation,
                affected_services, details
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            trace_id,
            if span.status_code >= 400 { "error" } else { "latency" },
            severity,
            Some(root_cause.service_name.clone()),
            Some(root_cause.operation_name.clone()),
            &affected.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            json!({
                "reason": reason,
                "span_id": span.span_id,
                "duration_ms": span.duration_ms,
                "status_code": span.status_code,
                "root_cause_span_id": root_cause.span_id
            }),
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn find_root_cause<'a, F>(
    start_span: &'a Span,
    children_map: &HashMap<&str, Vec<&'a Span>>,
    is_anomalous: F,
) -> &'a Span
where
    F: Fn(&Span) -> bool,
{
    let mut current = start_span;
    loop {
        let children = children_map.get(current.span_id.as_str()).unwrap_or(&vec![]);
        let anomalous_children: Vec<_> = children.iter().filter(|c| is_anomalous(c)).collect();

        if anomalous_children.is_empty() {
            return current;
        }

        current = anomalous_children.iter().max_by_key(|c| c.duration_ms).unwrap();
    }
}

fn trace_affected_services(spans: &[Span], start_span: &Span) -> Vec<String> {
    let mut affected = HashSet::new();
    affected.insert(start_span.service_name.clone());

    let span_map: HashMap<&str, &Span> = spans.iter().map(|s| (s.span_id.as_str(), s)).collect();

    let mut queue = VecDeque::new();
    queue.push_back(start_span.span_id.as_str());

    while let Some(span_id) = queue.pop_front() {
        if let Some(span) = span_map.get(span_id) {
            affected.insert(span.service_name.clone());
            if let Some(parent_id) = &span.parent_span_id {
                queue.push_back(parent_id.as_str());
            }
        }
    }

    affected.into_iter().collect()
}

pub async fn find_critical_path(pool: &DbPool, trace_id: &str) -> Result<CriticalPath> {
    let spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1 ORDER BY start_time",
        trace_id
    )
    .fetch_all(pool)
    .await?;

    if spans.is_empty() {
        anyhow::bail!("Trace not found");
    }

    let span_map: HashMap<&str, &Span> = spans.iter().map(|s| (s.span_id.as_str(), s)).collect();
    let mut children_map: HashMap<&str, Vec<&Span>> = HashMap::new();

    for span in &spans {
        if let Some(parent_id) = &span.parent_span_id {
            children_map.entry(parent_id.as_str()).or_default().push(span);
        }
    }

    let root = spans.iter().find(|s| s.parent_span_id.is_none()).unwrap_or(&spans[0]);
    let total_duration = root.duration_ms;

    let mut critical_path = Vec::new();
    let mut current = root;

    loop {
        critical_path.push(current);
        let children = children_map.get(current.span_id.as_str()).unwrap_or(&vec![]);
        if children.is_empty() {
            break;
        }
        current = children.iter().max_by_key(|c| c.duration_ms).unwrap();
    }

    let path_spans = critical_path
        .into_iter()
        .map(|s| CriticalPathSpan {
            span_id: s.span_id.clone(),
            service_name: s.service_name.clone(),
            operation_name: s.operation_name.clone(),
            duration_ms: s.duration_ms,
            contribution_pct: if total_duration > 0 {
                s.duration_ms as f64 / total_duration as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(CriticalPath {
        trace_id: trace_id.to_string(),
        total_duration,
        spans: path_spans,
        parallel_optimization_suggestions: vec![],
    })
}

pub async fn compare_traces(
    pool: &DbPool,
    baseline_id: &str,
    comparison_id: &str,
) -> Result<TraceComparison> {
    let baseline_spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1",
        baseline_id
    )
    .fetch_all(pool)
    .await?;

    let comparison_spans = sqlx::query_as!(
        Span,
        "SELECT * FROM spans WHERE trace_id = $1",
        comparison_id
    )
    .fetch_all(pool)
    .await?;

    let baseline_map: HashMap<(String, String), &Span> = baseline_spans
        .iter()
        .map(|s| ((s.service_name.clone(), s.operation_name.clone()), s))
        .collect();

    let comparison_map: HashMap<(String, String), &Span> = comparison_spans
        .iter()
        .map(|s| ((s.service_name.clone(), s.operation_name.clone()), s))
        .collect();

    let mut differences = Vec::new();

    for (key, comp_span) in &comparison_map {
        if let Some(base_span) = baseline_map.get(key) {
            let change = comp_span.duration_ms - base_span.duration_ms;
            let change_pct = if base_span.duration_ms > 0 {
                change as f64 / base_span.duration_ms as f64 * 100.0
            } else {
                0.0
            };

            if change_pct.abs() > 10.0 {
                differences.push(TraceDifference {
                    difference_type: "latency_change".to_string(),
                    service_name: key.0.clone(),
                    operation_name: key.1.clone(),
                    baseline_duration_ms: Some(base_span.duration_ms),
                    comparison_duration_ms: Some(comp_span.duration_ms),
                    duration_change_pct: Some(change_pct),
                    description: format!("Latency changed by {:.1}% ({}ms -> {}ms)",
                        change_pct, base_span.duration_ms, comp_span.duration_ms),
                });
            }
        } else {
            differences.push(TraceDifference {
                difference_type: "added".to_string(),
                service_name: key.0.clone(),
                operation_name: key.1.clone(),
                baseline_duration_ms: None,
                comparison_duration_ms: Some(comp_span.duration_ms),
                duration_change_pct: None,
                description: format!("New span added: {} / {}", key.0, key.1),
            });
        }
    }

    for (key, base_span) in &baseline_map {
        if !comparison_map.contains_key(key) {
            differences.push(TraceDifference {
                difference_type: "removed".to_string(),
                service_name: key.0.clone(),
                operation_name: key.1.clone(),
                baseline_duration_ms: Some(base_span.duration_ms),
                comparison_duration_ms: None,
                duration_change_pct: None,
                description: format!("Span removed: {} / {}", key.0, key.1),
            });
        }
    }

    Ok(TraceComparison {
        baseline_trace_id: baseline_id.to_string(),
        comparison_trace_id: comparison_id.to_string(),
        differences,
    })
}

pub async fn calculate_slo_status(pool: &DbPool, slo_id: uuid::Uuid) -> Result<()> {
    let slo = sqlx::query_as!(
        SloConfig,
        "SELECT * FROM slo_configs WHERE id = $1",
        slo_id
    )
    .fetch_one(pool)
    .await?;

    let now = Utc::now().date_naive();
    let window_start = now - Duration::days(slo.window_days as i64);

    let metrics = sqlx::query!(
        r#"
        SELECT
            SUM(call_count) as total_calls,
            SUM(error_count) as total_errors,
            AVG(p95_ms) as avg_p95
        FROM service_metrics
        WHERE service_name = $1 AND time_bucket >= $2
        "#,
        slo.service_name,
        chrono::NaiveDateTime::new(window_start, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()).and_utc(),
    )
    .fetch_one(pool)
    .await?;

    let current_value = match slo.slo_type.as_str() {
        "latency_p95" => metrics.avg_p95.unwrap_or(0.0),
        "error_rate" => {
            let total = metrics.total_calls.unwrap_or(0);
            if total > 0 {
                metrics.total_errors.unwrap_or(0) as f64 / total as f64 * 100.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    let error_budget = 100.0 - slo.target;
    let actual_error = (current_value - slo.threshold).max(0.0);
    let budget_remaining = ((error_budget - actual_error) / error_budget * 100.0).max(0.0);

    sqlx::query!(
        r#"
        INSERT INTO slo_status (slo_id, date, current_value, error_budget_remaining, is_breached)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (slo_id, date) DO UPDATE SET
            current_value = EXCLUDED.current_value,
            error_budget_remaining = EXCLUDED.error_budget_remaining,
            is_breached = EXCLUDED.is_breached
        "#,
        slo_id,
        now,
        current_value,
        budget_remaining,
        budget_remaining <= 0.0,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn evaluate_alert_rules(pool: &DbPool) -> Result<Vec<AlertEvent>> {
    let rules = sqlx::query_as!(
        AlertRule,
        "SELECT * FROM alert_rules WHERE is_active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    let mut new_events = Vec::new();

    for rule in rules {
        if let Some(last_triggered) = rule.last_triggered_at {
            let silence_until = last_triggered + Duration::minutes(rule.silence_minutes as i64);
            if Utc::now() < silence_until {
                continue;
            }
        }

        let should_trigger = evaluate_rule_condition(pool, &rule).await?;

        if should_trigger {
            let event = create_alert_event(pool, &rule).await?;
            new_events.push(event);

            sqlx::query!(
                "UPDATE alert_rules SET last_triggered_at = NOW(), updated_at = NOW() WHERE id = $1",
                rule.id
            )
            .execute(pool)
            .await?;
        } else {
            resolve_recovered_alerts(pool, &rule).await?;
        }
    }

    Ok(new_events)
}

async fn evaluate_rule_condition(pool: &DbPool, rule: &AlertRule) -> Result<bool> {
    let window_duration = Duration::minutes(rule.window_minutes as i64);
    let now = Utc::now();
    let mut consecutive_count = 0;

    for i in 0..rule.consecutive_windows {
        let window_end = now - Duration::minutes((i * rule.window_minutes) as i64);
        let window_start = window_end - window_duration;

        let metric_value = calculate_metric_value(
            pool,
            &rule.metric_type,
            rule.service_name.as_deref(),
            rule.operation_name.as_deref(),
            window_start,
            window_end,
        ).await?;

        let threshold_met = match rule.comparison_operator.as_str() {
            ">" => metric_value > rule.threshold,
            ">=" => metric_value >= rule.threshold,
            "<" => metric_value < rule.threshold,
            "<=" => metric_value <= rule.threshold,
            "==" => (metric_value - rule.threshold).abs() < f64::EPSILON,
            _ => metric_value > rule.threshold,
        };

        if threshold_met {
            consecutive_count += 1;
        } else {
            break;
        }
    }

    Ok(consecutive_count >= rule.consecutive_windows)
}

async fn calculate_metric_value(
    pool: &DbPool,
    metric_type: &str,
    service_name: Option<&str>,
    operation_name: Option<&str>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<f64> {
    let mut sql = String::new();

    match metric_type {
        "latency_p95" => {
            sql = "SELECT AVG(p95_ms) as value FROM service_metrics WHERE time_bucket >= $1 AND time_bucket < $2".to_string();
        }
        "latency_p50" => {
            sql = "SELECT AVG(p50_ms) as value FROM service_metrics WHERE time_bucket >= $1 AND time_bucket < $2".to_string();
        }
        "error_rate" => {
            sql = "SELECT CASE WHEN SUM(call_count) > 0 THEN SUM(error_count)::FLOAT / SUM(call_count) * 100 ELSE 0 END as value FROM service_metrics WHERE time_bucket >= $1 AND time_bucket < $2".to_string();
        }
        "qps" => {
            sql = "SELECT SUM(call_count)::FLOAT / EXTRACT(EPOCH FROM ($2::TIMESTAMPTZ - $1::TIMESTAMPTZ)) as value FROM service_metrics WHERE time_bucket >= $1 AND time_bucket < $2".to_string();
        }
        _ => {
            return Ok(0.0);
        }
    }

    if service_name.is_some() {
        sql.push_str(" AND service_name = $3");
    }
    if operation_name.is_some() {
        let param_index = if service_name.is_some() { 4 } else { 3 };
        sql.push_str(&format!(" AND operation_name = ${}", param_index));
    }

    let value: Option<f64> = if service_name.is_some() && operation_name.is_some() {
        sqlx::query_scalar(&sql)
            .bind(start_time)
            .bind(end_time)
            .bind(service_name.unwrap())
            .bind(operation_name.unwrap())
            .fetch_one(pool)
            .await?
    } else if service_name.is_some() {
        sqlx::query_scalar(&sql)
            .bind(start_time)
            .bind(end_time)
            .bind(service_name.unwrap())
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar(&sql)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(pool)
            .await?
    };

    Ok(value.unwrap_or(0.0))
}

async fn create_alert_event(pool: &DbPool, rule: &AlertRule) -> Result<AlertEvent> {
    let window_duration = Duration::minutes(rule.window_minutes as i64);
    let now = Utc::now();
    let window_start = now - window_duration;

    let metric_value = calculate_metric_value(
        pool,
        &rule.metric_type,
        rule.service_name.as_deref(),
        rule.operation_name.as_deref(),
        window_start,
        now,
    ).await?;

    let trace_ids = if let Some(svc_name) = &rule.service_name {
        let traces = sqlx::query!(
            r#"
            SELECT DISTINCT t.trace_id
            FROM traces t
            JOIN spans s ON t.trace_id = s.trace_id
            WHERE s.service_name = $1
              AND t.start_time >= $2
              AND (t.has_errors = TRUE OR t.duration_ms > 3000)
            ORDER BY t.start_time DESC
            LIMIT 10
            "#,
            svc_name,
            window_start
        )
        .fetch_all(pool)
        .await?;
        traces.iter().map(|t| t.trace_id.clone()).collect()
    } else {
        vec![]
    };

    let event = sqlx::query_as!(
        AlertEvent,
        r#"
        INSERT INTO alert_events (
            rule_id, rule_name, service_name, operation_name,
            metric_type, metric_value, threshold, status, trace_ids
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'firing', $8)
        RETURNING *
        "#,
        rule.id,
        rule.name,
        rule.service_name,
        rule.operation_name,
        rule.metric_type,
        metric_value,
        rule.threshold,
        &trace_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    )
    .fetch_one(pool)
    .await?;

    Ok(event)
}

async fn resolve_recovered_alerts(pool: &DbPool, rule: &AlertRule) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE alert_events
        SET status = 'resolved', resolved_at = NOW()
        WHERE rule_id = $1 AND status = 'firing'
        "#,
        rule.id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn batch_compare_traces(
    pool: &DbPool,
    baseline_start: DateTime<Utc>,
    baseline_end: DateTime<Utc>,
    comparison_start: DateTime<Utc>,
    comparison_end: DateTime<Utc>,
) -> Result<Vec<BatchComparisonResult>> {
    let baseline_metrics = sqlx::query!(
        r#"
        SELECT
            service_name,
            operation_name,
            AVG(total_duration_ms::FLOAT / NULLIF(call_count, 0)) as avg_duration,
            AVG(p95_ms) as p95,
            SUM(call_count) as call_count
        FROM service_metrics
        WHERE time_bucket >= $1 AND time_bucket < $2
        GROUP BY service_name, operation_name
        "#,
        baseline_start,
        baseline_end
    )
    .fetch_all(pool)
    .await?;

    let comparison_metrics = sqlx::query!(
        r#"
        SELECT
            service_name,
            operation_name,
            AVG(total_duration_ms::FLOAT / NULLIF(call_count, 0)) as avg_duration,
            AVG(p95_ms) as p95,
            SUM(call_count) as call_count
        FROM service_metrics
        WHERE time_bucket >= $1 AND time_bucket < $2
        GROUP BY service_name, operation_name
        "#,
        comparison_start,
        comparison_end
    )
    .fetch_all(pool)
    .await?;

    let baseline_map: HashMap<(String, String), _> = baseline_metrics
        .into_iter()
        .map(|m| {
            (
                (m.service_name.clone(), m.operation_name.clone()),
                m,
            )
        })
        .collect();

    let mut results = Vec::new();

    for comp in comparison_metrics {
        let key = (comp.service_name.clone(), comp.operation_name.clone());
        if let Some(base) = baseline_map.get(&key) {
            let base_avg = base.avg_duration.unwrap_or(0.0);
            let comp_avg = comp.avg_duration.unwrap_or(0.0);
            let change_pct = if base_avg > 0.0 {
                (comp_avg - base_avg) / base_avg * 100.0
            } else {
                0.0
            };

            results.push(BatchComparisonResult {
                service_name: comp.service_name,
                operation_name: comp.operation_name,
                baseline_avg_duration: base_avg,
                comparison_avg_duration: comp_avg,
                duration_change_pct: change_pct,
                baseline_p95: base.p95.unwrap_or(0.0),
                comparison_p95: comp.p95.unwrap_or(0.0),
                baseline_call_count: base.call_count.unwrap_or(0),
                comparison_call_count: comp.call_count.unwrap_or(0),
                is_regression: change_pct >= 20.0,
            });
        }
    }

    results.sort_by(|a, b| b.duration_change_pct.partial_cmp(&a.duration_change_pct).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}
