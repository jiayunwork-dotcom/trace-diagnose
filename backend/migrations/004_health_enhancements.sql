CREATE TABLE IF NOT EXISTS health_weight_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(256) NOT NULL UNIQUE,
    availability_weight DOUBLE PRECISION NOT NULL DEFAULT 0.4,
    latency_weight DOUBLE PRECISION NOT NULL DEFAULT 0.3,
    throughput_weight DOUBLE PRECISION NOT NULL DEFAULT 0.2,
    error_diversity_weight DOUBLE PRECISION NOT NULL DEFAULT 0.1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_health_weights_service ON health_weight_configs(service_name);

CREATE TABLE IF NOT EXISTS webhook_configs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    url TEXT NOT NULL,
    threshold_score_drop DOUBLE PRECISION NOT NULL DEFAULT 20.0,
    cooldown_minutes INTEGER NOT NULL DEFAULT 60,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_webhook_configs_active ON webhook_configs(is_active);

CREATE TABLE IF NOT EXISTS webhook_delivery_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    webhook_id UUID NOT NULL REFERENCES webhook_configs(id) ON DELETE CASCADE,
    service_name VARCHAR(256) NOT NULL,
    current_score DOUBLE PRECISION NOT NULL,
    previous_score DOUBLE PRECISION NOT NULL,
    score_drop DOUBLE PRECISION NOT NULL,
    response_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_webhook_logs_webhook ON webhook_delivery_logs(webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_service ON webhook_delivery_logs(service_name);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_created ON webhook_delivery_logs(created_at);

ALTER TABLE capacity_plans 
ADD COLUMN IF NOT EXISTS trend_direction VARCHAR(16) DEFAULT 'stable',
ADD COLUMN IF NOT EXISTS predicted_qps_24h DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS hours_to_saturation INTEGER;
