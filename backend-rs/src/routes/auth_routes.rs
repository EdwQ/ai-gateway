//! Authentication routes: DingTalk OAuth, JWT management, dev login.
//!
//! Ported from `backend/app/api/v1/auth.py`

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Redirect},
};
use chrono::Utc;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::dingtalk::{DingTalkClient, DingTalkUser};
use crate::models::{User, UserInfo, sqlx_decimal_to_rust, Department, DingTalkCallbackRequest, RefreshTokenRequest};
use crate::routes::gateway::AppState;
use crate::security;

/// Shared DingTalk client (lazily initialized)
static DT_CLIENT: std::sync::OnceLock<DingTalkClient> = std::sync::OnceLock::new();

fn get_dingtalk_client() -> &'static DingTalkClient {
    DT_CLIENT.get_or_init(|| DingTalkClient::new())
}

// ============================================================
// Helper: Extract & validate JWT user
// ============================================================

/// Errors returned from JWT auth as (status_code, error_message)
pub type AuthError = (StatusCode, &'static str, String);

/// Extract current user from JWT access token in Authorization header.
///
/// Returns `(user, jti_string)` on success, or an error response.
pub async fn extract_jwt_user(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<(User, String), (StatusCode, Json<serde_json::Value>)> {
    // 1. Extract Bearer token
    let auth = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            let err = serde_json::json!({"detail": "Not authenticated"});
            (StatusCode::UNAUTHORIZED, Json(err))
        })?;

    let token = auth.strip_prefix("Bearer ").ok_or_else(|| {
        let err = serde_json::json!({"detail": "Invalid Authorization header format"});
        (StatusCode::UNAUTHORIZED, Json(err))
    })?;

    // 2. Decode JWT
    let claims = security::decode_token(token, &state.config).map_err(|e| {
        let err = serde_json::json!({"detail": e});
        (StatusCode::UNAUTHORIZED, Json(err))
    })?;

    // 3. Check token type
    if claims.token_type != "access" {
        let err = serde_json::json!({"detail": "Invalid token type"});
        return Err((StatusCode::UNAUTHORIZED, Json(err)));
    }

    // 4. Check Redis blacklist
    let blacklist_key = security::get_token_blacklist_key(&claims.jti);
    let mut redis_conn = state.redis.conn.clone();

    let blacklisted: Option<String> = redis::cmd("GET")
        .arg(&blacklist_key)
        .query_async(&mut redis_conn)
        .await
        .map_err(|_| {
            let err = serde_json::json!({"detail": "Redis error"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        })?;

    if blacklisted.is_some() {
        let err = serde_json::json!({"detail": "Token has been revoked"});
        return Err((StatusCode::UNAUTHORIZED, Json(err)));
    }

    // 5. Lookup user
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        let err = serde_json::json!({"detail": "Invalid token payload"});
        (StatusCode::UNAUTHORIZED, Json(err))
    })?;

    let user = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        let err = serde_json::json!({"detail": format!("DB error: {}", e)});
        (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
    })?
    .ok_or_else(|| {
        let err = serde_json::json!({"detail": "User not found"});
        (StatusCode::UNAUTHORIZED, Json(err))
    })?;

    if !user.is_active {
        let err = serde_json::json!({"detail": "User account is disabled"});
        return Err((StatusCode::FORBIDDEN, Json(err)));
    }

    Ok((user, claims.jti))
}

/// Build a UserInfo response struct from a DB User model
fn user_to_info(user: &User) -> UserInfo {
    UserInfo {
        id: user.id.to_string(),
        name: user.name.clone(),
        email: user.email.clone(),
        avatar: user.avatar.clone(),
        role: user.role.clone(),
        department_name: user.department_name.clone(),
        quota_balance: sqlx_decimal_to_rust(&user.quota_balance),
        quota_used: sqlx_decimal_to_rust(&user.quota_used),
    }
}

/// Build login response with access/refresh tokens + user info
fn make_login_response(user: &User, config: &AppConfig) -> Result<Json<serde_json::Value>, String> {
    let access_token = security::create_access_token(
        serde_json::json!({"sub": user.id.to_string(), "role": user.role}),
        config,
    )?;
    let refresh_token = security::create_refresh_token(
        serde_json::json!({"sub": user.id.to_string()}),
        config,
    )?;

    let user_info = user_to_info(user);

    Ok(Json(serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "bearer",
        "user": {
            "id": user_info.id,
            "name": user_info.name,
            "email": user_info.email,
            "avatar": user_info.avatar,
            "role": user_info.role,
            "department_name": user_info.department_name,
            "quota_balance": user_info.quota_balance,
            "quota_used": user_info.quota_used,
        }
    })))
}

// ============================================================
// Route handlers
// ============================================================

/// POST /api/v1/auth/dingtalk/qrcode
///
/// Generate DingTalk QR code URL for scanning login.
pub async fn dingtalk_qrcode(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let config = &*state.config;
    let base_url = config.frontend_url.trim_end_matches('/');
    let redirect_uri = format!("{}/api/v1/auth/dingtalk/callback", base_url);
    let qr_code_url = DingTalkClient::get_qrcode_url(config, &redirect_uri, "login");
    Json(serde_json::json!({"qr_code_url": qr_code_url}))
}

/// POST /api/v1/auth/dingtalk/callback
///
/// Exchange DingTalk auth_code for JWT tokens (JSON body).
pub async fn dingtalk_callback_post(
    State(state): State<AppState>,
    Json(body): Json<DingTalkCallbackRequest>,
) -> impl IntoResponse {
    let config = &*state.config;

    // 1. Get user info from DingTalk
    let dt_user = match get_dingtalk_client()
        .get_user_info(&body.auth_code, config)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"detail": e}))).into_response()
        }
    };

    // 2. Login or auto-register
    match login_or_register(&state, dt_user).await {
        Ok(resp) => resp.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"detail": e}))).into_response(),
    }
}

/// Query params for the GET dingtalk callback (DingTalk redirect)
#[derive(Debug, Deserialize)]
pub struct DingTalkCallbackQuery {
    pub code: String,
    #[serde(default)]
    pub state: String,
}

/// GET /api/v1/auth/dingtalk/callback
///
/// Handle DingTalk OAuth redirect after QR code scan.
/// DingTalk redirects here with ?code=AUTH_CODE, we process login
/// and redirect browser to frontend with JWT tokens.
pub async fn dingtalk_callback_get(
    State(state): State<AppState>,
    Query(query): Query<DingTalkCallbackQuery>,
) -> impl IntoResponse {
    let config = &*state.config;
    let frontend_base = config.frontend_url.trim_end_matches('/');

    // 1. Get user info from DingTalk
    let dt_user = match get_dingtalk_client()
        .get_user_info(&query.code, config)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            let err_msg = urlencoding::encode(&e);
            let redirect = format!("{}/login?error={}", frontend_base, err_msg);
            return Redirect::to(&redirect);
        }
    };

    // 2. Login or register
    match login_or_register(&state, dt_user).await {
        Ok(resp_json) => {
            // Extract tokens from response
            let resp_val = resp_json.0; // Json wrapper
            let access_token = resp_val["access_token"].as_str().unwrap_or("");
            let refresh_token = resp_val["refresh_token"].as_str().unwrap_or("");
            let redirect = format!(
                "{}/login?access_token={}&refresh_token={}",
                frontend_base, access_token, refresh_token
            );
            Redirect::to(&redirect)
        }
        Err(e) => {
            let err_msg = urlencoding::encode(&e);
            let redirect = format!("{}/login?error={}", frontend_base, err_msg);
            Redirect::to(&redirect)
        }
    }
}

/// POST /api/v1/auth/dev/login
///
/// Development mode: create a test admin user and return JWT tokens.
pub async fn dev_login(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let config = &*state.config;
    let test_user_id = "test-dev-user-001";

    // Check if user exists
    let existing = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE user_id = $1"
    )
    .bind(test_user_id)
    .fetch_optional(&state.db.pool)
    .await;

    match existing {
        Ok(Some(user)) => {
            // User exists — update last_login
            let _ = sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
                .bind(user.id)
                .execute(&state.db.pool)
                .await;

            match make_login_response(&user, config) {
                Ok(resp) => resp.into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": e}))).into_response(),
            }
        }
        Ok(None) => {
            // Create new dev user
            let user_id = Uuid::new_v4();
            let now = Utc::now();

            let result = sqlx::query(
                "INSERT INTO users (id, union_id, user_id, name, email, department_id, department_name, \
                 title, role, is_active, quota_balance, quota_used, last_login_at, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
            )
            .bind(user_id)
            .bind(format!("dev-{}", test_user_id))
            .bind(test_user_id)
            .bind("Development User")
            .bind("dev@localhost.local")
            .bind("1")
            .bind("研发部")
            .bind("Engineer")
            .bind("admin") // Dev user has admin role
            .bind(true) // is_active
            .bind(config.default_quota_amount) // quota_balance
            .bind(0.0_f64) // quota_used
            .bind(now)
            .bind(now)
            .execute(&state.db.pool)
            .await;

            match result {
                Ok(_) => {
                    // Fetch the newly created user
                    let user = sqlx::query_as::<_, User>(
                        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
                         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
                         allowed_models \
                         FROM users WHERE id = $1"
                    )
                    .bind(user_id)
                    .fetch_one(&state.db.pool)
                    .await
                    .unwrap();

                    match make_login_response(&user, config) {
                        Ok(resp) => resp.into_response(),
                        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": e}))).into_response(),
                    }
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
    }
}

/// POST /api/v1/auth/refresh
///
/// Refresh access token using a refresh token.
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    // 1. Decode the refresh token
    let claims = match security::decode_token(&body.refresh_token, &state.config) {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"detail": e}))).into_response()
        }
    };

    // 2. Check token type
    if claims.token_type != "refresh" {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"detail": "Invalid refresh token type"}))).into_response();
    }

    // 3. Check Redis blacklist
    let blacklist_key = security::get_token_blacklist_key(&claims.jti);
    let mut redis_conn = state.redis.conn.clone();

    let blacklisted: Option<String> = redis::cmd("GET")
        .arg(&blacklist_key)
        .query_async(&mut redis_conn)
        .await
        .unwrap_or(None);

    if blacklisted.is_some() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"detail": "Refresh token has been revoked"}))).into_response();
    }

    // 4. Issue new access token
    let new_access = match security::create_access_token(
        serde_json::json!({
            "sub": claims.sub,
            "role": claims.role.unwrap_or_default(),
        }),
        &state.config,
    ) {
        Ok(t) => t,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": e}))).into_response()
        }
    };

    Json(serde_json::json!({
        "access_token": new_access,
        "token_type": "bearer",
    }))
    .into_response()
}

/// POST /api/v1/auth/logout
///
/// Logout: blacklist the current JWT in Redis.
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth = match headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        Some(a) => a,
        None => {
            return Json(serde_json::json!({"message": "Logged out successfully"})).into_response()
        }
    };

    let token = match auth.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            return Json(serde_json::json!({"message": "Logged out successfully"})).into_response()
        }
    };

    // Decode token to get jti and exp
    let claims = match security::decode_token(token, &state.config) {
        Ok(c) => c,
        Err(_) => {
            return Json(serde_json::json!({"message": "Logged out successfully"})).into_response()
        }
    };

    // Blacklist the JTI
    let jti = claims.jti;
    let blacklist_key = security::get_token_blacklist_key(&jti);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let ttl = (claims.exp as i64) - now;
    if ttl > 0 {
        let mut redis_conn = state.redis.conn.clone();
        let _: Result<(), _> = redis::cmd("SET")
            .arg(&blacklist_key)
            .arg("1")
            .arg("EX")
            .arg(ttl)
            .query_async(&mut redis_conn)
            .await;
    }

    Json(serde_json::json!({"message": "Logged out successfully"}))
        .into_response()
}

/// GET /api/v1/auth/me
///
/// Return current user info.
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match extract_jwt_user(&headers, &state).await {
        Ok((user, _jti)) => {
            let info = user_to_info(&user);
            Json(serde_json::json!({"user": info})).into_response()
        }
        Err((status, json)) => (status, json).into_response(),
    }
}

/// POST /api/v1/auth/init
///
/// 初始化系统：创建第一个超级管理员。
/// 仅在系统中没有任何 admin / super_admin 用户时可调用。
/// 成功返回 JWT Token，后续不再可用。
pub async fn init_admin(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let config = &*state.config;

    // 检查是否已有管理员
    let admin_count: (i64,) = match sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE role IN ('admin', 'super_admin')"
    )
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "detail": format!("DB error: {}", e)
            }))).into_response()
        }
    };

    if admin_count.0 > 0 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "detail": "系统已初始化，已有管理员用户。如需重置请联系现有管理员。"
        }))).into_response();
    }

    // 创建第一个管理员
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let result = sqlx::query(
        "INSERT INTO users (id, union_id, user_id, name, email, department_id, \
         department_name, title, role, is_active, quota_balance, quota_used, \
         last_login_at, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
    )
    .bind(user_id)
    .bind(format!("init-{}", user_id))
    .bind("init-admin-user")
    .bind("System Admin")
    .bind("admin@ai-gateway.local")
    .bind("1")
    .bind("系统管理")
    .bind("Administrator")
    .bind("super_admin")
    .bind(true)
    .bind(config.default_quota_amount)
    .bind(0.0_f64)
    .bind(now)
    .bind(now)
    .execute(&state.db.pool)
    .await;

    match result {
        Ok(_) => {
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
                Ok(u) => match make_login_response(&u, config) {
                    Ok(resp) => (StatusCode::CREATED, resp).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": e}))).into_response(),
                },
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("DB error: {}", e)}))).into_response(),
            }
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to create admin: {}", e)}))).into_response()
        }
    }
}

// ============================================================
// Internal helpers
// ============================================================

/// Process DingTalk login: find or create user, return JWT response.
async fn login_or_register(
    state: &AppState,
    dt_user: DingTalkUser,
) -> Result<Json<serde_json::Value>, String> {
    let config = &*state.config;
    let union_id = &dt_user.unionid;
    let now = Utc::now();

    // Check if user exists by union_id
    let existing = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE union_id = $1"
    )
    .bind(union_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let user = match existing {
        Some(u) => {
            // Existing user
            if !u.is_active {
                return Err("User account is disabled".to_string());
            }
            // Update last_login
            let _ = sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
                .bind(now)
                .bind(u.id)
                .execute(&state.db.pool)
                .await;
            u
        }
        None => {
            // First-time user → auto-register
            // Promote to super_admin if no admin exists yet
            let admin_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE role IN ('admin', 'super_admin')"
            )
            .fetch_one(&state.db.pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

            let role = if admin_count.0 == 0 { "super_admin" } else { "employee" };

            // Get department info
            let dept_id = dt_user.dept_id_list.as_ref()
                .and_then(|list| list.first().cloned());
            let dept_name = if let Some(dept_id_str) = dept_id.as_ref() {
                get_dept_name(state, dept_id_str).await
            } else {
                None
            };

            let user_id = Uuid::new_v4();

            sqlx::query(
                "INSERT INTO users (id, union_id, user_id, name, email, avatar, department_id, \
                 department_name, title, role, is_active, quota_balance, quota_used, last_login_at, \
                 created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"
            )
            .bind(user_id)
            .bind(union_id)
            .bind(&dt_user.userid)
            .bind(&dt_user.name)
            .bind(&dt_user.email)
            .bind(&dt_user.avatar)
            .bind(&dept_id)
            .bind(&dept_name)
            .bind(&dt_user.title)
            .bind(role)
            .bind(true)
            .bind(config.default_quota_amount)
            .bind(0.0_f64)
            .bind(now)
            .bind(now)
            .execute(&state.db.pool)
            .await
            .map_err(|e| format!("Failed to create user: {}", e))?;

            // Fetch the new user
            sqlx::query_as::<_, User>(
                "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
                 title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
                 allowed_models \
                 FROM users WHERE id = $1"
            )
            .bind(user_id)
            .fetch_one(&state.db.pool)
            .await
            .map_err(|e| format!("Failed to fetch new user: {}", e))?
        }
    };

    make_login_response(&user, config)
}

/// Get department name from DingTalk (with caching in dept_name_list)
async fn get_dept_name(state: &AppState, dept_id: &str) -> Option<String> {
    // Try the departments table first
    let dept = sqlx::query_as::<_, Department>(
        "SELECT id, name, parent_id, order_num, is_active, created_at \
         FROM departments WHERE id = $1 AND is_active = true"
    )
    .bind(dept_id)
    .fetch_optional(&state.db.pool)
    .await
    .ok()
    .flatten();

    if let Some(d) = dept {
        return Some(d.name);
    }

    // Fallback: query DingTalk API
    let config = &*state.config;
    if !config.dingtalk_app_id.is_empty() {
        if let Ok(detail) = get_dingtalk_client()
            .get_department_detail(dept_id, config)
            .await
        {
            if let Some(name) = detail["name"].as_str() {
                return Some(name.to_string());
            }
        }
    }

    None
}
