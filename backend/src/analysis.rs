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

    for (span, reason) in &anomaly_spans {
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
            Some(span.service_name.clone()),
            Some(span.operation_name.clone()),
            &affected.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            json!({ "reason": reason, "span_id": span.span_id, "duration_ms": span.duration_ms, "status_code": span.status_code }),
        )
        .execute(pool)
        .await?;
    }

    Ok(())
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
