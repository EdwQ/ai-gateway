use std::env;
use std::time::Duration;

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

    // DingTalk OAuth
    pub dingtalk_app_id: String,
    pub dingtalk_app_secret: String,
    pub dingtalk_agent_id: String,

    // JWT
    pub jwt_access_token_expire_minutes: i64,
    pub jwt_refresh_token_expire_days: i64,
    pub jwt_algorithm: String,

    // Quota
    pub default_quota_amount: f64,

    // Prompt audit
    pub prompt_save_mode: String, // off | summary | masked | full

    // Allowed origins for CORS
    pub allowed_origins: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let allowed_origins_str = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| {
            "http://localhost:3000,http://localhost:5173".to_string()
        });

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

            // DingTalk
            dingtalk_app_id: env::var("DINGTALK_APP_ID").unwrap_or_default(),
            dingtalk_app_secret: env::var("DINGTALK_APP_SECRET").unwrap_or_default(),
            dingtalk_agent_id: env::var("DINGTALK_AGENT_ID").unwrap_or_default(),

            // JWT
            jwt_access_token_expire_minutes: env::var("JWT_ACCESS_TOKEN_EXPIRE_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            jwt_refresh_token_expire_days: env::var("JWT_REFRESH_TOKEN_EXPIRE_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            jwt_algorithm: "HS256".to_string(),

            // Quota
            default_quota_amount: env::var("DEFAULT_QUOTA_AMOUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50.0),

            // Prompt audit
            prompt_save_mode: env::var("PROMPT_SAVE_MODE")
                .unwrap_or_else(|_| "off".to_string()),

            // CORS
            allowed_origins: allowed_origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }

    pub fn jwt_access_expires_in(&self) -> Duration {
        Duration::from_secs((self.jwt_access_token_expire_minutes * 60) as u64)
    }

    pub fn jwt_refresh_expires_in(&self) -> Duration {
        Duration::from_secs((self.jwt_refresh_token_expire_days * 86400) as u64)
    }
}
