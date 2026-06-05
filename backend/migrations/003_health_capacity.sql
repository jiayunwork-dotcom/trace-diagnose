CREATE TABLE IF NOT EXISTS health_baselines (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL,
    operation_name VARCHAR(512) NOT NULL,
    hour_of_day INTEGER NOT NULL,
    baseline_p99_ms DOUBLE PRECISION NOT NULL DEFAULT 3000,
    data_points INTEGER NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, operation_name, hour_of_day)
);

CREATE INDEX IF NOT EXISTS idx_health_baselines_service ON health_baselines(service_name);
CREATE INDEX IF NOT EXISTS idx_health_baselines_hour ON health_baselines(hour_of_day);

CREATE TABLE IF NOT EXISTS health_score_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL,
    snapshot_time TIMESTAMPTZ NOT NULL,
    total_score DOUBLE PRECISION NOT NULL,
    availability_score DOUBLE PRECISION NOT NULL,
    latency_score DOUBLE PRECISION NOT NULL,
    throughput_stability_score DOUBLE PRECISION NOT NULL,
    error_diversity_score DOUBLE PRECISION NOT NULL,
    availability_weight DOUBLE PRECISION NOT NULL DEFAULT 0.4,
    latency_weight DOUBLE PRECISION NOT NULL DEFAULT 0.3,
    throughput_weight DOUBLE PRECISION NOT NULL DEFAULT 0.2,
    error_diversity_weight DOUBLE PRECISION NOT NULL DEFAULT 0.1,
    raw_metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, snapshot_time)
);

CREATE INDEX IF NOT EXISTS idx_health_snapshots_service ON health_score_snapshots(service_name);
CREATE INDEX IF NOT EXISTS idx_health_snapshots_time ON health_score_snapshots(snapshot_time);

CREATE TABLE IF NOT EXISTS capacity_plans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL,
    snapshot_time TIMESTAMPTZ NOT NULL,
    current_qps DOUBLE PRECISION NOT NULL,
    max_qps DOUBLE PRECISION NOT NULL,
    remaining_capacity DOUBLE PRECISION NOT NULL,
    avg_response_time_ms DOUBLE PRECISION NOT NULL,
    concurrent_peak_p95 INTEGER NOT NULL,
    is_warning BOOLEAN NOT NULL DEFAULT FALSE,
    warning_threshold_pct DOUBLE PRECISION NOT NULL DEFAULT 20,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, snapshot_time)
);

CREATE INDEX IF NOT EXISTS idx_capacity_plans_service ON capacity_plans(service_name);
CREATE INDEX IF NOT EXISTS idx_capacity_plans_time ON capacity_plans(snapshot_time);
CREATE INDEX IF NOT EXISTS idx_capacity_plans_warning ON capacity_plans(is_warning);

CREATE TABLE IF NOT EXISTS health_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type VARCHAR(64) NOT NULL,
    service_name VARCHAR(256) NOT NULL,
    severity VARCHAR(32) NOT NULL,
    message TEXT NOT NULL,
    score DOUBLE PRECISION,
    threshold DOUBLE PRECISION,
    consecutive_hours INTEGER,
    details JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_health_events_service ON health_events(service_name);
CREATE INDEX IF NOT EXISTS idx_health_events_severity ON health_events(severity);
CREATE INDEX IF NOT EXISTS idx_health_events_created ON health_events(created_at);
