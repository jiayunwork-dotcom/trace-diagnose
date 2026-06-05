CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE traces (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    trace_id VARCHAR(128) NOT NULL UNIQUE,
    root_span_id VARCHAR(128),
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,
    duration_ms BIGINT,
    service_count INTEGER DEFAULT 0,
    span_count INTEGER DEFAULT 0,
    has_errors BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_traces_trace_id ON traces(trace_id);
CREATE INDEX idx_traces_start_time ON traces(start_time);

CREATE TABLE spans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    trace_id VARCHAR(128) NOT NULL,
    span_id VARCHAR(128) NOT NULL,
    parent_span_id VARCHAR(128),
    service_name VARCHAR(256) NOT NULL,
    operation_name VARCHAR(512) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    duration_ms BIGINT NOT NULL,
    status_code INTEGER DEFAULT 200,
    status_message TEXT,
    is_anomaly BOOLEAN DEFAULT FALSE,
    anomaly_reason TEXT,
    tags JSONB DEFAULT '{}'::jsonb,
    logs JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(trace_id, span_id)
);

CREATE INDEX idx_spans_trace_id ON spans(trace_id);
CREATE INDEX idx_spans_service_name ON spans(service_name);
CREATE INDEX idx_spans_parent_span_id ON spans(parent_span_id);
CREATE INDEX idx_spans_start_time ON spans(start_time);
CREATE INDEX idx_spans_is_anomaly ON spans(is_anomaly);

CREATE TABLE service_dependencies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    caller_service VARCHAR(256) NOT NULL,
    callee_service VARCHAR(256) NOT NULL,
    call_count BIGINT DEFAULT 0,
    total_duration_ms BIGINT DEFAULT 0,
    avg_duration_ms DOUBLE PRECISION DEFAULT 0,
    error_count BIGINT DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(caller_service, callee_service)
);

CREATE INDEX idx_service_deps_caller ON service_dependencies(caller_service);
CREATE INDEX idx_service_deps_callee ON service_dependencies(callee_service);

CREATE TABLE service_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL,
    operation_name VARCHAR(512) NOT NULL,
    time_bucket TIMESTAMPTZ NOT NULL,
    call_count BIGINT DEFAULT 0,
    error_count BIGINT DEFAULT 0,
    total_duration_ms BIGINT DEFAULT 0,
    min_duration_ms BIGINT,
    max_duration_ms BIGINT,
    p50_ms DOUBLE PRECISION,
    p95_ms DOUBLE PRECISION,
    p99_ms DOUBLE PRECISION,
    duration_histogram JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, operation_name, time_bucket)
);

CREATE INDEX idx_service_metrics_service ON service_metrics(service_name);
CREATE INDEX idx_service_metrics_time_bucket ON service_metrics(time_bucket);

CREATE TABLE slo_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL,
    slo_type VARCHAR(64) NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    target DOUBLE PRECISION NOT NULL,
    window_days INTEGER DEFAULT 30,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, slo_type)
);

CREATE TABLE slo_status (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slo_id UUID NOT NULL REFERENCES slo_configs(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    current_value DOUBLE PRECISION NOT NULL,
    error_budget_remaining DOUBLE PRECISION NOT NULL,
    is_breached BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(slo_id, date)
);

CREATE INDEX idx_slo_status_slo ON slo_status(slo_id);
CREATE INDEX idx_slo_status_date ON slo_status(date);

CREATE TABLE anomaly_traces (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    trace_id VARCHAR(128) NOT NULL,
    anomaly_type VARCHAR(64) NOT NULL,
    severity VARCHAR(32) NOT NULL,
    root_service VARCHAR(256),
    root_operation VARCHAR(512),
    affected_services TEXT[],
    details JSONB DEFAULT '{}'::jsonb,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_anomaly_traces_trace_id ON anomaly_traces(trace_id);
CREATE INDEX idx_anomaly_traces_severity ON anomaly_traces(severity);
CREATE INDEX idx_anomaly_traces_detected_at ON anomaly_traces(detected_at);

CREATE TABLE import_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_name VARCHAR(512),
    format VARCHAR(64) NOT NULL,
    total_spans INTEGER DEFAULT 0,
    processed_spans INTEGER DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_jobs_status ON import_jobs(status);
