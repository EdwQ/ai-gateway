//! Provider management routes.
//!
//! Ported from `backend/app/api/v1/providers.py`

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use std::time::Instant;
use uuid::Uuid;

use crate::models::{
    Provider, ProviderCreateRequest, ProviderListResponse, ProviderResponse,
    ProviderUpdateRequest, HealthCheckResponse, DiscoverModelsRequest, DiscoverModelsResponse,
};
use crate::routes::auth_routes::extract_jwt_user;
use crate::routes::gateway::AppState;
use crate::security;

// ============================================================
// Helper
// ============================================================

fn provider_to_response(p: &Provider) -> ProviderResponse {
    ProviderResponse {
        id: p.id.to_string(),
        name: p.name.clone(),
        display_name: p.display_name.clone(),
        base_url: p.base_url.clone(),
        models: p.models.0.clone(),
        is_active: p.is_active,
        priority: p.priority,
        health_status: p.health_status.clone(),
        rate_limit_qps: p.rate_limit_qps,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

fn require_role<'a>(
    user: &crate::models::User,
    allowed_roles: &[&str],
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if allowed_roles.contains(&user.role.as_str()) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "detail": format!("Requires one of roles: {}", allowed_roles.join(", "))
            })),
        ))
    }
}

// ============================================================
// GET /api/v1/admin/providers
// ============================================================

pub async fn list_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    let providers = sqlx::query_as::<_, Provider>(
        "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
         priority, health_status, rate_limit_qps, created_at, updated_at \
         FROM providers ORDER BY priority"
    )
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<ProviderResponse> = providers.iter().map(provider_to_response).collect();
    Json(ProviderListResponse { items }).into_response()
}

// ============================================================
// POST /api/v1/admin/providers
// ============================================================

pub async fn create_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProviderCreateRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    // Encrypt API key
    let encrypted = match security::encrypt_value(&body.api_key, &state.config.encryption_key) {
        Ok(e) => e,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": e}))).into_response()
        }
    };

    let provider_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Serialize models to JSON
    let models_json = serde_json::to_value(&body.models).unwrap_or(serde_json::Value::Array(vec![]));

    let result = sqlx::query(
        "INSERT INTO providers (id, name, display_name, base_url, api_key_encrypted, models, \
         is_active, priority, health_status, rate_limit_qps, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
    )
    .bind(provider_id)
    .bind(&body.name)
    .bind(&body.display_name)
    .bind(&body.base_url)
    .bind(&encrypted)
    .bind(&models_json)
    .bind(body.is_active)
    .bind(body.priority)
    .bind("unknown")
    .bind(body.rate_limit_qps)
    .bind(now)
    .bind(now)
    .execute(&state.db.pool)
    .await;

    match result {
        Ok(_) => {
            let provider = sqlx::query_as::<_, Provider>(
                "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
                 priority, health_status, rate_limit_qps, created_at, updated_at \
                 FROM providers WHERE id = $1"
            )
            .bind(provider_id)
            .fetch_one(&state.db.pool)
            .await;

            match provider {
                Ok(p) => (StatusCode::CREATED, Json(provider_to_response(&p))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
            }
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("duplicate key") {
                (StatusCode::CONFLICT, Json(serde_json::json!({"detail": format!("Provider '{}' already exists", body.name)}))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response()
            }
        }
    }
}

// ============================================================
// PUT /api/v1/admin/providers/{provider_id}
// ============================================================

pub async fn update_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_id): Path<Uuid>,
    Json(body): Json<ProviderUpdateRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    // Check exists
    let existing = sqlx::query_as::<_, Provider>(
        "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
         priority, health_status, rate_limit_qps, created_at, updated_at \
         FROM providers WHERE id = $1"
    )
    .bind(provider_id)
    .fetch_optional(&state.db.pool)
    .await;

    if let Ok(None) = existing {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Provider not found"}))).into_response();
    }
    if let Err(e) = existing {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    // Pre-compute encrypted API key if changing
    let new_encrypted_key = body.api_key.as_ref().map(|k|
        security::encrypt_value(k, &state.config.encryption_key).unwrap_or_default()
    );

    // Build dynamic UPDATE
    let mut set_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1i32;

    if body.display_name.is_some() {
        set_clauses.push(format!("display_name = ${}", param_idx)); param_idx += 1;
    }
    if body.base_url.is_some() {
        set_clauses.push(format!("base_url = ${}", param_idx)); param_idx += 1;
    }
    if new_encrypted_key.is_some() {
        set_clauses.push(format!("api_key_encrypted = ${}", param_idx)); param_idx += 1;
    }
    if body.models.is_some() {
        set_clauses.push(format!("models = ${}::jsonb", param_idx)); param_idx += 1;
    }
    if body.is_active.is_some() {
        set_clauses.push(format!("is_active = ${}", param_idx)); param_idx += 1;
    }
    if body.priority.is_some() {
        set_clauses.push(format!("priority = ${}", param_idx)); param_idx += 1;
    }
    if body.rate_limit_qps.is_some() {
        set_clauses.push(format!("rate_limit_qps = ${}", param_idx)); param_idx += 1;
    }

    set_clauses.push(format!("updated_at = ${}", param_idx)); param_idx += 1;

    if set_clauses.is_empty() {
        // Return as-is
        let p = existing.unwrap().unwrap();
        return Json(provider_to_response(&p)).into_response();
    }

    let set_sql = set_clauses.join(", ");
    let query_sql = format!("UPDATE providers SET {} WHERE id = ${}", set_sql, param_idx);

    let mut query = sqlx::query(&query_sql);

    if let Some(ref v) = body.display_name { query = query.bind(v); }
    if let Some(ref v) = body.base_url { query = query.bind(v); }
    if let Some(ref v) = new_encrypted_key { query = query.bind(v); }
    if let Some(ref v) = body.models {
        let json_val = serde_json::to_value(v).unwrap_or(serde_json::Value::Null);
        query = query.bind(json_val);
    }
    if let Some(v) = body.is_active { query = query.bind(v); }
    if let Some(v) = body.priority { query = query.bind(v); }
    if let Some(v) = body.rate_limit_qps { query = query.bind(v); }

    query = query.bind(chrono::Utc::now());
    query = query.bind(provider_id);

    if let Err(e) = query.execute(&state.db.pool).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    // Fetch updated
    let updated = sqlx::query_as::<_, Provider>(
        "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
         priority, health_status, rate_limit_qps, created_at, updated_at \
         FROM providers WHERE id = $1"
    )
    .bind(provider_id)
    .fetch_one(&state.db.pool)
    .await;

    match updated {
        Ok(p) => Json(provider_to_response(&p)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

// ============================================================
// DELETE /api/v1/admin/providers/{provider_id}
// ============================================================

pub async fn delete_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_id): Path<Uuid>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["super_admin"]) {
        return (status, json).into_response();
    }

    let result = sqlx::query("DELETE FROM providers WHERE id = $1")
        .bind(provider_id)
        .execute(&state.db.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Json(serde_json::json!({"message": "Provider deleted successfully"})).into_response()
        }
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Provider not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

// ============================================================
// POST /api/v1/admin/providers/{provider_id}/check
// ============================================================

pub async fn check_provider_health(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_id): Path<Uuid>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    let provider = sqlx::query_as::<_, Provider>(
        "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
         priority, health_status, rate_limit_qps, created_at, updated_at \
         FROM providers WHERE id = $1"
    )
    .bind(provider_id)
    .fetch_optional(&state.db.pool)
    .await;

    let provider = match provider {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Provider not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    };

    // Decrypt API key
    let api_key = match security::decrypt_value(&provider.api_key_encrypted, &state.config.encryption_key) {
        Ok(k) => k,
        Err(_) => {
            let _ = sqlx::query("UPDATE providers SET health_status = 'down' WHERE id = $1")
                .bind(provider_id)
                .execute(&state.db.pool)
                .await;
            return Json(HealthCheckResponse { status: "down (decrypt error)".to_string(), latency_ms: 0.0 }).into_response();
        }
    };

    let base_url = provider.base_url.trim_end_matches('/').to_string();
    let url = format!("{}/v1/models", base_url);

    let start = Instant::now();
    match state.http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as f64;
            let status_code = resp.status().as_u16();

            let (status_text, health_status) = if status_code == 200 {
                ("healthy".to_string(), "healthy")
            } else {
                (format!("degraded (HTTP {})", status_code), "degraded")
            };

            let _ = sqlx::query("UPDATE providers SET health_status = $1 WHERE id = $2")
                .bind(health_status)
                .bind(provider_id)
                .execute(&state.db.pool)
                .await;

            Json(HealthCheckResponse { status: status_text, latency_ms: latency }).into_response()
        }
        Err(e) => {
            let _ = sqlx::query("UPDATE providers SET health_status = 'down' WHERE id = $1")
                .bind(provider_id)
                .execute(&state.db.pool)
                .await;

            Json(HealthCheckResponse {
                status: format!("down ({})", &e.to_string()[..e.to_string().len().min(100)]),
                latency_ms: 0.0,
            }).into_response()
        }
    }
}

// ============================================================
// POST /api/v1/admin/providers/discover-models
// ============================================================

pub async fn discover_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DiscoverModelsRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    let base_url = body.base_url.trim_end_matches('/').to_string();
    let url = format!("{}/v1/models", base_url);

    match state.http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", body.api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status_code = resp.status().as_u16();
                return Json(DiscoverModelsResponse {
                    models: vec![],
                    error: Some(format!("HTTP {}", status_code)),
                }).into_response();
            }

            let data: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
            let models_array = data.get("data").and_then(|d| d.as_array());

            let mut model_ids = Vec::new();
            if let Some(models) = models_array {
                for m in models {
                    if let Some(id) = m.get("id").and_then(|v| v.as_str()) {
                        model_ids.push(id.to_string());
                    } else if let Some(s) = m.as_str() {
                        model_ids.push(s.to_string());
                    }
                }
            }

            Json(DiscoverModelsResponse { models: model_ids, error: None }).into_response()
        }
        Err(e) => {
            Json(DiscoverModelsResponse {
                models: vec![],
                error: Some(e.to_string()[..e.to_string().len().min(200)].to_string()),
            }).into_response()
        }
    }
}
