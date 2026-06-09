use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub secret_key: String,
    pub encryption_key: String,
    pub frontend_url: String,
    pub rate_limit_user_qps: u32,
    pub rate_limit_provider_qps: u32,
    pub debug: bool,
    pub listen_addr: String,
    pub python_backend_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:postgres@localhost:5432/ai_gateway".to_string()
            }),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| {
                "redis://localhost:6379/0".to_string()
            }),
            secret_key: env::var("SECRET_KEY").unwrap_or_else(|_| {
                "change-this-to-a-random-secret-key-min-32-chars".to_string()
            }),
            encryption_key: env::var("ENCRYPTION_KEY").unwrap_or_else(|_| {
                "change-this-to-32-byte-key!!".to_string()
            }),
            frontend_url: env::var("FRONTEND_URL").unwrap_or_else(|_| {
                "http://localhost:3000".to_string()
            }),
            rate_limit_user_qps: env::var("RATE_LIMIT_USER_QPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            rate_limit_provider_qps: env::var("RATE_LIMIT_PROVIDER_QPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            debug: env::var("DEBUG").ok().map(|v| v == "true").unwrap_or(false),
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| {
                "0.0.0.0:2887".to_string()
            }),
            python_backend_url: env::var("PYTHON_BACKEND_URL").unwrap_or_else(|_| {
                "http://backend:8001".to_string()
            }),
        }
    }
}
