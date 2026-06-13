//! API Token management routes.
//!
//! Ported from `backend/app/api/v1/tokens.py`

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use uuid::Uuid;

use crate::models::{ApiTokenCreateRequest, ApiTokenCreatedResponse, ApiTokenListResponse, ApiTokenResponse, ApiTokenRotateResponse};
use crate::routes::auth_routes::extract_jwt_user;
use crate::routes::gateway::AppState;
use crate::security;

/// Maximum active tokens per user
const MAX_TOKENS_PER_USER: i64 = 10;

// ============================================================
// GET /api/v1/tokens
// ============================================================

/// List current user's API tokens.
pub async fn list_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    let tokens = sqlx::query_as::<_, crate::models::ApiToken>(
        "SELECT id, user_id, token_hash, token_prefix, name, is_active, \
         last_used_at, expires_at, created_at, updated_at \
         FROM api_tokens WHERE user_id = $1 \
         ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<ApiTokenResponse> = tokens
        .into_iter()
        .map(|t| ApiTokenResponse {
            id: t.id.to_string(),
            token_prefix: t.token_prefix,
            name: t.name,
            is_active: t.is_active,
            last_used_at: t.last_used_at,
            expires_at: t.expires_at,
            created_at: t.created_at,
        })
        .collect();

    (StatusCode::OK, Json(ApiTokenListResponse { items })).into_response()
}

// ============================================================
// POST /api/v1/tokens
// ============================================================

/// Create a new API token.
pub async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ApiTokenCreateRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    // Check active token count
    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM api_tokens WHERE user_id = $1 AND is_active = true"
    )
    .bind(user.id)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or((0,));

    if active_count.0 >= MAX_TOKENS_PER_USER {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "detail": format!("Maximum {} active tokens allowed", MAX_TOKENS_PER_USER)
            })),
        )
            .into_response();
    }

    // Generate token
    let raw_token = security::generate_api_token();
    let token_hash = security::hash_token(&raw_token);
    let token_prefix = raw_token[..20].to_string(); // "sk-company-a1b2c3d4e5"

    let token_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let result = sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, token_prefix, name, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(token_id)
    .bind(user.id)
    .bind(&token_hash)
    .bind(&token_prefix)
    .bind(&body.name)
    .bind(true)
    .bind(now)
    .bind(now)
    .execute(&state.db.pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ApiTokenCreatedResponse {
                id: token_id.to_string(),
                token: raw_token,
                name: body.name,
                created_at: Some(now),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"detail": format!("DB error: {}", e)})),
        )
            .into_response(),
    }
}

// ============================================================
// DELETE /api/v1/tokens/{token_id}
// ============================================================

/// Deactivate (soft-delete) an API token.
pub async fn delete_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    let result = sqlx::query_as::<_, crate::models::ApiToken>(
        "SELECT id, user_id, token_hash, token_prefix, name, is_active, \
         last_used_at, expires_at, created_at, updated_at \
         FROM api_tokens WHERE id = $1 AND user_id = $2"
    )
    .bind(token_id)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await;

    let token = match result {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Token not found"}))).into_response()
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response()
        }
    };

    let _ = sqlx::query("UPDATE api_tokens SET is_active = false WHERE id = $1")
        .bind(token.id)
        .execute(&state.db.pool)
        .await;

    (
        StatusCode::OK,
        Json(ApiTokenResponse {
            id: token.id.to_string(),
            token_prefix: token.token_prefix,
            name: token.name,
            is_active: false,
            last_used_at: token.last_used_at,
            expires_at: token.expires_at,
            created_at: token.created_at,
        }),
    )
        .into_response()
}

// ============================================================
// POST /api/v1/tokens/{token_id}/rotate
// ============================================================

/// Rotate an API token (deactivate old, create new).
pub async fn rotate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    // Find existing token
    let old_token = sqlx::query_as::<_, crate::models::ApiToken>(
        "SELECT id, user_id, token_hash, token_prefix, name, is_active, \
         last_used_at, expires_at, created_at, updated_at \
         FROM api_tokens WHERE id = $1 AND user_id = $2"
    )
    .bind(token_id)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await;

    let old_token = match old_token {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Token not found"}))).into_response()
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response()
        }
    };

    // Deactivate old token
    let _ = sqlx::query("UPDATE api_tokens SET is_active = false WHERE id = $1")
        .bind(old_token.id)
        .execute(&state.db.pool)
        .await;

    // Generate new token
    let raw_token = security::generate_api_token();
    let token_hash = security::hash_token(&raw_token);
    let token_prefix = raw_token[..20].to_string();
    let new_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let result = sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, token_prefix, name, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(new_id)
    .bind(user.id)
    .bind(&token_hash)
    .bind(&token_prefix)
    .bind(&old_token.name)
    .bind(true)
    .bind(now)
    .bind(now)
    .execute(&state.db.pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiTokenRotateResponse {
                id: new_id.to_string(),
                token: raw_token,
                name: old_token.name,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"detail": format!("DB error: {}", e)})),
        )
            .into_response(),
    }
}
