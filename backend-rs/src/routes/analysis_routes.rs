use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::auth::validate_api_token;
use crate::models::CallContent;
use crate::routes::gateway::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub user_id: Option<String>,
    pub model: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
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
