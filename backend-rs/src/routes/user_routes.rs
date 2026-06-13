//! User management routes.
//!
//! Ported from `backend/app/api/v1/users.py`

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

use crate::models::{
    User, UserListResponse, UserQueryParams, UserResponse, UserUpdateRequest,
    sqlx_decimal_to_rust,
};
use crate::routes::auth_routes::extract_jwt_user;
use crate::routes::gateway::AppState;

// ============================================================
// Role checking
// ============================================================

fn require_role<'a>(
    user: &User,
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

fn user_to_response(user: &User) -> UserResponse {
    UserResponse {
        id: user.id.to_string(),
        union_id: user.union_id.clone(),
        name: user.name.clone(),
        email: user.email.clone(),
        avatar: user.avatar.clone(),
        department_id: user.department_id.clone(),
        department_name: user.department_name.clone(),
        title: user.title.clone(),
        role: user.role.clone(),
        allowed_models: user.allowed_models.as_ref()
            .map(|j| j.0.clone())
            .unwrap_or_default(),
        is_active: user.is_active,
        quota_balance: sqlx_decimal_to_rust(&user.quota_balance),
        quota_used: sqlx_decimal_to_rust(&user.quota_used),
        last_login_at: user.last_login_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}

// ============================================================
// GET /api/v1/users
// ============================================================

pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<UserQueryParams>,
) -> impl IntoResponse {
    let (current_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&current_user, &["admin", "super_admin", "finance"]) {
        return (status, json).into_response();
    }

    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Build conditions vector
    let search_pattern = params.search.as_ref().map(|s| format!("%{}%", s));

    // Count query — use a separate COUNT SQL
    let (count_where, _) = build_where_clause(&search_pattern, params.is_active, &params.role);
    let count_sql = format!("SELECT COUNT(*) FROM users {}", count_where);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ref pattern) = search_pattern {
        count_query = count_query.bind(pattern).bind(pattern).bind(pattern);
    }
    if let Some(active) = params.is_active {
        count_query = count_query.bind(active);
    }
    if let Some(ref role) = params.role {
        count_query = count_query.bind(role);
    }
    let total = count_query.fetch_one(&state.db.pool).await.unwrap_or((0,)).0;

    // Data query
    let (data_sql, param_count) = build_user_query(&search_pattern, params.is_active, &params.role, true);
    let limit_idx = param_count + 1;
    let offset_idx = param_count + 2;
    let data_sql = format!("{} LIMIT ${} OFFSET ${}", data_sql, limit_idx, offset_idx);

    let mut data_query = sqlx::query_as::<_, User>(&data_sql);
    if let Some(ref pattern) = search_pattern {
        data_query = data_query.bind(pattern).bind(pattern).bind(pattern);
    }
    if let Some(active) = params.is_active {
        data_query = data_query.bind(active);
    }
    if let Some(ref role) = params.role {
        data_query = data_query.bind(role);
    }
    data_query = data_query.bind(page_size as i64).bind(offset as i64);

    let users = data_query.fetch_all(&state.db.pool).await.unwrap_or_default();

    let items: Vec<UserResponse> = users.iter().map(user_to_response).collect();
    Json(UserListResponse { items, total, page, page_size }).into_response()
}

/// Build WHERE clause for user queries.
/// Returns (where_clause_string, number_of_parameters_used).
fn build_where_clause(
    search: &Option<String>,
    is_active: Option<bool>,
    role: &Option<String>,
) -> (String, i32) {
    let mut conditions = Vec::new();
    let mut param_idx = 0i32;

    if search.is_some() {
        conditions.push(format!(
            "(name ILIKE ${} OR email ILIKE ${} OR department_name ILIKE ${})",
            param_idx + 1, param_idx + 2, param_idx + 3
        ));
        param_idx += 3;
    }
    if is_active.is_some() {
        conditions.push(format!("is_active = ${}", param_idx + 1));
        param_idx += 1;
    }
    if role.is_some() {
        conditions.push(format!("role = ${}", param_idx + 1));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    (where_clause, param_idx)
}

/// Build dynamic SQL for user list query.
/// Returns (sql_string, number_of_parameter_placeholders_used).
fn build_user_query(
    search: &Option<String>,
    is_active: Option<bool>,
    role: &Option<String>,
    ordered: bool,
) -> (String, i32) {
    let (where_clause, param_idx) = build_where_clause(search, is_active, role);
    let order_clause = if ordered { "ORDER BY created_at DESC" } else { "" };

    let sql = format!(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users {} {}",
        where_clause, order_clause
    );

    (sql, param_idx)
}

// ============================================================
// GET /api/v1/users/{user_id}
// ============================================================

pub async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let (current_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&current_user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    let user = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db.pool)
    .await;

    match user {
        Ok(Some(u)) => Json(user_to_response(&u)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "User not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

// ============================================================
// PATCH /api/v1/users/{user_id}
// ============================================================

pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserUpdateRequest>,
) -> impl IntoResponse {
    let (current_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&current_user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    // Check user exists
    let existing = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db.pool)
    .await;

    let _user = match existing {
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "User not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    };

    // Build dynamic UPDATE
    let mut set_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1i32;

    if body.name.is_some() {
        set_clauses.push(format!("name = ${}", param_idx)); param_idx += 1;
    }
    if body.email.is_some() {
        set_clauses.push(format!("email = ${}", param_idx)); param_idx += 1;
    }
    if body.role.is_some() {
        set_clauses.push(format!("role = ${}", param_idx)); param_idx += 1;
    }
    if body.is_active.is_some() {
        set_clauses.push(format!("is_active = ${}", param_idx)); param_idx += 1;
    }
    if body.quota_balance.is_some() {
        set_clauses.push(format!("quota_balance = ${}", param_idx)); param_idx += 1;
    }
    if body.allowed_models.is_some() {
        set_clauses.push(format!("allowed_models = ${}::jsonb", param_idx)); param_idx += 1;
    }

    // Add updated_at
    set_clauses.push(format!("updated_at = ${}", param_idx)); param_idx += 1;

    if set_clauses.is_empty() {
        // Nothing to update, return user as-is
        return Json(user_to_response(&_user)).into_response();
    }

    let set_sql = set_clauses.join(", ");
    let query_sql = format!("UPDATE users SET {} WHERE id = ${}", set_sql, param_idx);

    let mut query = sqlx::query(&query_sql);
    let mut p = 1i32;

    if let Some(ref name) = body.name {
        query = query.bind(name); p += 1;
    }
    if let Some(ref email) = body.email {
        query = query.bind(email); p += 1;
    }
    if let Some(ref role) = body.role {
        query = query.bind(role); p += 1;
    }
    if let Some(is_active) = body.is_active {
        query = query.bind(is_active); p += 1;
    }
    if let Some(ref quota) = body.quota_balance {
        // Convert rust_decimal::Decimal to f64 for sqlx binding
        query = query.bind(quota.to_f64().unwrap_or(0.0)); p += 1;
    }
    if let Some(ref models) = body.allowed_models {
        let json_val = serde_json::to_value(models).unwrap_or(serde_json::Value::Null);
        query = query.bind(json_val); p += 1;
    }

    query = query.bind(chrono::Utc::now()); p += 1;
    query = query.bind(user_id);

    if let Err(e) = query.execute(&state.db.pool).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response();
    }

    // Fetch updated user
    let updated = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.db.pool)
    .await;

    match updated {
        Ok(u) => Json(user_to_response(&u)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

// ============================================================
// DELETE /api/v1/users/{user_id}  (deactivate, super_admin only)
// ============================================================

pub async fn deactivate_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let (current_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&current_user, &["super_admin"]) {
        return (status, json).into_response();
    }

    let result = sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&state.db.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let user = sqlx::query_as::<_, User>(
                "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
                 title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
                 allowed_models \
                 FROM users WHERE id = $1"
            )
            .bind(user_id)
            .fetch_one(&state.db.pool)
            .await;

            match user {
                Ok(u) => Json(user_to_response(&u)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
            }
        }
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "User not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}
