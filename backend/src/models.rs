use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trace {
    pub id: Uuid,
    pub trace_id: String,
    pub root_span_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub service_count: i32,
    pub span_count: i32,
    pub has_errors: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Span {
    pub id: Uuid,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: i64,
    pub status_code: i32,
    pub status_message: Option<String>,
    pub is_anomaly: bool,
    pub anomaly_reason: Option<String>,
    pub tags: serde_json::Value,
    pub logs: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInput {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status_code: i32,
    pub status_message: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub logs: Option<serde_json::Value>,
}

impl SpanInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.trace_id.trim().is_empty() {
            return Err("trace_id is required".to_string());
        }
        if self.span_id.trim().is_empty() {
            return Err("span_id is required".to_string());
        }
        if self.service_name.trim().is_empty() {
            return Err("service_name is required".to_string());
        }
        if self.operation_name.trim().is_empty() {
            return Err("operation_name is required".to_string());
        }
        if self.end_time < self.start_time {
            return Err("end_time must be after start_time".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceDependency {
    pub id: Uuid,
    pub caller_service: String,
    pub callee_service: String,
    pub call_count: i64,
    pub total_duration_ms: i64,
    pub avg_duration_ms: f64,
    pub error_count: i64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceMetric {
    pub id: Uuid,
    pub service_name: String,
    pub operation_name: String,
    pub time_bucket: DateTime<Utc>,
    pub call_count: i64,
    pub error_count: i64,
    pub total_duration_ms: i64,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub duration_histogram: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SloConfig {
    pub id: Uuid,
    pub service_name: String,
    pub slo_type: String,
    pub threshold: f64,
    pub target: f64,
    pub window_days: i32,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SloStatus {
    pub id: Uuid,
    pub slo_id: Uuid,
    pub date: chrono::NaiveDate,
    pub current_value: f64,
    pub error_budget_remaining: f64,
    pub is_breached: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnomalyTrace {
    pub id: Uuid,
    pub trace_id: String,
    pub anomaly_type: String,
    pub severity: String,
    pub root_service: Option<String>,
    pub root_operation: Option<String>,
    pub affected_services: Option<Vec<String>>,
    pub details: serde_json::Value,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImportJob {
    pub id: Uuid,
    pub file_name: Option<String>,
    pub format: String,
    pub total_spans: i32,
    pub processed_spans: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub service_name: String,
    pub qps: f64,
    pub error_rate: f64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub source: String,
    pub target: String,
    pub call_count: i64,
    pub avg_duration_ms: f64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceWithSpans {
    pub trace: Trace,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceComparison {
    pub baseline_trace_id: String,
    pub comparison_trace_id: String,
    pub differences: Vec<TraceDifference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDifference {
    pub difference_type: String,
    pub service_name: String,
    pub operation_name: String,
    pub baseline_duration_ms: Option<i64>,
    pub comparison_duration_ms: Option<i64>,
    pub duration_change_pct: Option<f64>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPath {
    pub trace_id: String,
    pub total_duration_ms: i64,
    pub spans: Vec<CriticalPathSpan>,
    pub parallel_optimization_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPathSpan {
    pub span_id: String,
    pub service_name: String,
    pub operation_name: String,
    pub duration_ms: i64,
    pub contribution_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    pub buckets: Vec<LatencyBucket>,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBucket {
    pub min_ms: i64,
    pub max_ms: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub page_size: u32,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            sort_by: None,
            sort_order: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub service_name: Option<String>,
    pub operation_name: Option<String>,
    pub metric_type: String,
    pub threshold: f64,
    pub comparison_operator: String,
    pub window_minutes: i32,
    pub consecutive_windows: i32,
    pub severity: String,
    pub silence_minutes: i32,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertEvent {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: String,
    pub service_name: Option<String>,
    pub operation_name: Option<String>,
    pub metric_type: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub status: String,
    pub trace_ids: Option<Vec<String>>,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub firing_started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchComparisonRequest {
    pub baseline_start: String,
    pub baseline_end: String,
    pub comparison_start: String,
    pub comparison_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchComparisonResult {
    pub service_name: String,
    pub operation_name: String,
    pub baseline_avg_duration: f64,
    pub comparison_avg_duration: f64,
    pub duration_change_pct: f64,
    pub baseline_p95: f64,
    pub comparison_p95: f64,
    pub baseline_call_count: i64,
    pub comparison_call_count: i64,
    pub is_regression: bool,
}
