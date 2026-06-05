use crate::db::DbPool;
use crate::models::*;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration, Timelike};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

const DEFAULT_BASELINE_P99: f64 = 3000.0;
const AVAILABILITY_WEIGHT: f64 = 0.4;
const LATENCY_WEIGHT: f64 = 0.3;
const THROUGHPUT_WEIGHT: f64 = 0.2;
const ERROR_DIVERSITY_WEIGHT: f64 = 0.1;
const WARNING_THRESHOLD: f64 = 60.0;
const CAPACITY_WARNING_THRESHOLD_PCT: f64 = 20.0;
const MAX_QPS_CAP: f64 = 10000.0;

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
}

fn linear_interpolate(value: f64, min_val: f64, max_val: f64, min_score: f64, max_score: f64) -> f64 {
    if value <= min_val {
        return max_score;
    }
    if value >= max_val {
        return min_score;
    }
    let ratio = (value - min_val) / (max_val - min_val);
    max_score - ratio * (max_score - min_score)
}

pub async fn update_health_baselines(pool: &DbPool) -> Result<()> {
    tracing::info!("Updating health baselines...");
    
    let seven_days_ago = Utc::now() - Duration::days(7);
    
    let ops = sqlx::query!(
        r#"
        SELECT DISTINCT service_name, operation_name
        FROM spans
        WHERE start_time >= $1
        "#,
        seven_days_ago
    )
    .fetch_all(pool)
    .await?;

    for op in ops {
        let service_name = &op.service_name;
        let operation_name = &op.operation_name;

        for hour in 0..24 {
            let p99_values = sqlx::query!(
                r#"
                SELECT p99_ms
                FROM service_metrics
                WHERE service_name = $1
                  AND operation_name = $2
                  AND EXTRACT(HOUR FROM time_bucket) = $3
                  AND time_bucket >= $4
                ORDER BY time_bucket
                "#,
                service_name,
                operation_name,
                hour as f64,
                seven_days_ago
            )
            .fetch_all(pool)
            .await?;

            let mut values: Vec<f64> = p99_values
                .iter()
                .filter_map(|r| r.p99_ms)
                .collect();
            
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            
            let baseline_p99 = if !values.is_empty() {
                percentile(&values, 50.0)
            } else {
                DEFAULT_BASELINE_P99
            };

            let data_points = values.len() as i32;

            sqlx::query!(
                r#"
                INSERT INTO health_baselines (
                    service_name, operation_name, hour_of_day, baseline_p99_ms, data_points, last_updated
                ) VALUES ($1, $2, $3, $4, $5, NOW())
                ON CONFLICT (service_name, operation_name, hour_of_day) DO UPDATE SET
                    baseline_p99_ms = EXCLUDED.baseline_p99_ms,
                    data_points = EXCLUDED.data_points,
                    last_updated = NOW()
                "#,
                service_name,
                operation_name,
                hour,
                baseline_p99,
                data_points
            )
            .execute(pool)
            .await?;
        }
    }

    tracing::info!("Health baselines updated successfully");
    Ok(())
}

fn get_baseline_for_service(
    baselines: &HashMap<(String, String, i32), f64>,
    service_name: &str,
    operation_name: &str,
    hour: i32,
) -> f64 {
    baselines
        .get(&(service_name.to_string(), operation_name.to_string(), hour))
        .copied()
        .unwrap_or(DEFAULT_BASELINE_P99)
}

pub async fn compute_health_scores(pool: &DbPool, target_service: Option<&str>) -> Result<Vec<HealthScoreSnapshot>> {
    tracing::info!("Computing health scores...");
    
    let now = Utc::now();
    let snapshot_time = now
        .with_minute(0)
        .unwrap_or(now)
        .with_second(0)
        .unwrap_or(now)
        .with_nanosecond(0)
        .unwrap_or(now);
    let current_hour = snapshot_time.hour() as i32;

    let one_hour_ago = now - Duration::hours(1);
    
    let baselines = sqlx::query!(
        r#"
        SELECT service_name, operation_name, hour_of_day, baseline_p99_ms
        FROM health_baselines
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut baseline_map: HashMap<(String, String, i32), f64> = HashMap::new();
    for b in baselines {
        baseline_map.insert(
            (b.service_name, b.operation_name, b.hour_of_day),
            b.baseline_p99_ms,
        );
    }

    let services = if let Some(svc) = target_service {
        vec![svc.to_string()]
    } else {
        let svcs = sqlx::query!(
            r#"
            SELECT DISTINCT service_name
            FROM spans
            WHERE start_time >= $1
            "#,
            one_hour_ago - Duration::hours(1)
        )
        .fetch_all(pool)
        .await?;
        svcs.into_iter().map(|s| s.service_name).collect()
    };

    let mut results = Vec::new();

    for service_name in &services {
        let availability_data = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COUNT(*) FILTER (WHERE status_code >= 500) as server_errors,
                COUNT(*) FILTER (WHERE status_code >= 400 AND status_code < 500) as client_errors
            FROM spans
            WHERE service_name = $1 AND start_time >= $2
            "#,
            service_name,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let total_requests = availability_data.total_requests.unwrap_or(0);
        let server_errors = availability_data.server_errors.unwrap_or(0);
        let _client_errors = availability_data.client_errors.unwrap_or(0);

        let availability_score = if total_requests > 0 {
            let error_rate = server_errors as f64 / total_requests as f64;
            100.0 * (1.0 - error_rate)
        } else {
            100.0
        };

        let p99_data = sqlx::query!(
            r#"
            SELECT operation_name, p99_ms
            FROM service_metrics
            WHERE service_name = $1 AND time_bucket >= $2
            "#,
            service_name,
            one_hour_ago
        )
        .fetch_all(pool)
        .await?;

        let mut latency_scores = Vec::new();
        for record in &p99_data {
            if let Some(p99) = record.p99_ms {
                let baseline = get_baseline_for_service(
                    &baseline_map,
                    service_name,
                    &record.operation_name,
                    current_hour,
                );
                let ratio = p99 / baseline;
                
                let score = if ratio <= 1.0 {
                    100.0
                } else if ratio <= 2.0 {
                    linear_interpolate(ratio, 1.0, 2.0, 50.0, 100.0)
                } else if ratio <= 3.0 {
                    linear_interpolate(ratio, 2.0, 3.0, 0.0, 50.0)
                } else {
                    0.0
                };
                latency_scores.push(score);
            }
        }

        let latency_score = if !latency_scores.is_empty() {
            latency_scores.iter().sum::<f64>() / latency_scores.len() as f64
        } else {
            100.0
        };

        let window_counts = sqlx::query!(
            r#"
            SELECT
                time_bucket,
                call_count
            FROM service_metrics
            WHERE service_name = $1 AND time_bucket >= $2
            ORDER BY time_bucket
            "#,
            service_name,
            now - Duration::hours(1)
        )
        .fetch_all(pool)
        .await?;

        let counts: Vec<f64> = window_counts
            .iter()
            .map(|w| w.call_count.unwrap_or(0) as f64)
            .collect();

        let throughput_stability_score = if counts.len() >= 2 {
            let mean = counts.iter().sum::<f64>() / counts.len() as f64;
            if mean > 0.0 {
                let variance: f64 = counts
                    .iter()
                    .map(|&x| (x - mean).powi(2))
                    .sum::<f64>()
                    / counts.len() as f64;
                let std_dev = variance.sqrt();
                let cv = std_dev / mean;
                
                linear_interpolate(cv, 0.1, 0.5, 0.0, 100.0)
            } else {
                100.0
            }
        } else {
            100.0
        };

        let error_types = sqlx::query!(
            r#"
            SELECT DISTINCT status_code, operation_name
            FROM spans
            WHERE service_name = $1
              AND status_code >= 400
              AND start_time >= $2
            "#,
            service_name,
            one_hour_ago
        )
        .fetch_all(pool)
        .await?;

        let error_count = error_types.len() as f64;
        let error_diversity_score = linear_interpolate(error_count, 2.0, 10.0, 0.0, 100.0);

        let total_score = availability_score * AVAILABILITY_WEIGHT
            + latency_score * LATENCY_WEIGHT
            + throughput_stability_score * THROUGHPUT_WEIGHT
            + error_diversity_score * ERROR_DIVERSITY_WEIGHT;

        let raw_metrics = json!({
            "total_requests": total_requests,
            "server_errors": server_errors,
            "availability_score": availability_score,
            "latency_score": latency_score,
            "throughput_stability_score": throughput_stability_score,
            "error_diversity_score": error_diversity_score,
            "unique_error_types": error_count as i32,
            "window_count": counts.len(),
        });

        let snapshot = sqlx::query_as!(
            HealthScoreSnapshot,
            r#"
            INSERT INTO health_score_snapshots (
                service_name, snapshot_time, total_score,
                availability_score, latency_score, throughput_stability_score, error_diversity_score,
                availability_weight, latency_weight, throughput_weight, error_diversity_weight,
                raw_metrics
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (service_name, snapshot_time) DO UPDATE SET
                total_score = EXCLUDED.total_score,
                availability_score = EXCLUDED.availability_score,
                latency_score = EXCLUDED.latency_score,
                throughput_stability_score = EXCLUDED.throughput_stability_score,
                error_diversity_score = EXCLUDED.error_diversity_score,
                raw_metrics = EXCLUDED.raw_metrics,
                created_at = NOW()
            RETURNING *
            "#,
            service_name,
            snapshot_time,
            total_score,
            availability_score,
            latency_score,
            throughput_stability_score,
            error_diversity_score,
            AVAILABILITY_WEIGHT,
            LATENCY_WEIGHT,
            THROUGHPUT_WEIGHT,
            ERROR_DIVERSITY_WEIGHT,
            raw_metrics
        )
        .fetch_one(pool)
        .await?;

        results.push(snapshot);
    }

    check_health_alerts(pool, &services).await?;

    tracing::info!("Health scores computed for {} services", results.len());
    Ok(results)
}

pub async fn compute_capacity_plans(pool: &DbPool, target_service: Option<&str>) -> Result<Vec<CapacityPlan>> {
    tracing::info!("Computing capacity plans...");
    
    let now = Utc::now();
    let snapshot_time = now
        .with_minute(0)
        .unwrap_or(now)
        .with_second(0)
        .unwrap_or(now)
        .with_nanosecond(0)
        .unwrap_or(now);
    let twenty_four_hours_ago = now - Duration::hours(24);
    let five_minutes_ago = now - Duration::minutes(5);

    let services = if let Some(svc) = target_service {
        vec![svc.to_string()]
    } else {
        let svcs = sqlx::query!(
            r#"
            SELECT DISTINCT service_name
            FROM spans
            WHERE start_time >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_all(pool)
        .await?;
        svcs.into_iter().map(|s| s.service_name).collect()
    };

    let mut results = Vec::new();

    for service_name in &services {
        let current_metrics = sqlx::query!(
            r#"
            SELECT
                SUM(call_count) as total_calls,
                AVG(total_duration_ms::FLOAT / NULLIF(call_count, 0)) as avg_duration
            FROM service_metrics
            WHERE service_name = $1 AND time_bucket >= $2
            "#,
            service_name,
            five_minutes_ago
        )
        .fetch_one(pool)
        .await?;

        let total_calls = current_metrics.total_calls.unwrap_or(0);
        let current_qps = total_calls as f64 / 300.0;
        let avg_response_time_ms = current_metrics.avg_duration.unwrap_or(0.0).max(1.0);

        let concurrent_peaks = sqlx::query!(
            r#"
            WITH span_windows AS (
                SELECT
                    generate_series(
                        date_trunc('minute', MIN(start_time)),
                        date_trunc('minute', MAX(end_time)),
                        INTERVAL '1 minute'
                    ) as window_start
                FROM spans
                WHERE service_name = $1 AND start_time >= $2
            )
            SELECT
                COUNT(*) as concurrent_count
            FROM span_windows w
            JOIN spans s ON s.service_name = $1
                AND s.start_time <= w.window_start + INTERVAL '1 minute'
                AND s.end_time >= w.window_start
            WHERE w.window_start >= $2
            GROUP BY w.window_start
            ORDER BY concurrent_count DESC
            "#,
            service_name,
            twenty_four_hours_ago
        )
        .fetch_all(pool)
        .await?;

        let mut concurrencies: Vec<f64> = concurrent_peaks
            .iter()
            .map(|c| c.concurrent_count.unwrap_or(0) as f64)
            .collect();
        
        concurrencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let concurrent_peak_p95 = if !concurrencies.is_empty() {
            percentile(&concurrencies, 95.0) as i32
        } else {
            1
        };

        let concurrent_limit = concurrent_peak_p95.max(1) as f64;
        let avg_response_time_sec = (avg_response_time_ms.max(1.0)) / 1000.0;
        let theoretical_max_qps = concurrent_limit / avg_response_time_sec;
        let max_qps = theoretical_max_qps.min(MAX_QPS_CAP);
        
        let remaining_capacity = (max_qps - current_qps).max(0.0);
        let remaining_pct = if current_qps > 0.0 {
            (remaining_capacity / current_qps) * 100.0
        } else {
            100.0
        };
        let is_warning = remaining_pct < CAPACITY_WARNING_THRESHOLD_PCT;

        let plan = sqlx::query_as!(
            CapacityPlan,
            r#"
            INSERT INTO capacity_plans (
                service_name, snapshot_time, current_qps, max_qps, remaining_capacity,
                avg_response_time_ms, concurrent_peak_p95, is_warning, warning_threshold_pct
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (service_name, snapshot_time) DO UPDATE SET
                current_qps = EXCLUDED.current_qps,
                max_qps = EXCLUDED.max_qps,
                remaining_capacity = EXCLUDED.remaining_capacity,
                avg_response_time_ms = EXCLUDED.avg_response_time_ms,
                concurrent_peak_p95 = EXCLUDED.concurrent_peak_p95,
                is_warning = EXCLUDED.is_warning,
                created_at = NOW()
            RETURNING *
            "#,
            service_name,
            snapshot_time,
            current_qps,
            max_qps,
            remaining_capacity,
            avg_response_time_ms,
            concurrent_peak_p95,
            is_warning,
            CAPACITY_WARNING_THRESHOLD_PCT
        )
        .fetch_one(pool)
        .await?;

        results.push(plan);
    }

    tracing::info!("Capacity plans computed for {} services", results.len());
    Ok(results)
}

async fn check_health_alerts(pool: &DbPool, services: &[String]) -> Result<()> {
    let three_hours_ago = Utc::now() - Duration::hours(3);

    for service_name in services {
        let recent_scores = sqlx::query!(
            r#"
            SELECT total_score, snapshot_time
            FROM health_score_snapshots
            WHERE service_name = $1 AND snapshot_time >= $2
            ORDER BY snapshot_time DESC
            LIMIT 3
            "#,
            service_name,
            three_hours_ago
        )
        .fetch_all(pool)
        .await?;

        if recent_scores.len() >= 3 {
            let all_below_threshold = recent_scores
                .iter()
                .all(|s| s.total_score < WARNING_THRESHOLD);

            if all_below_threshold {
                let existing_event = sqlx::query!(
                    r#"
                    SELECT id FROM health_events
                    WHERE service_name = $1
                      AND event_type = 'low_health_score'
                      AND created_at >= $2
                    ORDER BY created_at DESC
                    LIMIT 1
                    "#,
                    service_name,
                    three_hours_ago
                )
                .fetch_optional(pool)
                .await?;

                if existing_event.is_none() {
                    let min_score = recent_scores
                        .iter()
                        .map(|s| s.total_score)
                        .fold(f64::INFINITY, f64::min);

                    sqlx::query!(
                        r#"
                        INSERT INTO health_events (
                            id, event_type, service_name, severity, message,
                            score, threshold, consecutive_hours, details
                        ) VALUES ($1, 'low_health_score', $2, 'warning', $3, $4, $5, 3, '{}'::jsonb)
                        "#,
                        Uuid::new_v4(),
                        service_name,
                        format!(
                            "Service health score below {} for 3 consecutive hours. Minimum score: {:.1}",
                            WARNING_THRESHOLD, min_score
                        ),
                        min_score,
                        WARNING_THRESHOLD
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

pub async fn get_health_rankings(pool: &DbPool) -> Result<Vec<HealthRankItem>> {
    let latest_snapshot = sqlx::query!(
        r#"
        SELECT MAX(snapshot_time) as latest
        FROM health_score_snapshots
        "#
    )
    .fetch_one(pool)
    .await?;

    let latest_time = match latest_snapshot.latest {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let snapshots = sqlx::query_as!(
        HealthScoreSnapshot,
        r#"
        SELECT * FROM health_score_snapshots
        WHERE snapshot_time = $1
        ORDER BY total_score DESC
        "#,
        latest_time
    )
    .fetch_all(pool)
    .await?;

    let rankings = snapshots
        .into_iter()
        .map(|s| {
            let status = if s.total_score >= 80.0 {
                "healthy".to_string()
            } else if s.total_score >= 60.0 {
                "warning".to_string()
            } else {
                "danger".to_string()
            };

            HealthRankItem {
                service_name: s.service_name,
                total_score: s.total_score,
                availability_score: s.availability_score,
                latency_score: s.latency_score,
                throughput_stability_score: s.throughput_stability_score,
                error_diversity_score: s.error_diversity_score,
                snapshot_time: s.snapshot_time,
                status,
            }
        })
        .collect();

    Ok(rankings)
}

pub async fn get_service_health_trend(
    pool: &DbPool,
    service_name: &str,
    days: i64,
) -> Result<Vec<HealthTrendPoint>> {
    let start_time = Utc::now() - Duration::days(days);

    let snapshots = sqlx::query!(
        r#"
        SELECT
            snapshot_time,
            total_score,
            availability_score,
            latency_score,
            throughput_stability_score,
            error_diversity_score
        FROM health_score_snapshots
        WHERE service_name = $1 AND snapshot_time >= $2
        ORDER BY snapshot_time ASC
        "#,
        service_name,
        start_time
    )
    .fetch_all(pool)
    .await?;

    let trends = snapshots
        .into_iter()
        .map(|s| HealthTrendPoint {
            snapshot_time: s.snapshot_time,
            total_score: s.total_score,
            availability_score: s.availability_score,
            latency_score: s.latency_score,
            throughput_stability_score: s.throughput_stability_score,
            error_diversity_score: s.error_diversity_score,
        })
        .collect();

    Ok(trends)
}

pub async fn get_latest_capacity_plans(pool: &DbPool) -> Result<Vec<CapacityPlan>> {
    let latest_snapshot = sqlx::query!(
        r#"
        SELECT MAX(snapshot_time) as latest
        FROM capacity_plans
        "#
    )
    .fetch_one(pool)
    .await?;

    let latest_time = match latest_snapshot.latest {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let plans = sqlx::query_as!(
        CapacityPlan,
        r#"
        SELECT * FROM capacity_plans
        WHERE snapshot_time = $1
        ORDER BY remaining_capacity ASC
        "#,
        latest_time
    )
    .fetch_all(pool)
    .await?;

    Ok(plans)
}

pub async fn get_health_events(
    pool: &DbPool,
    service_name: Option<&str>,
    limit: i64,
) -> Result<Vec<HealthEvent>> {
    let events = if let Some(svc) = service_name {
        sqlx::query_as!(
            HealthEvent,
            r#"
            SELECT * FROM health_events
            WHERE service_name = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            svc,
            limit
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            HealthEvent,
            r#"
            SELECT * FROM health_events
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await?
    };

    Ok(events)
}

pub async fn run_full_health_computation(pool: &DbPool) -> Result<()> {
    update_health_baselines(pool).await?;
    compute_health_scores(pool, None).await?;
    compute_capacity_plans(pool, None).await?;
    Ok(())
}
