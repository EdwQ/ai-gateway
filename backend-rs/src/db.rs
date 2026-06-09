use sqlx::PgPool;
use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppDb {
    pub pool: PgPool,
}

impl AppDb {
    pub async fn new(config: &AppConfig) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(&config.database_url).await?;
        Ok(Self { pool })
    }
}

pub type DbPool = Arc<AppDb>;
