use redis::aio::MultiplexedConnection;
use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppRedis {
    pub conn: MultiplexedConnection,
}

impl AppRedis {
    pub async fn new(config: &AppConfig) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(config.redis_url.as_str())?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn })
    }
}

pub type RedisPool = Arc<AppRedis>;
