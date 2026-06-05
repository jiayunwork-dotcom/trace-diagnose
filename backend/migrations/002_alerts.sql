CREATE TABLE alert_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    service_name VARCHAR(256),
    operation_name VARCHAR(512),
    metric_type VARCHAR(64) NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    comparison_operator VARCHAR(16) NOT NULL DEFAULT '>',
    window_minutes INTEGER NOT NULL DEFAULT 5,
    consecutive_windows INTEGER NOT NULL DEFAULT 1,
    severity VARCHAR(32) NOT NULL DEFAULT 'warning',
    silence_minutes INTEGER NOT NULL DEFAULT 30,
    is_active BOOLEAN DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_rules_service ON alert_rules(service_name);
CREATE INDEX idx_alert_rules_is_active ON alert_rules(is_active);

CREATE TABLE alert_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rule_id UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    rule_name VARCHAR(256) NOT NULL,
    service_name VARCHAR(256),
    operation_name VARCHAR(512),
    metric_type VARCHAR(64) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'firing',
    trace_ids TEXT[] DEFAULT '{}',
    acknowledged_by VARCHAR(256),
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    firing_started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_events_rule_id ON alert_events(rule_id);
CREATE INDEX idx_alert_events_status ON alert_events(status);
CREATE INDEX idx_alert_events_created_at ON alert_events(created_at);
CREATE INDEX idx_alert_events_service ON alert_events(service_name);
