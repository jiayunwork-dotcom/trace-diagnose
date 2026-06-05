use crate::AppState;
use crate::models::*;
use crate::importer;
use crate::analysis;
use crate::health;
use axum::{
    extract::{Path, Query, State, Multipart},
    http::StatusCode,
    response::IntoResponse,
    Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;
use std::time::Duration;

pub fn traces_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_traces))
        .route("/{id}", get(get_trace))
        .route("/{id}/spans", get(get_trace_spans))
        .route("/{id}/critical-path", get(get_critical_path))
        .route("/compare", get(compare_traces_handler))
}

pub fn services_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_services))
        .route("/{name}", get(get_service_details))
        .route("/{name}/metrics", get(get_service_metrics))
        .route("/{name}/latency-distribution", get(get_latency_distribution_handler))
}

pub fn topology_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_topology_handler))
}

pub fn analysis_routes() -> Router<AppState> {
    Router::new()
        .route("/latency-distribution", get(get_latency_distribution_handler))
        .route("/anomalies", get(list_anomalies))
        .route("/critical-path/{trace_id}", get(get_critical_path))
        .route("/batch-compare", get(batch_compare_handler))
}

pub fn slo_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_slos).post(create_slo))
        .route("/{id}", get(get_slo).put(update_slo).delete(delete_slo))
        .route("/{id}/status", get(get_slo_status))
}

pub fn alerts_routes() -> Router<AppState> {
    Router::new()
        .route("/rules", get(list_alert_rules).post(create_alert_rule))
        .route("/rules/{id}", get(get_alert_rule).put(update_alert_rule).delete(delete_alert_rule))
        .route("/rules/{id}/events", get(get_rule_events))
        .route("/events", get(list_alert_events))
        .route("/events/{id}/acknowledge", post(acknowledge_alert_event))
        .route("/evaluate", post(evaluate_alerts))
}

pub fn import_routes() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/push", post(push_spans))
        .route("/jobs", get(list_import_jobs))
        .route("/jobs/{id}", get(get_import_job))
        .route("/jobs/{id}/progress", get(get_import_progress))
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Debug, Deserialize)]
pub struct ListTracesParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub service: Option<String>,
    pub min_duration: Option<i64>,
    pub has_errors: Option<bool>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

async fn list_traces(
    State(state): State<AppState>,
    Query(params): Query<ListTracesParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = ((page - 1) * page_size) as i64;

    let mut query = "SELECT * FROM traces WHERE 1=1".to_string();
    let mut count_query = "SELECT COUNT(*) FROM traces WHERE 1=1".to_string();
    let mut conditions: Vec<String> = Vec::new();

    if let Some(service) = &params.service {
        conditions.push(format!("trace_id IN (SELECT DISTINCT trace_id FROM spans WHERE service_name = '{}')", service.replace("'", "''")));
    }
    if let Some(min_dur) = params.min_duration {
        conditions.push(format!("duration_ms >= {}", min_dur));
    }
    if let Some(has_errors) = params.has_errors {
        conditions.push(format!("has_errors = {}", has_errors));
    }

    if !conditions.is_empty() {
        let where_clause = conditions.join(" AND ");
        query.push_str(" AND ");
        query.push_str(&where_clause);
        count_query.push_str(" AND ");
        count_query.push_str(&where_clause);
    }

    query.push_str(" ORDER BY start_time DESC LIMIT $1 OFFSET $2");

    let traces: Vec<Trace> = sqlx::query_as(&query)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total: Option<i64> = sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await
        .ok();
    let total = total.unwrap_or(0);

    let total_pages = (total + page_size as i64 - 1) / page_size as i64;

    Json(PaginatedResponse {
        data: traces,
        total,
        page,
        page_size,
        total_pages,
    })
}

async fn get_trace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TraceWithSpans>, StatusCode> {
    let cache_key = format!("trace:{}", id);
    if let Ok(Some(cached)) = state.cache.get::<TraceWithSpans>(&cache_key).await {
        return Ok(Json(cached));
    }

    let trace = sqlx::query_as!(Trace, "SELECT * FROM traces WHERE trace_id = $1", id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let spans = sqlx::query_as!(Span, "SELECT * FROM spans WHERE trace_id = $1 ORDER BY start_time", id)
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = TraceWithSpans { trace, spans };

    let _ = state.cache.set(&cache_key, &result, Duration::from_secs(300)).await;

    Ok(Json(result))
}

async fn get_trace_spans(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Vec<Span>> {
    let spans = sqlx::query_as!(Span, "SELECT * FROM spans WHERE trace_id = $1 ORDER BY start_time", id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    Json(spans)
}

#[derive(Debug, Deserialize)]
struct CompareParams {
    baseline: String,
    comparison: String,
}

async fn compare_traces_handler(
    State(state): State<AppState>,
    Query(params): Query<CompareParams>,
) -> Result<Json<TraceComparison>, StatusCode> {
    analysis::compare_traces(&state.db, &params.baseline, &params.comparison)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn list_services(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let services = sqlx::query!(
        r#"
        SELECT
            service_name,
            COUNT(DISTINCT trace_id) as trace_count,
            COUNT(*) as span_count,
            AVG(duration_ms) as avg_duration,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count
        FROM spans
        GROUP BY service_name
        ORDER BY span_count DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let result: Vec<serde_json::Value> = services
        .into_iter()
        .map(|s| {
            let total = s.span_count.unwrap_or(0);
            let error_rate = if total > 0 {
                s.error_count.unwrap_or(0) as f64 / total as f64
            } else {
                0.0
            };
            serde_json::json!({
                "service_name": s.service_name,
                "trace_count": s.trace_count,
                "span_count": s.span_count,
                "avg_duration_ms": s.avg_duration.unwrap_or(0.0),
                "error_rate": error_rate,
            })
        })
        .collect();

    Json(result)
}

async fn get_service_details(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let details = sqlx::query!(
        r#"
        SELECT
            COUNT(DISTINCT trace_id) as trace_count,
            COUNT(*) as span_count,
            AVG(duration_ms) as avg_duration,
            MIN(duration_ms) as min_duration,
            MAX(duration_ms) as max_duration,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count
        FROM spans
        WHERE service_name = $1
        "#,
        name
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let operations = sqlx::query!(
        r#"
        SELECT
            operation_name,
            COUNT(*) as call_count,
            AVG(duration_ms) as avg_duration,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count
        FROM spans
        WHERE service_name = $1
        GROUP BY operation_name
        ORDER BY call_count DESC
        LIMIT 20
        "#,
        name
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total = details.span_count.unwrap_or(0);
    let error_rate = if total > 0 {
        details.error_count.unwrap_or(0) as f64 / total as f64
    } else {
        0.0
    };

    Ok(Json(serde_json::json!({
        "service_name": name,
        "trace_count": details.trace_count,
        "span_count": details.span_count,
        "avg_duration_ms": details.avg_duration.unwrap_or(0.0),
        "min_duration_ms": details.min_duration,
        "max_duration_ms": details.max_duration,
        "error_rate": error_rate,
        "operations": operations.into_iter().map(|op| {
            let total_ops = op.call_count.unwrap_or(0);
            let op_error_rate = if total_ops > 0 {
                op.error_count.unwrap_or(0) as f64 / total_ops as f64
            } else { 0.0 };
            serde_json::json!({
                "operation_name": op.operation_name,
                "call_count": op.call_count,
                "avg_duration_ms": op.avg_duration.unwrap_or(0.0),
                "error_rate": op_error_rate,
            })
        }).collect::<Vec<_>>(),
    })))
}

#[derive(Debug, Deserialize)]
struct ServiceMetricsParams {
    pub operation: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

async fn get_service_metrics(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<ServiceMetricsParams>,
) -> Json<Vec<ServiceMetric>> {
    let metrics = if let Some(op) = &params.operation {
        sqlx::query_as::<_, ServiceMetric>(
            "SELECT * FROM service_metrics WHERE service_name = $1 AND operation_name = $2 ORDER BY time_bucket DESC LIMIT 100"
        )
        .bind(&name)
        .bind(op)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as::<_, ServiceMetric>(
            "SELECT * FROM service_metrics WHERE service_name = $1 ORDER BY time_bucket DESC LIMIT 100"
        )
        .bind(&name)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    };
    Json(metrics)
}

#[derive(Debug, Deserialize)]
struct LatencyDistributionParams {
    pub service: Option<String>,
    pub operation: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

async fn get_latency_distribution_handler(
    State(state): State<AppState>,
    Query(params): Query<LatencyDistributionParams>,
) -> Json<LatencyDistribution> {
    let dist = analysis::get_latency_distribution(
        &state.db,
        params.service.as_deref(),
        params.operation.as_deref(),
        None,
        None,
    )
    .await
    .unwrap_or(LatencyDistribution {
        buckets: vec![],
        p50: 0.0,
        p95: 0.0,
        p99: 0.0,
    });
    Json(dist)
}

async fn get_topology_handler(
    State(state): State<AppState>,
) -> Json<TopologyResponse> {
    let cache_key = "topology:full";
    if let Ok(Some(cached)) = state.cache.get::<TopologyResponse>(cache_key).await {
        return Json(cached);
    }

    let topology = analysis::get_topology(&state.db)
        .await
        .unwrap_or(TopologyResponse {
            nodes: vec![],
            edges: vec![],
        });

    let _ = state.cache.set(cache_key, &topology, Duration::from_secs(60)).await;

    Json(topology)
}

async fn list_anomalies(
    State(state): State<AppState>,
    Query(params): Query<ListTracesParams>,
) -> Json<Vec<AnomalyTrace>> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = ((page - 1) * page_size) as i64;

    let anomalies = sqlx::query_as!(
        AnomalyTrace,
        "SELECT * FROM anomaly_traces ORDER BY detected_at DESC LIMIT $1 OFFSET $2",
        page_size as i64,
        offset
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(anomalies)
}

async fn get_critical_path(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<CriticalPath>, StatusCode> {
    analysis::find_critical_path(&state.db, &trace_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn list_slos(
    State(state): State<AppState>,
) -> Json<Vec<SloConfig>> {
    let slos = sqlx::query_as!(SloConfig, "SELECT * FROM slo_configs WHERE is_active = TRUE ORDER BY service_name")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    Json(slos)
}

async fn create_slo(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let service_name = body["service_name"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let slo_type = body["slo_type"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let threshold = body["threshold"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
    let target = body["target"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
    let window_days = body["window_days"].as_i64().unwrap_or(30) as i32;
    let description = body["description"].as_str().map(|s| s.to_string());

    sqlx::query!(
        r#"
        INSERT INTO slo_configs (service_name, slo_type, threshold, target, window_days, description)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        service_name,
        slo_type,
        threshold,
        target,
        window_days,
        description
    )
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

async fn get_slo(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SloConfig>, StatusCode> {
    let slo = sqlx::query_as!(SloConfig, "SELECT * FROM slo_configs WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(slo))
}

async fn update_slo(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let threshold = body["threshold"].as_f64();
    let target = body["target"].as_f64();
    let is_active = body["is_active"].as_bool();

    if threshold.is_some() || target.is_some() || is_active.is_some() {
        sqlx::query!(
            r#"
            UPDATE slo_configs SET
                threshold = COALESCE($1, threshold),
                target = COALESCE($2, target),
                is_active = COALESCE($3, is_active),
                updated_at = NOW()
            WHERE id = $4
            "#,
            threshold,
            target,
            is_active,
            id
        )
        .execute(&state.db)
        .await
        .ok();
    }

    StatusCode::NO_CONTENT
}

async fn delete_slo(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    sqlx::query!("DELETE FROM slo_configs WHERE id = $1", id)
        .execute(&state.db)
        .await
        .ok();
    StatusCode::NO_CONTENT
}

async fn get_slo_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Json<Vec<SloStatus>> {
    let _ = analysis::calculate_slo_status(&state.db, id).await;
    let status = sqlx::query_as!(
        SloStatus,
        "SELECT * FROM slo_status WHERE slo_id = $1 ORDER BY date DESC LIMIT 30",
        id
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Json(status)
}

async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let job_id = Uuid::new_v4();
    let mut format = "otel".to_string();
    let mut file_name = None;
    let mut file_data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        if name == "format" {
            format = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        } else if name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();
        }
    }

    let trace_format = importer::TraceFormat::from_str(&format).map_err(|_| StatusCode::BAD_REQUEST)?;

    sqlx::query!(
        r#"
        INSERT INTO import_jobs (id, file_name, format, status, started_at)
        VALUES ($1, $2, $3, 'processing', NOW())
        "#,
        job_id,
        file_name,
        format
    )
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = state.db.clone();
    let progress_map = state.import_progress.clone();

    tokio::spawn(async move {
        let progress_map_clone = progress_map.clone();
        let progress_callback = move |total: usize, processed: usize| -> anyhow::Result<()> {
            let progress = importer::ImportProgress {
                job_id,
                total,
                processed,
                status: "processing".to_string(),
            };
            let map = progress_map_clone.clone();
            tokio::spawn(async move {
                let mut w = map.write().await;
                w.insert(job_id.to_string(), progress);
            });
            Ok(())
        };

        let result = importer::parse_and_import(&db, trace_format, &file_data, job_id, progress_callback).await;

        match result {
            Ok(count) => {
                sqlx::query!(
                    r#"
                    UPDATE import_jobs
                    SET status = 'completed', total_spans = $1, processed_spans = $2, completed_at = NOW()
                    WHERE id = $3
                    "#,
                    count as i32,
                    count as i32,
                    job_id
                )
                .execute(&db)
                .await
                .ok();
            }
            Err(e) => {
                sqlx::query!(
                    r#"
                    UPDATE import_jobs
                    SET status = 'failed', error_message = $1, completed_at = NOW()
                    WHERE id = $2
                    "#,
                    e.to_string(),
                    job_id
                )
                .execute(&db)
                .await
                .ok();
            }
        }

        progress_map.write().await.remove(&job_id.to_string());
    });

    Ok(Json(serde_json::json!({
        "job_id": job_id,
        "status": "processing"
    })))
}

async fn push_spans(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let format = params.get("format").cloned().unwrap_or_else(|| "otel".to_string());
    let trace_format = importer::TraceFormat::from_str(&format).map_err(|_| StatusCode::BAD_REQUEST)?;

    let job_id = Uuid::new_v4();
    let data = body.as_bytes();

    let result = importer::parse_and_import(
        &state.db,
        trace_format,
        data,
        job_id,
        |_, _| Ok(()),
    )
    .await
    .map_err(|e| {
        tracing::error!("Import failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "imported_spans": result,
        "status": "success"
    })))
}

async fn list_import_jobs(
    State(state): State<AppState>,
) -> Json<Vec<ImportJob>> {
    let jobs = sqlx::query_as!(ImportJob, "SELECT * FROM import_jobs ORDER BY created_at DESC LIMIT 50")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    Json(jobs)
}

async fn get_import_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJob>, StatusCode> {
    let job = sqlx::query_as!(ImportJob, "SELECT * FROM import_jobs WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(job))
}

async fn get_import_progress(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Json<serde_json::Value> {
    let map = state.import_progress.read().await;
    if let Some(progress) = map.get(&id.to_string()) {
        return Json(serde_json::json!(progress));
    }

    let job = sqlx::query_as!(ImportJob, "SELECT * FROM import_jobs WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(job) = job {
        Json(serde_json::json!({
            "job_id": job.id,
            "total": job.total_spans,
            "processed": job.processed_spans,
            "status": job.status
        }))
    } else {
        Json(serde_json::json!({
            "status": "not_found"
        }))
    }
}

async fn list_alert_rules(
    State(state): State<AppState>,
) -> Json<Vec<AlertRule>> {
    let rules = sqlx::query_as!(
        AlertRule,
        "SELECT * FROM alert_rules ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Json(rules)
}

async fn create_alert_rule(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let name = body["name"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let description = body["description"].as_str().map(|s| s.to_string());
    let service_name = body["service_name"].as_str().map(|s| s.to_string());
    let operation_name = body["operation_name"].as_str().map(|s| s.to_string());
    let metric_type = body["metric_type"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let threshold = body["threshold"].as_f64().ok_or(StatusCode::BAD_REQUEST)?;
    let comparison_operator = body["comparison_operator"].as_str().unwrap_or(">").to_string();
    let window_minutes = body["window_minutes"].as_i64().unwrap_or(5) as i32;
    let consecutive_windows = body["consecutive_windows"].as_i64().unwrap_or(1) as i32;
    let severity = body["severity"].as_str().unwrap_or("warning").to_string();
    let silence_minutes = body["silence_minutes"].as_i64().unwrap_or(30) as i32;

    sqlx::query!(
        r#"
        INSERT INTO alert_rules (
            name, description, service_name, operation_name,
            metric_type, threshold, comparison_operator,
            window_minutes, consecutive_windows, severity, silence_minutes
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        name,
        description,
        service_name,
        operation_name,
        metric_type,
        threshold,
        comparison_operator,
        window_minutes,
        consecutive_windows,
        severity,
        silence_minutes,
    )
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

async fn get_alert_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertRule>, StatusCode> {
    let rule = sqlx::query_as!(AlertRule, "SELECT * FROM alert_rules WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(rule))
}

async fn update_alert_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let name = body["name"].as_str().map(|s| s.to_string());
    let description = body["description"].as_str().map(|s| s.to_string());
    let threshold = body["threshold"].as_f64();
    let comparison_operator = body["comparison_operator"].as_str().map(|s| s.to_string());
    let window_minutes = body["window_minutes"].as_i64().map(|v| v as i32);
    let consecutive_windows = body["consecutive_windows"].as_i64().map(|v| v as i32);
    let severity = body["severity"].as_str().map(|s| s.to_string());
    let silence_minutes = body["silence_minutes"].as_i64().map(|v| v as i32);
    let is_active = body["is_active"].as_bool();

    sqlx::query!(
        r#"
        UPDATE alert_rules SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            threshold = COALESCE($3, threshold),
            comparison_operator = COALESCE($4, comparison_operator),
            window_minutes = COALESCE($5, window_minutes),
            consecutive_windows = COALESCE($6, consecutive_windows),
            severity = COALESCE($7, severity),
            silence_minutes = COALESCE($8, silence_minutes),
            is_active = COALESCE($9, is_active),
            updated_at = NOW()
        WHERE id = $10
        "#,
        name,
        description,
        threshold,
        comparison_operator,
        window_minutes,
        consecutive_windows,
        severity,
        silence_minutes,
        is_active,
        id
    )
    .execute(&state.db)
    .await
    .ok();

    StatusCode::NO_CONTENT
}

async fn delete_alert_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    sqlx::query!("DELETE FROM alert_rules WHERE id = $1", id)
        .execute(&state.db)
        .await
        .ok();
    StatusCode::NO_CONTENT
}

async fn get_rule_events(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<ListTracesParams>,
) -> Json<Vec<AlertEvent>> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = ((page - 1) * page_size) as i64;

    let events = sqlx::query_as!(
        AlertEvent,
        "SELECT * FROM alert_events WHERE rule_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        id,
        page_size as i64,
        offset
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(events)
}

#[derive(Debug, Deserialize)]
struct ListAlertEventsParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<String>,
    pub service: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

async fn list_alert_events(
    State(state): State<AppState>,
    Query(params): Query<ListAlertEventsParams>,
) -> Json<PaginatedResponse<AlertEvent>> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = ((page - 1) * page_size) as i64;

    let mut query = "SELECT * FROM alert_events WHERE 1=1".to_string();
    let mut count_query = "SELECT COUNT(*) FROM alert_events WHERE 1=1".to_string();
    let mut conditions: Vec<String> = Vec::new();

    if let Some(status) = &params.status {
        conditions.push(format!("status = '{}'", status.replace("'", "''")));
    }
    if let Some(service) = &params.service {
        conditions.push(format!("service_name = '{}'", service.replace("'", "''")));
    }

    if !conditions.is_empty() {
        let where_clause = conditions.join(" AND ");
        query.push_str(" AND ");
        query.push_str(&where_clause);
        count_query.push_str(" AND ");
        count_query.push_str(&where_clause);
    }

    let sort_by = params.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = params.sort_order.as_deref().unwrap_or("DESC");
    query.push_str(&format!(" ORDER BY {} {} LIMIT $1 OFFSET $2", sort_by, sort_order));

    let events: Vec<AlertEvent> = sqlx::query_as(&query)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total: Option<i64> = sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await
        .ok();
    let total = total.unwrap_or(0);

    let total_pages = (total + page_size as i64 - 1) / page_size as i64;

    Json(PaginatedResponse {
        data: events,
        total,
        page,
        page_size,
        total_pages,
    })
}

async fn acknowledge_alert_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let acknowledged_by = body["acknowledged_by"].as_str().unwrap_or("system").to_string();

    sqlx::query!(
        r#"
        UPDATE alert_events
        SET status = 'acknowledged', acknowledged_by = $1, acknowledged_at = NOW()
        WHERE id = $2
        "#,
        acknowledged_by,
        id
    )
    .execute(&state.db)
    .await
    .ok();

    StatusCode::NO_CONTENT
}

async fn evaluate_alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertEvent>>, StatusCode> {
    analysis::evaluate_alert_rules(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, Deserialize)]
struct BatchCompareParams {
    pub baseline_start: String,
    pub baseline_end: String,
    pub comparison_start: String,
    pub comparison_end: String,
}

pub async fn batch_compare_handler(
    State(state): State<AppState>,
    Query(params): Query<BatchCompareParams>,
) -> Result<Json<Vec<BatchComparisonResult>>, StatusCode> {
    let parse_dt = |s: &str| -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    };

    let baseline_start = parse_dt(&params.baseline_start);
    let baseline_end = parse_dt(&params.baseline_end);
    let comparison_start = parse_dt(&params.comparison_start);
    let comparison_end = parse_dt(&params.comparison_end);

    analysis::batch_compare_traces(&state.db, baseline_start, baseline_end, comparison_start, comparison_end)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/rankings", get(get_health_rankings))
        .route("/trends/{service_name}", get(get_service_health_trend))
        .route("/capacity", get(get_capacity_plans))
        .route("/events", get(get_health_events_handler))
        .route("/compute", post(compute_health_now))
}

async fn get_health_rankings(
    State(state): State<AppState>,
) -> Json<Vec<HealthRankItem>> {
    let rankings = health::get_health_rankings(&state.db)
        .await
        .unwrap_or_default();
    Json(rankings)
}

#[derive(Debug, Deserialize)]
struct HealthTrendParams {
    pub days: Option<i64>,
}

async fn get_service_health_trend(
    State(state): State<AppState>,
    Path(service_name): Path<String>,
    Query(params): Query<HealthTrendParams>,
) -> Json<Vec<HealthTrendPoint>> {
    let days = params.days.unwrap_or(7);
    let trends = health::get_service_health_trend(&state.db, &service_name, days)
        .await
        .unwrap_or_default();
    Json(trends)
}

async fn get_capacity_plans(
    State(state): State<AppState>,
) -> Json<Vec<CapacityPlan>> {
    let plans = health::get_latest_capacity_plans(&state.db)
        .await
        .unwrap_or_default();
    Json(plans)
}

#[derive(Debug, Deserialize)]
struct HealthEventsParams {
    pub service: Option<String>,
    pub limit: Option<i64>,
}

async fn get_health_events_handler(
    State(state): State<AppState>,
    Query(params): Query<HealthEventsParams>,
) -> Json<Vec<HealthEvent>> {
    let limit = params.limit.unwrap_or(50);
    let events = health::get_health_events(&state.db, params.service.as_deref(), limit)
        .await
        .unwrap_or_default();
    Json(events)
}

async fn compute_health_now(
    State(state): State<AppState>,
    Json(body): Json<ComputeHealthRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.clone();
    let service_name = body.service_name.clone();
    
    tokio::spawn(async move {
        let _ = health::run_full_health_computation(&db).await;
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "message": "Health computation started in background",
        "service_name": service_name
    })))
}
