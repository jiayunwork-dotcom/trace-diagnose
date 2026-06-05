use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    async fn get_conn(&self) -> Result<MultiplexedConnection> {
        let conn = self.client.get_multiplexed_async_connection().await?;
        Ok(conn)
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_conn().await?;
        let data: Option<String> = conn.get(key).await?;
        match data {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let data = serde_json::to_string(value)?;
        conn.set_ex(key, data, ttl.as_secs() as usize).await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_conn().await?;
        conn.del(key).await?;
        Ok(())
    }

    pub async fn increment(&self, key: &str, amount: i64) -> Result<i64> {
        let mut conn = self.get_conn().await?;
        let result = conn.incr(key, amount).await?;
        Ok(result)
    }
}
