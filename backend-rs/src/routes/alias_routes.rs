//! Model alias management routes.
//!
//! Ported from `backend/app/api/v1/aliases.py`

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use uuid::Uuid;

use crate::models::{AliasCreateRequest, AliasListResponse, AliasResponse, AliasUpdateRequest, ModelAlias};
use crate::routes::auth_routes::extract_jwt_user;
use crate::routes::gateway::AppState;

// ============================================================
// Helper
// ============================================================

fn alias_to_response(a: &ModelAlias) -> AliasResponse {
    AliasResponse {
        id: a.id.to_string(),
        alias_name: a.alias_name.clone(),
        target_model: a.target_model.clone(),
        description: a.description.clone(),
        is_active: a.is_active,
        created_at: a.created_at,
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
// GET /api/v1/admin/model-aliases
// ============================================================

pub async fn list_aliases(
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

    let aliases = sqlx::query_as::<_, ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases ORDER BY alias_name"
    )
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<AliasResponse> = aliases.iter().map(alias_to_response).collect();
    Json(AliasListResponse { items }).into_response()
}

// ============================================================
// POST /api/v1/admin/model-aliases
// ============================================================

pub async fn create_alias(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AliasCreateRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    // Check uniqueness
    let existing = sqlx::query_as::<_, ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases WHERE alias_name = $1"
    )
    .bind(&body.alias_name)
    .fetch_optional(&state.db.pool)
    .await;

    if let Ok(Some(_)) = existing {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "detail": format!("Alias '{}' already exists", body.alias_name)
        }))).into_response();
    }
    if let Err(e) = existing {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    let alias_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let result = sqlx::query(
        "INSERT INTO model_aliases (id, alias_name, target_model, description, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(alias_id)
    .bind(&body.alias_name)
    .bind(&body.target_model)
    .bind(&body.description)
    .bind(body.is_active)
    .bind(now)
    .bind(now)
    .execute(&state.db.pool)
    .await;

    match result {
        Ok(_) => {
            let alias = sqlx::query_as::<_, ModelAlias>(
                "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
                 FROM model_aliases WHERE id = $1"
            )
            .bind(alias_id)
            .fetch_one(&state.db.pool)
            .await;

            match alias {
                Ok(a) => (StatusCode::CREATED, Json(alias_to_response(&a))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
            }
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response()
        }
    }
}

// ============================================================
// PUT /api/v1/admin/model-aliases/{alias_id}
// ============================================================

pub async fn update_alias(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alias_id): Path<Uuid>,
    Json(body): Json<AliasUpdateRequest>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    // Check exists
    let existing = sqlx::query_as::<_, ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases WHERE id = $1"
    )
    .bind(alias_id)
    .fetch_optional(&state.db.pool)
    .await;

    if let Ok(None) = existing {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Alias not found"}))).into_response();
    }
    if let Err(e) = existing {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    // If alias_name is changing, check uniqueness
    if let Some(ref new_name) = body.alias_name {
        let dup = sqlx::query_as::<_, ModelAlias>(
            "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
             FROM model_aliases WHERE alias_name = $1 AND id != $2"
        )
        .bind(new_name)
        .bind(alias_id)
        .fetch_optional(&state.db.pool)
        .await;

        if let Ok(Some(_)) = dup {
            return (StatusCode::CONFLICT, Json(serde_json::json!({
                "detail": format!("Alias '{}' already exists", new_name)
            }))).into_response();
        }
        if let Err(e) = dup {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
        }
    }

    // Build dynamic UPDATE
    let mut set_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1i32;

    if body.alias_name.is_some() {
        set_clauses.push(format!("alias_name = ${}", param_idx)); param_idx += 1;
    }
    if body.target_model.is_some() {
        set_clauses.push(format!("target_model = ${}", param_idx)); param_idx += 1;
    }
    if body.description.is_some() {
        set_clauses.push(format!("description = ${}", param_idx)); param_idx += 1;
    }
    if body.is_active.is_some() {
        set_clauses.push(format!("is_active = ${}", param_idx)); param_idx += 1;
    }

    set_clauses.push(format!("updated_at = ${}", param_idx)); param_idx += 1;

    if set_clauses.is_empty() {
        // Return existing
        let a = existing.unwrap().unwrap();
        return Json(alias_to_response(&a)).into_response();
    }

    let set_sql = set_clauses.join(", ");
    let query_sql = format!("UPDATE model_aliases SET {} WHERE id = ${}", set_sql, param_idx);

    let mut query = sqlx::query(&query_sql);

    if let Some(ref v) = body.alias_name { query = query.bind(v); }
    if let Some(ref v) = body.target_model { query = query.bind(v); }
    if let Some(ref v) = body.description { query = query.bind(v); }
    if let Some(v) = body.is_active { query = query.bind(v); }

    query = query.bind(chrono::Utc::now());
    query = query.bind(alias_id);

    if let Err(e) = query.execute(&state.db.pool).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    // Fetch updated
    let updated = sqlx::query_as::<_, ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases WHERE id = $1"
    )
    .bind(alias_id)
    .fetch_one(&state.db.pool)
    .await;

    match updated {
        Ok(a) => Json(alias_to_response(&a)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

// ============================================================
// DELETE /api/v1/admin/model-aliases/{alias_id}
// ============================================================

pub async fn delete_alias(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alias_id): Path<Uuid>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["super_admin"]) {
        return (status, json).into_response();
    }

    let result = sqlx::query("DELETE FROM model_aliases WHERE id = $1")
        .bind(alias_id)
        .execute(&state.db.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Json(serde_json::json!({"message": "Alias deleted successfully"})).into_response()
        }
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Alias not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}
