mod auth;
mod config;
mod db;
mod dingtalk;
mod mask;
mod models;
mod proxy;
mod rate_limit;
mod redis;
mod routes;
mod security;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::config::AppConfig;
use crate::db::AppDb;
use crate::redis::AppRedis;
use crate::routes::{alias_routes, audit_routes, auth_routes, gateway, health, provider_routes, stats_routes, token_routes, user_routes};

#[tokio::main]
async fn main() {
    use std::str::FromStr;

    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(
            tracing_subscriber::filter::Directive::from_str("ai_gateway_rs=info").unwrap()
        )
        .add_directive(
            tracing_subscriber::filter::Directive::from_str("sqlx=warn").unwrap()
        )
        .add_directive(
            tracing_subscriber::filter::Directive::from_str("reqwest=warn").unwrap()
        );

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Load .env file (optional)
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env();

    // Initialize database
    let db = AppDb::new(&config)
        .await
        .expect("Failed to connect to database");
    let db_pool = Arc::new(db);

    // Initialize Redis
    let redis = AppRedis::new(&config)
        .await
        .expect("Failed to connect to Redis");
    let redis_pool = Arc::new(redis);

    // HTTP client for upstream proxy
    let http_client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client"),
    );

    // Key manager
    let key_manager = Arc::new(proxy::KeyManager::new());

    let config_arc = Arc::new(config);

    let state = gateway::AppState {
        db: db_pool,
        redis: redis_pool,
        config: config_arc.clone(),
        key_manager,
        http_client,
    };

    let addr = config_arc.listen_addr.clone();

    // Build router
    let app = Router::new()
        // Health endpoints
        .route("/health/liveness", get(health::liveness))
        .route("/health/readiness", get(health::readiness))
        // AI Gateway endpoints
        .route("/v1/chat/completions", post(gateway::chat_completions))
        .route("/v1/embeddings", post(gateway::embeddings))
        .route("/v1/models", get(gateway::list_models))
        // Auth endpoints
        .route("/api/v1/auth/dingtalk/qrcode", post(auth_routes::dingtalk_qrcode))
        .route("/api/v1/auth/dingtalk/callback", get(auth_routes::dingtalk_callback_get).post(auth_routes::dingtalk_callback_post))
        .route("/api/v1/auth/dev/login", post(auth_routes::dev_login))
        .route("/api/v1/auth/init", post(auth_routes::init_admin))
        .route("/api/v1/auth/refresh", post(auth_routes::refresh_token))
        .route("/api/v1/auth/logout", post(auth_routes::logout))
        .route("/api/v1/auth/me", get(auth_routes::me))
        // Token management
        .route("/api/v1/tokens", get(token_routes::list_tokens).post(token_routes::create_token))
        .route("/api/v1/tokens/{id}", delete(token_routes::delete_token))
        .route("/api/v1/tokens/{id}/rotate", post(token_routes::rotate_token))
        // User management
        .route("/api/v1/users", get(user_routes::list_users))
        .route("/api/v1/users/{id}", get(user_routes::get_user).patch(user_routes::update_user).delete(user_routes::deactivate_user))
        // Provider management
        .route("/api/v1/admin/providers", get(provider_routes::list_providers).post(provider_routes::create_provider))
        .route("/api/v1/admin/providers/discover-models", post(provider_routes::discover_models))
        .route("/api/v1/admin/providers/{id}", put(provider_routes::update_provider).delete(provider_routes::delete_provider))
        .route("/api/v1/admin/providers/{id}/check", post(provider_routes::check_provider_health))
        // Model alias management
        .route("/api/v1/admin/model-aliases", get(alias_routes::list_aliases).post(alias_routes::create_alias))
        .route("/api/v1/admin/model-aliases/{id}", put(alias_routes::update_alias).delete(alias_routes::delete_alias))
        // Stats & BI
        .route("/api/v1/stats/dashboard", get(stats_routes::get_dashboard))
        .route("/api/v1/stats/daily", get(stats_routes::get_daily_stats))
        .route("/api/v1/stats/monthly", get(stats_routes::get_monthly_stats))
        .route("/api/v1/stats/export", get(stats_routes::export_stats))
        // Audit
        .route("/api/v1/audit/logs", get(audit_routes::list_audit_logs))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    tracing::info!("AI Gateway Rust proxy starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");
    axum::serve(listener, app)
        .await
        .expect("Server error");
}
