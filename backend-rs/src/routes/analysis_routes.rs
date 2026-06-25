use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::validate_api_token;
use crate::models::CallContent;
use crate::routes::gateway::AppState;

/// GET /api/v1/analysis/alerts
pub async fn alerts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<AlertParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * page_size;
    let severity = params.severity.as_deref().unwrap_or("");

    let items = if severity.is_empty() {
        sqlx::query(
            r#"SELECT cm.id, cm.call_content_id, cm.mask_type, cm.mask_pattern,
                      cm.match_count, cm.severity, cm.created_at,
                      cc.model, cc.provider, cc.user_id
               FROM content_masks cm
               LEFT JOIN call_contents cc ON cc.id = cm.call_content_id
               ORDER BY cm.created_at DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db.pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query(
            r#"SELECT cm.id, cm.call_content_id, cm.mask_type, cm.mask_pattern,
                      cm.match_count, cm.severity, cm.created_at,
                      cc.model, cc.provider, cc.user_id
               FROM content_masks cm
               LEFT JOIN call_contents cc ON cc.id = cm.call_content_id
               WHERE cm.severity = $1
               ORDER BY cm.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(severity)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db.pool)
        .await
        .unwrap_or_default()
    };

    let total: i64 = if severity.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*) FROM content_masks")
            .fetch_one(&state.db.pool)
            .await
            .unwrap_or(0)
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM content_masks WHERE severity = $1")
            .bind(severity)
            .fetch_one(&state.db.pool)
            .await
            .unwrap_or(0)
    };

    let alerts_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i32, _>("id"),
                "call_content_id": r.get::<Uuid, _>("call_content_id").to_string(),
                "mask_type": r.get::<String, _>("mask_type"),
                "severity": r.get::<String, _>("severity"),
                "match_count": r.get::<i32, _>("match_count"),
                "model": r.get::<Option<String>, _>("model"),
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            })
        })
        .collect();

    Json(serde_json::json!({
        "items": alerts_json,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
    .into_response()
}

const ANALYSIS_CSV_HEADER: &str = "date,user_id,user_name,model,provider,calls,input_tokens,output_tokens,cost,avg_latency_ms,errors";

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub id: String,
    pub user_id: String,
    pub model: String,
    pub provider: String,
    pub request_preview: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub items: Vec<SearchResultItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// POST /api/v1/analysis/search
pub async fn search_content(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * page_size;

    // Simple ILIKE search on request_content
    let search_term = format!("%{}%", params.q.replace('\'', "''"));

    let count_sql = "SELECT COUNT(*) FROM call_contents WHERE request_content::text ILIKE $1";
    let data_sql = "SELECT id, user_id, token_id, request_id, model, provider, \
         request_content, response_content, file_metadata, \
         input_tokens, output_tokens, latency_ms, is_stream, \
         ip_address, created_at, expires_at \
         FROM call_contents WHERE request_content::text ILIKE $1 \
         ORDER BY created_at DESC LIMIT $2 OFFSET $3";

    let total: i64 = sqlx::query(count_sql)
        .bind(&search_term)
        .fetch_one(&state.db.pool)
        .await
        .map(|r| r.get::<i64, _>(0))
        .unwrap_or(0);

    let items: Vec<CallContent> = sqlx::query_as::<_, CallContent>(data_sql)
        .bind(&search_term)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db.pool)
        .await
        .unwrap_or_default();

    let results: Vec<SearchResultItem> = items
        .into_iter()
        .map(|c| {
            let preview = c
                .request_content
                .0
                .get("messages")
                .and_then(|m| m.as_array())
                .and_then(|arr| arr.first())
                .and_then(|msg| msg.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();

            SearchResultItem {
                id: c.id.to_string(),
                user_id: c.user_id.to_string(),
                model: c.model,
                provider: c.provider,
                request_preview: if preview.len() >= 200 {
                    format!("{}...", preview)
                } else {
                    preview
                },
                created_at: c
                    .created_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Json(SearchResponse {
        items: results,
        total,
        page,
        page_size,
    })
    .into_response()
}

/// GET /api/v1/analysis/dashboard
pub async fn dashboard(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let total_calls: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_calls), 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let total_input: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_input_tokens), 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let total_output: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_output_tokens), 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let total_cost: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_cost)::float8, 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0.0);

    let avg_latency: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(avg_latency_ms)::float8, 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0.0);

    let total_errors: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(error_count), 0) FROM daily_usage_stats"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let active_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM daily_usage_stats WHERE stat_date >= CURRENT_DATE - 7"
    )
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let error_rate = if total_calls > 0 {
        (total_errors as f64 / total_calls as f64 * 100.0 * 100.0).round() / 100.0
    } else {
        0.0
    };

    Json(serde_json::json!({
        "total_calls": total_calls,
        "total_input_tokens": total_input,
        "total_output_tokens": total_output,
        "total_cost": (total_cost * 100.0).round() / 100.0,
        "avg_latency_ms": (avg_latency * 100.0).round() / 100.0,
        "error_rate": error_rate,
        "active_users_7d": active_users,
    }))
    .into_response()
}

/// GET /api/v1/analysis/trends
pub async fn trends(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<TrendParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let days = params.days.unwrap_or(30).max(1).min(365);

    let rows = sqlx::query(
        r#"SELECT stat_date,
                  COALESCE(SUM(total_calls), 0) as calls,
                  COALESCE(SUM(total_input_tokens), 0) as input_tokens,
                  COALESCE(SUM(total_output_tokens), 0) as output_tokens,
                  COALESCE(SUM(total_cost)::float8, 0) as cost,
                  COALESCE(AVG(avg_latency_ms)::float8, 0) as avg_latency
           FROM daily_usage_stats
           WHERE stat_date >= CURRENT_DATE - $1::int
           GROUP BY stat_date
           ORDER BY stat_date"#,
    )
    .bind(days)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let date: chrono::NaiveDate = r.get("stat_date");
            serde_json::json!({
                "date": date.format("%Y-%m-%d").to_string(),
                "calls": r.get::<i64, _>("calls"),
                "input_tokens": r.get::<i64, _>("input_tokens"),
                "output_tokens": r.get::<i64, _>("output_tokens"),
                "cost": r.get::<f64, _>("cost"),
                "avg_latency_ms": r.get::<f64, _>("avg_latency"),
            })
        })
        .collect();

    Json(serde_json::json!({ "items": items })).into_response()
}

/// GET /api/v1/analysis/top-users
pub async fn top_users(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<TrendParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let days = params.days.unwrap_or(30).max(1).min(365);

    let rows = sqlx::query(
        r#"SELECT dus.user_id, u.name as user_name,
                  COALESCE(SUM(dus.total_calls), 0) as calls,
                  COALESCE(SUM(dus.total_input_tokens + dus.total_output_tokens), 0) as total_tokens,
                  COALESCE(SUM(dus.total_cost)::float8, 0) as cost
           FROM daily_usage_stats dus
           LEFT JOIN users u ON u.id = dus.user_id
           WHERE dus.stat_date >= CURRENT_DATE - $1::int
           GROUP BY dus.user_id, u.name
           ORDER BY cost DESC
           LIMIT 10"#,
    )
    .bind(days)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "user_id": r.get::<uuid::Uuid, _>("user_id").to_string(),
                "user_name": r.get::<Option<String>, _>("user_name").unwrap_or_default(),
                "calls": r.get::<i64, _>("calls"),
                "total_tokens": r.get::<i64, _>("total_tokens"),
                "cost": r.get::<f64, _>("cost"),
            })
        })
        .collect();

    Json(serde_json::json!({ "items": items })).into_response()
}

/// GET /api/v1/analysis/top-models
pub async fn top_models(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<TrendParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let days = params.days.unwrap_or(30).max(1).min(365);

    let rows = sqlx::query(
        r#"SELECT model,
                  COALESCE(SUM(total_calls), 0) as calls,
                  COALESCE(SUM(total_input_tokens + total_output_tokens), 0) as total_tokens,
                  COALESCE(SUM(total_cost)::float8, 0) as cost,
                  COALESCE(AVG(avg_latency_ms)::float8, 0) as avg_latency
           FROM daily_usage_stats
           WHERE stat_date >= CURRENT_DATE - $1::int
           GROUP BY model
           ORDER BY cost DESC
           LIMIT 10"#,
    )
    .bind(days)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "model": r.get::<String, _>("model"),
                "calls": r.get::<i64, _>("calls"),
                "total_tokens": r.get::<i64, _>("total_tokens"),
                "cost": r.get::<f64, _>("cost"),
                "avg_latency_ms": r.get::<f64, _>("avg_latency"),
            })
        })
        .collect();

    Json(serde_json::json!({ "items": items })).into_response()
}

/// GET /api/v1/analysis/export
pub async fn export(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<ExportParams>,
) -> impl IntoResponse {
    let token = match extract_token_from_analysis(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(e),
    };

    let month = params.month.unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m").to_string()
    });

    let rows = sqlx::query(
        r#"SELECT dus.stat_date, dus.user_id, u.name as user_name,
                  dus.model, dus.provider,
                  dus.total_calls, dus.total_input_tokens, dus.total_output_tokens,
                  dus.total_cost, dus.avg_latency_ms, dus.error_count
           FROM daily_usage_stats dus
           LEFT JOIN users u ON u.id = dus.user_id
           WHERE to_char(dus.stat_date, 'YYYY-MM') = $1
           ORDER BY dus.stat_date, dus.user_id"#,
    )
    .bind(&month)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let csv_lines: Vec<String> = rows
        .iter()
        .map(|r| {
            let date: chrono::NaiveDate = r.get("stat_date");
            format!(
                "{},{},{},{},{},{},{},{},{:.4},{:.2},{}",
                date.format("%Y-%m-%d"),
                r.get::<uuid::Uuid, _>("user_id"),
                r.get::<Option<String>, _>("user_name").unwrap_or_default(),
                r.get::<String, _>("model"),
                r.get::<String, _>("provider"),
                r.get::<i64, _>("total_calls"),
                r.get::<i64, _>("total_input_tokens"),
                r.get::<i64, _>("total_output_tokens"),
                r.get::<f64, _>("total_cost"),
                r.get::<f64, _>("avg_latency_ms"),
                r.get::<i32, _>("error_count"),
            )
        })
        .collect();

    let mut csv = String::from(ANALYSIS_CSV_HEADER);
    csv.push('\n');
    csv.push_str(&csv_lines.join("\n"));

    (
        [(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
         (axum::http::header::CONTENT_DISPOSITION, &format!("attachment; filename=\"analysis_{}.csv\"", month))],
        csv,
    ).into_response()
}

#[derive(Debug, Deserialize)]
pub struct TrendParams {
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AlertParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub severity: Option<String>,
}

fn extract_token_from_analysis(headers: &axum::http::HeaderMap) -> Result<String, String> {
    let auth = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Not authenticated".to_string())?;

    if let Some(token) = auth.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err("Invalid Authorization header format".to_string())
    }
}

fn error_response(detail: impl Into<String>) -> axum::response::Response {
    let body = serde_json::json!({ "error": { "message": detail.into() } });
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
