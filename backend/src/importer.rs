use crate::models::*;
use crate::db::DbPool;
use crate::analysis;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use chrono::{DateTime, Utc, TimeZone};
use std::collections::HashMap;
use uuid::Uuid;
use tracing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub job_id: Uuid,
    pub total: usize,
    pub processed: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceFormat {
    OpenTelemetryJson,
    OpenTelemetryProto,
    Jaeger,
    ZipkinV2,
}

impl TraceFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "otel" | "opentelemetry" | "opentelemetry-json" => Ok(Self::OpenTelemetryJson),
            "otel-proto" | "opentelemetry-proto" => Ok(Self::OpenTelemetryProto),
            "jaeger" => Ok(Self::Jaeger),
            "zipkin" | "zipkin-v2" => Ok(Self::ZipkinV2),
            _ => anyhow::bail!("Unknown format: {}", s),
        }
    }
}

pub async fn parse_and_import(
    pool: &DbPool,
    format: TraceFormat,
    data: &[u8],
    job_id: Uuid,
    progress_callback: impl Fn(usize, usize) -> Result<()> + Send + Sync,
) -> Result<usize> {
    let spans = match format {
        TraceFormat::OpenTelemetryJson => parse_otel_json(data)?,
        TraceFormat::Jaeger => parse_jaeger_json(data)?,
        TraceFormat::ZipkinV2 => parse_zipkin_v2_json(data)?,
        TraceFormat::OpenTelemetryProto => parse_otel_proto(data)?,
    };

    let total = spans.len();
    progress_callback(total, 0)?;

    let batch_size = 100;
    let mut processed = 0;

    for chunk in spans.chunks(batch_size) {
        import_spans_batch(pool, chunk).await?;
        processed += chunk.len();
        progress_callback(total, processed)?;
    }

    let trace_ids: std::collections::HashSet<_> = spans.iter().map(|s| s.trace_id.as_str()).collect();
    for trace_id in trace_ids {
        analysis::rebuild_trace_topology(pool, trace_id).await?;
        analysis::update_service_metrics_for_trace(pool, trace_id).await?;
        analysis::detect_anomalies_in_trace(pool, trace_id).await?;
    }

    analysis::update_service_dependencies(pool).await?;

    Ok(total)
}

fn parse_otel_json(data: &[u8]) -> Result<Vec<SpanInput>> {
    let root: Value = serde_json::from_slice(data)?;
    let mut spans = Vec::new();

    let resource_spans = root["resourceSpans"]
        .as_array()
        .or_else(|| root.as_array())
        .context("Invalid OTel format: expected resourceSpans array")?;

    for resource_span in resource_spans {
        let service_name = resource_span["resource"]["attributes"]
            .as_array()
            .and_then(|attrs| {
                attrs.iter().find(|a| a["key"] == "service.name")
                    .and_then(|a| a["value"]["stringValue"].as_str())
            })
            .unwrap_or("unknown")
            .to_string();

        let scope_spans = resource_span["scopeSpans"].as_array().unwrap_or(&vec![]);
        for scope_span in scope_spans {
            let span_arr = scope_span["spans"].as_array().unwrap_or(&vec![]);
            for span in span_arr {
                let span_input = otel_span_to_input(span, &service_name)?;
                spans.push(span_input);
            }
        }

        if let Some(span_arr) = resource_span["spans"].as_array() {
            for span in span_arr {
                let span_input = otel_span_to_input(span, &service_name)?;
                spans.push(span_input);
            }
        }
    }

    Ok(spans)
}

fn otel_span_to_input(span: &Value, service_name: &str) -> Result<SpanInput> {
    let trace_id = span["traceId"].as_str().unwrap_or("").to_string();
    let span_id = span["spanId"].as_str().unwrap_or("").to_string();
    let parent_span_id = span["parentSpanId"].as_str().map(|s| s.to_string());
    let operation_name = span["name"].as_str().unwrap_or("unknown").to_string();

    let start_time_unix_nano = span["startTimeUnixNano"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| span["startTimeUnixNano"].as_u64())
        .unwrap_or(0);

    let end_time_unix_nano = span["endTimeUnixNano"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| span["endTimeUnixNano"].as_u64())
        .unwrap_or(0);

    let start_time = Utc.timestamp_nanos(start_time_unix_nano as i64);
    let end_time = Utc.timestamp_nanos(end_time_unix_nano as i64);

    let status_code = match span["status"]["code"].as_str() {
        Some("STATUS_CODE_ERROR") => 500,
        _ => 200,
    };

    let mut tags: HashMap<String, Value> = HashMap::new();
    if let Some(attrs) = span["attributes"].as_array() {
        for attr in attrs {
            let key = attr["key"].as_str().unwrap_or("");
            let value = if let Some(v) = attr["value"]["stringValue"].as_str() {
                Value::String(v.to_string())
            } else if let Some(v) = attr["value"]["intValue"].as_str() {
                v.parse::<i64>().ok().map(Value::from).unwrap_or(Value::Null)
            } else if let Some(v) = attr["value"]["boolValue"].as_bool() {
                Value::Bool(v)
            } else {
                Value::Null
            };
            if !key.is_empty() {
                tags.insert(key.to_string(), value);
            }
        }
    }

    Ok(SpanInput {
        trace_id,
        span_id,
        parent_span_id,
        service_name: service_name.to_string(),
        operation_name,
        start_time,
        end_time,
        status_code,
        status_message: None,
        tags: Some(Value::Object(serde_json::Map::new())),
        logs: Some(Value::Array(vec![])),
    })
}

fn parse_jaeger_json(data: &[u8]) -> Result<Vec<SpanInput>> {
    let root: Value = serde_json::from_slice(data)?;
    let mut spans = Vec::new();

    let data_arr = root["data"].as_array().or_else(|| root.as_array()).context("Invalid Jaeger format")?;

    for trace in data_arr {
        let trace_id = trace["traceID"].as_str().unwrap_or("").to_string();
        if let Some(span_arr) = trace["spans"].as_array() {
            for span in span_arr {
                let span_id = span["spanID"].as_str().unwrap_or("").to_string();
                let operation_name = span["operationName"].as_str().unwrap_or("unknown").to_string();
                let parent_span_id = span["references"]
                    .as_array()
                    .and_then(|refs| refs.iter().find(|r| r["refType"] == "CHILD_OF"))
                    .and_then(|r| r["spanID"].as_str())
                    .map(|s| s.to_string());

                let service_name = span["processID"]
                    .as_str()
                    .and_then(|pid| trace["processes"][pid]["serviceName"].as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let start_time_micro = span["startTime"].as_u64().unwrap_or(0);
                let duration_micro = span["duration"].as_u64().unwrap_or(0);
                let start_time = Utc.timestamp_nanos((start_time_micro * 1000) as i64);
                let end_time = Utc.timestamp_nanos(((start_time_micro + duration_micro) * 1000) as i64);

                let mut tags = serde_json::Map::new();
                if let Some(tag_arr) = span["tags"].as_array() {
                    for tag in tag_arr {
                        if let (Some(key), Some(value)) = (tag["key"].as_str(), tag["value"].as_str()) {
                            tags.insert(key.to_string(), Value::String(value.to_string()));
                        }
                    }
                }

                spans.push(SpanInput {
                    trace_id: trace_id.clone(),
                    span_id,
                    parent_span_id,
                    service_name,
                    operation_name,
                    start_time,
                    end_time,
                    status_code: 200,
                    status_message: None,
                    tags: Some(Value::Object(tags)),
                    logs: Some(Value::Array(vec![])),
                });
            }
        }
    }

    Ok(spans)
}

fn parse_zipkin_v2_json(data: &[u8]) -> Result<Vec<SpanInput>> {
    let span_arr: Vec<Value> = serde_json::from_slice(data)?;
    let mut spans = Vec::new();

    for span in span_arr {
        let trace_id = span["traceId"].as_str().unwrap_or("").to_string();
        let span_id = span["id"].as_str().unwrap_or("").to_string();
        let parent_span_id = span["parentId"].as_str().map(|s| s.to_string());
        let operation_name = span["name"].as_str().unwrap_or("unknown").to_string();
        let service_name = span["localEndpoint"]["serviceName"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let timestamp_micro = span["timestamp"].as_u64().unwrap_or(0);
        let duration_micro = span["duration"].as_u64().unwrap_or(0);
        let start_time = Utc.timestamp_nanos((timestamp_micro * 1000) as i64);
        let end_time = Utc.timestamp_nanos(((timestamp_micro + duration_micro) * 1000) as i64);

        let status_code = if span["tags"]["error"].is_string() {
            500
        } else {
            200
        };

        let mut tags_map = serde_json::Map::new();
        if let Some(tags) = span["tags"].as_object() {
            for (k, v) in tags {
                tags_map.insert(k.clone(), v.clone());
            }
        }

        spans.push(SpanInput {
            trace_id,
            span_id,
            parent_span_id,
            service_name,
            operation_name,
            start_time,
            end_time,
            status_code,
            status_message: None,
            tags: Some(Value::Object(tags_map)),
            logs: Some(Value::Array(vec![])),
        });
    }

    Ok(spans)
}

fn parse_otel_proto(_data: &[u8]) -> Result<Vec<SpanInput>> {
    Ok(Vec::new())
}

pub async fn import_spans_batch(pool: &DbPool, spans: &[SpanInput]) -> Result<()> {
    let mut tx = pool.begin().await?;

    for span_input in spans {
        let is_anomaly = span_input.validate().is_err();
        let anomaly_reason = span_input.validate().err();
        let duration_ms = (span_input.end_time - span_input.start_time).num_milliseconds();

        let result = sqlx::query!(
            r#"
            INSERT INTO spans (
                trace_id, span_id, parent_span_id, service_name, operation_name,
                start_time, end_time, duration_ms, status_code, status_message,
                is_anomaly, anomaly_reason, tags, logs
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (trace_id, span_id) DO UPDATE SET
                parent_span_id = EXCLUDED.parent_span_id,
                service_name = EXCLUDED.service_name,
                operation_name = EXCLUDED.operation_name,
                start_time = EXCLUDED.start_time,
                end_time = EXCLUDED.end_time,
                duration_ms = EXCLUDED.duration_ms,
                status_code = EXCLUDED.status_code,
                status_message = EXCLUDED.status_message,
                is_anomaly = EXCLUDED.is_anomaly,
                anomaly_reason = EXCLUDED.anomaly_reason,
                tags = EXCLUDED.tags,
                logs = EXCLUDED.logs
            "#,
            span_input.trace_id,
            span_input.span_id,
            span_input.parent_span_id,
            span_input.service_name,
            span_input.operation_name,
            span_input.start_time,
            span_input.end_time,
            duration_ms,
            span_input.status_code,
            span_input.status_message,
            is_anomaly,
            anomaly_reason,
            span_input.tags.clone().unwrap_or(Value::Object(serde_json::Map::new())),
            span_input.logs.clone().unwrap_or(Value::Array(vec![])),
        )
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            tracing::warn!("Failed to insert span {}: {}", span_input.span_id, e);
        }
    }

    tx.commit().await?;
    Ok(())
}
