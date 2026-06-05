use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;
use std::path::Path;

pub type DbPool = PgPool;

pub async fn init_pool(database_url: &str) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    let migrations_dir = Path::new("migrations");
    if migrations_dir.exists() {
        let mut entries = std::fs::read_dir(migrations_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            if entry.extension().and_then(|s| s.to_str()) == Some("sql") {
                let sql = std::fs::read_to_string(&entry)?;
                sqlx::query(&sql).execute(pool).await?;
                tracing::info!("Applied migration: {:?}", entry.file_name());
            }
        }
    }
    Ok(())
}
