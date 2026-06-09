use axum::{extract::State, Json, response::IntoResponse};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::ReadinessResponse;
use crate::routes::gateway::AppState;

pub async fn liveness() -> impl IntoResponse {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as f64;
    Json(json!({"status": "alive", "timestamp": timestamp}))
}

pub async fn readiness(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as f64;

    // Check database
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db.pool)
        .await
        .is_ok();

    // Check Redis
    let mut conn = state.redis.conn.clone();
    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .is_ok();

    if !db_ok || !redis_ok {
        let resp = ReadinessResponse {
            status: "not ready".to_string(),
            database: if db_ok { "ok".to_string() } else { "down".to_string() },
            redis: if redis_ok { "ok".to_string() } else { "down".to_string() },
            timestamp,
        };
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(json!(resp)));
    }

    let resp = ReadinessResponse {
        status: "ready".to_string(),
        database: "ok".to_string(),
        redis: "ok".to_string(),
        timestamp,
    };
    (axum::http::StatusCode::OK, Json(json!(resp)))
}
