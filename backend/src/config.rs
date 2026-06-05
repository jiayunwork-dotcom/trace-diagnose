use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub anomaly_latency_threshold_ms: u64,
    pub max_upload_size: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/trace_diagnose".to_string()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            anomaly_latency_threshold_ms: env::var("ANOMALY_LATENCY_THRESHOLD_MS").unwrap_or_else(|_| "3000".to_string()).parse().unwrap_or(3000),
            max_upload_size: env::var("MAX_UPLOAD_SIZE").unwrap_or_else(|_| "104857600".to_string()).parse().unwrap_or(100 * 1024 * 1024),
        }
    }
}
