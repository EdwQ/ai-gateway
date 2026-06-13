//! Statistics & reporting routes.
//!
//! Ported from `backend/app/api/v1/stats.py`

use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json},
};
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

use crate::models::{
    DashboardStats, DailyStatsItem, DailyStatsParams, DailyStatsResponse, ExportParams,
    ModelRankItem, MonthlyStatsItem, MonthlyStatsParams, MonthlyStatsResponse,
};
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
// GET /api/v1/stats/dashboard
// ============================================================

pub async fn get_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    // Total users
    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db.pool)
        .await
        .unwrap_or((0,));

    // Active users
    let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db.pool)
        .await
        .unwrap_or((0,));

    // Total tokens and cost
    let total_row: (Option<sqlx::types::BigDecimal>, Option<sqlx::types::BigDecimal>) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(cost_rmb), 0) \
         FROM usage_logs WHERE is_success = true"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or((None, None));

    let total_tokens = total_row.0
        .and_then(|d| d.to_string().parse::<i64>().ok())
        .unwrap_or(0);
    let total_cost = total_row.1
        .and_then(|d| d.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);

    // Model rank (top 10 by total tokens)
    #[derive(sqlx::FromRow)]
    struct ModelRankRow {
        model: String,
        calls: Option<i64>,
        total_tokens: Option<sqlx::types::BigDecimal>,
        cost: Option<sqlx::types::BigDecimal>,
    }

    let rank_rows: Vec<ModelRankRow> = sqlx::query_as(
        "SELECT model, COUNT(id) as calls, SUM(total_tokens) as total_tokens, SUM(cost_rmb) as cost \
         FROM usage_logs WHERE is_success = true \
         GROUP BY model ORDER BY SUM(total_tokens) DESC LIMIT 10"
    )
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let model_rank: Vec<ModelRankItem> = rank_rows
        .into_iter()
        .map(|r| ModelRankItem {
            model: r.model,
            calls: r.calls.unwrap_or(0),
            total_tokens: r.total_tokens
                .and_then(|d| d.to_string().parse::<i64>().ok())
                .unwrap_or(0),
            cost: r.cost
                .and_then(|d| d.to_string().parse::<f64>().ok())
                .unwrap_or(0.0),
        })
        .collect();

    Json(DashboardStats {
        total_users: total_users.0,
        active_users: active_users.0,
        total_tokens,
        total_cost,
        model_rank,
    })
    .into_response()
}

// ============================================================
// GET /api/v1/stats/daily?days=30
// ============================================================

pub async fn get_daily_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<DailyStatsParams>,
) -> impl IntoResponse {
    let (_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    let days = params.days.max(1).min(365);
    let since = (Utc::now() - chrono::Duration::days(days)).naive_utc();

    let mut sql = String::from(
        "SELECT DATE(created_at) as date, \
         COALESCE(SUM(total_tokens), 0) as tokens, \
         COALESCE(SUM(cost_rmb), 0) as cost, \
         COUNT(id) as requests \
         FROM usage_logs \
         WHERE created_at >= $1 AND is_success = true"
    );
    let mut param_idx = 2;

    if params.user_id.is_some() {
        sql.push_str(&format!(" AND user_id = ${}", param_idx));
        param_idx += 1;
    }
    if params.model.is_some() {
        sql.push_str(&format!(" AND model = ${}", param_idx));
        param_idx += 1;
    }

    sql.push_str(" GROUP BY DATE(created_at) ORDER BY DATE(created_at)");

    #[derive(sqlx::FromRow)]
    struct DailyRow {
        date: chrono::NaiveDate,
        tokens: Option<sqlx::types::BigDecimal>,
        cost: Option<sqlx::types::BigDecimal>,
        requests: Option<i64>,
    }

    let mut query = sqlx::query_as::<_, DailyRow>(&sql);
    query = query.bind(since);

    if let Some(ref uid) = params.user_id {
        query = query.bind(uid);
    }
    if let Some(ref model) = params.model {
        query = query.bind(model);
    }

    let rows = query.fetch_all(&state.db.pool).await.unwrap_or_default();

    let items: Vec<DailyStatsItem> = rows
        .into_iter()
        .map(|r| DailyStatsItem {
            date: r.date.to_string(),
            total_tokens: r.tokens
                .and_then(|d| d.to_string().parse::<i64>().ok())
                .unwrap_or(0),
            total_cost: r.cost
                .and_then(|d| d.to_string().parse::<f64>().ok())
                .unwrap_or(0.0),
            request_count: r.requests.unwrap_or(0),
        })
        .collect();

    Json(DailyStatsResponse { items }).into_response()
}

// ============================================================
// GET /api/v1/stats/monthly?months=6
// ============================================================

pub async fn get_monthly_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MonthlyStatsParams>,
) -> impl IntoResponse {
    let (_user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };

    let months = params.months.max(1).min(24);

    #[derive(sqlx::FromRow)]
    struct MonthlyRow {
        month: chrono::NaiveDate, // DATE_TRUNC returns date
        tokens: Option<sqlx::types::BigDecimal>,
        cost: Option<sqlx::types::BigDecimal>,
        requests: Option<i64>,
    }

    let rows: Vec<MonthlyRow> = sqlx::query_as(
        "SELECT DATE_TRUNC('month', created_at)::date as month, \
         COALESCE(SUM(total_tokens), 0) as tokens, \
         COALESCE(SUM(cost_rmb), 0) as cost, \
         COUNT(id) as requests \
         FROM usage_logs \
         WHERE created_at >= NOW() - ($1 || ' months')::interval AND is_success = true \
         GROUP BY DATE_TRUNC('month', created_at) \
         ORDER BY DATE_TRUNC('month', created_at)"
    )
    .bind(months.to_string())
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<MonthlyStatsItem> = rows
        .into_iter()
        .map(|r| MonthlyStatsItem {
            month: r.month.format("%Y-%m").to_string(),
            total_tokens: r.tokens
                .and_then(|d| d.to_string().parse::<i64>().ok())
                .unwrap_or(0),
            total_cost: r.cost
                .and_then(|d| d.to_string().parse::<f64>().ok())
                .unwrap_or(0.0),
            request_count: r.requests.unwrap_or(0),
        })
        .collect();

    Json(MonthlyStatsResponse { items }).into_response()
}

// ============================================================
// GET /api/v1/stats/export?month=2024-01
// ============================================================

pub async fn export_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> impl IntoResponse {
    let (user, _jti) = match extract_jwt_user(&headers, &state).await {
        Ok(u) => u,
        Err((status, json)) => return (status, json).into_response(),
    };
    if let Err((status, json)) = require_role(&user, &["admin", "super_admin", "finance"]) {
        return (status, json).into_response();
    }

    let month_start = format!("{}-01", params.month);

    #[derive(sqlx::FromRow)]
    struct ExportRow {
        created_at: Option<chrono::DateTime<Utc>>,
        user_id: uuid::Uuid,
        model: String,
        provider: String,
        prompt_tokens: i32,
        completion_tokens: i32,
        total_tokens: i32,
        cost_rmb: sqlx::types::BigDecimal,
        duration_ms: i32,
        is_success: bool,
        is_stream: bool,
    }

    let rows: Vec<ExportRow> = sqlx::query_as(
        "SELECT created_at, user_id, model, provider, prompt_tokens, completion_tokens, \
         total_tokens, cost_rmb, duration_ms, is_success, is_stream \
         FROM usage_logs \
         WHERE created_at >= $1::date \
           AND created_at < ($1::date + INTERVAL '1 month') \
         ORDER BY created_at"
    )
    .bind(&month_start)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    // Build CSV
    let mut csv = String::from(
        "Date,User ID,Model,Provider,Prompt Tokens,Completion Tokens,Total Tokens,Cost (RMB),Duration (ms),Success,Stream\n"
    );

    for r in &rows {
        let date = r.created_at
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let cost = r.cost_rmb.to_string();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            date, r.user_id, r.model, r.provider,
            r.prompt_tokens, r.completion_tokens, r.total_tokens,
            cost, r.duration_ms, r.is_success, r.is_stream,
        ));
    }

    let filename = format!("usage_{}.csv", params.month);
    let body = axum::body::Body::from(csv);

    let headers = [
        ("content-type", "text/csv; charset=utf-8"),
        ("content-disposition", &format!("attachment; filename=\"{}\"", filename)),
    ];

    (headers, body).into_response()
}
