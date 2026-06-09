mod auth;
mod config;
mod db;
mod models;
mod proxy;
mod rate_limit;
mod redis;
mod routes;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::config::AppConfig;
use crate::db::AppDb;
use crate::redis::AppRedis;
use crate::routes::{gateway, health};

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
