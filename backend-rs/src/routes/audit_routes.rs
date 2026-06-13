//! Audit log routes.
//!
//! Ported from `backend/app/api/v1/audit.py`

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};

use crate::models::{AuditLog, AuditLogListResponse, AuditLogQueryParams, AuditLogResponse};
use crate::routes::auth_routes::extract_jwt_user;
use crate::routes::gateway::AppState;

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
// GET /api/v1/audit/logs
// ============================================================

pub async fn list_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AuditLogQueryParams>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin"]) {
        return (status, json).into_response();
    }

    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Build WHERE clause
    let mut conditions = Vec::new();
    let mut param_idx = 1i32;

    if let Some(ref action) = params.action {
        conditions.push(format!("action = ${}", param_idx)); param_idx += 1;
    }
    if let Some(ref uid) = params.user_id {
        conditions.push(format!("user_id = ${}::uuid", param_idx)); param_idx += 1;
    }
    if let Some(ref rt) = params.resource_type {
        conditions.push(format!("resource_type = ${}", param_idx)); param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);

    let mut p = 1i32;
    if let Some(ref action) = params.action { count_query = count_query.bind(action); p += 1; }
    if let Some(ref uid) = params.user_id { count_query = count_query.bind(uid); p += 1; }
    if let Some(ref rt) = params.resource_type { count_query = count_query.bind(rt); p += 1; }

    let total = count_query.fetch_one(&state.db.pool).await.unwrap_or((0,)).0;

    // Data
    let limit_idx = param_idx;
    let offset_idx = param_idx + 1;
    let data_sql = format!(
        "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, \
         user_agent, created_at \
         FROM audit_logs {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, limit_idx, offset_idx
    );

    let mut data_query = sqlx::query_as::<_, AuditLog>(&data_sql);

    if let Some(ref action) = params.action { data_query = data_query.bind(action); }
    if let Some(ref uid) = params.user_id { data_query = data_query.bind(uid); }
    if let Some(ref rt) = params.resource_type { data_query = data_query.bind(rt); }

    data_query = data_query.bind(page_size as i64).bind(offset as i64);

    let logs = data_query.fetch_all(&state.db.pool).await.unwrap_or_default();

    let items: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(|l| AuditLogResponse {
            id: l.id,
            user_id: l.user_id.to_string(),
            action: l.action,
            resource_type: l.resource_type,
            resource_id: l.resource_id,
            details: l.details.map(|d| d.0),
            ip_address: l.ip_address,
            user_agent: l.user_agent,
            created_at: l.created_at,
        })
        .collect();

    Json(AuditLogListResponse {
        items,
        total,
        page,
        page_size,
    })
    .into_response()
}
