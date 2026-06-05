mod config;
mod models;
mod db;
mod handlers;
mod importer;
mod analysis;
mod cache;
mod health;

use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    routing::{get, post, put, delete},
    Router,
    http::Method,
};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::DbPool;
use crate::cache::RedisCache;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DbPool,
    pub cache: Arc<RedisCache>,
    pub import_progress: Arc<RwLock<std::collections::HashMap<String, importer::ImportProgress>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "trace_diagnose_backend=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(Config::from_env());
    let db = db::init_pool(&config.database_url).await?;
    let cache = Arc::new(RedisCache::new(&config.redis_url).await?);

    db::run_migrations(&db).await?;

    let app_state = AppState {
        config: config.clone(),
        db,
        cache,
        import_progress: Arc::new(RwLock::new(std::collections::HashMap::new())),
    };

    let app = Router::new()
        .route("/api/health", get(handlers::health_check))
        .nest("/api/traces", handlers::traces_routes())
        .nest("/api/services", handlers::services_routes())
        .nest("/api/topology", handlers::topology_routes())
        .nest("/api/analysis", handlers::analysis_routes())
        .nest("/api/slo", handlers::slo_routes())
        .nest("/api/alerts", handlers::alerts_routes())
        .nest("/api/import", handlers::import_routes())
        .nest("/api/health-score", handlers::health_routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
                .allow_headers(Any),
        )
        .with_state(app_state.clone());

    let db_clone = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            tracing::info!("Running scheduled health computation...");
            if let Err(e) = health::run_full_health_computation(&db_clone).await {
                tracing::error!("Scheduled health computation failed: {}", e);
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("Server listening on port {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}
