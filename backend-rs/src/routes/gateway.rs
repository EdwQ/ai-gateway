use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{sse::Sse, IntoResponse, Response, Json},
};
use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::Instant;
use uuid::Uuid;

use crate::auth::validate_api_token;
use crate::config::AppConfig;
use crate::db::DbPool;
use crate::models::{
    ChatCompletionRequest, Message, ModelInfo, ModelListResponse, User,
};
use crate::proxy::{
    check_user_allowed, find_provider, proxy_non_stream, record_usage,
    resolve_alias, KeyManager,
};
use crate::rate_limit;
use crate::redis::RedisPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: RedisPool,
    pub config: Arc<AppConfig>,
    pub key_manager: Arc<KeyManager>,
    pub http_client: Arc<reqwest::Client>,
}

/// Extract API token from Authorization header
fn extract_token_from_map(headers: &HeaderMap) -> Result<String, String> {
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

/// POST /v1/chat/completions
#[axum::debug_handler]
pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let request_id = Uuid::new_v4();

    // 1. Extract and validate API token
    let token = match extract_token_from_map(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    let user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    // 2. Rate limit check (by token prefix)
    let token_prefix = &token[..20.min(token.len())];
    if let Err(e) = rate_limit::check_rate_limit(
        &state.redis,
        &format!("user:{}", token_prefix),
        state.config.rate_limit_user_qps,
    ).await {
        return error_response(StatusCode::TOO_MANY_REQUESTS, &e);
    }

    // 3. Check user's allowed models
    if let Err(e) = check_user_allowed(&user, &body.model).await {
        return error_response(StatusCode::FORBIDDEN, &e);
    }

    // 4. Resolve alias -> real model
    let real_model = match resolve_alias(&state.db.pool, &body.model).await {
        Ok(m) => m,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    };

    // 5. Find provider
    let provider = match find_provider(&state.db.pool, &real_model).await {
        Ok(Some(p)) => p,
        Ok(None) => return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("No active provider found for model: {}", real_model),
        ),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    };

    // 6. Get API key
    let (api_key, _provider) = match state.key_manager.get_next_key(
        &state.db.pool,
        provider.id,
        &state.config.encryption_key,
    ).await {
        Ok(k) => k,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
    };

    // 7. Proxy the request
    let messages: Vec<Message> = body.messages.iter().map(|m| Message {
        role: m.role.clone(),
        content: m.content.clone(),
    }).collect();

    if body.stream {
        return handle_stream(
            &state, &user, &body, &real_model, &provider.name,
            &provider.base_url, &api_key, &messages, &request_id, start,
        ).await;
    }

    // Non-streaming
    let (data, status_code) = match proxy_non_stream(
        &state.http_client,
        &provider.base_url,
        &api_key,
        &real_model,
        &messages,
        body.temperature,
        body.max_tokens,
        body.top_p,
        body.frequency_penalty,
        body.presence_penalty,
        body.stop.clone(),
    ).await {
        Ok(d) => d,
        Err(e) => {
            // Record failure
            let duration = start.elapsed().as_millis() as i32;
            let _ = record_usage(
                &state.db.pool, user.id, &body.model, &provider.name,
                0, 0, 0, duration, false, 502,
                Some(&e), &request_id.to_string(), None, false,
            ).await;
            return error_response(StatusCode::BAD_GATEWAY, &e);
        }
    };

    let duration = start.elapsed().as_millis() as i32;
    let usage = &data["usage"];
    let prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let completion_tokens = usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let total_tokens = usage.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

    // Record usage
    let _ = record_usage(
        &state.db.pool, user.id, &body.model, &provider.name,
        prompt_tokens, completion_tokens, total_tokens, duration,
        true, status_code, None, &request_id.to_string(), None, false,
    ).await;

    // Reset provider fail count on success
    let _ = sqlx::query("UPDATE providers SET health_status = 'healthy' WHERE id = $1")
        .bind(provider.id)
        .execute(&state.db.pool)
        .await;

    Json(data).into_response()
}

/// Handle streaming (SSE) response
async fn handle_stream(
    state: &AppState,
    user: &User,
    body: &ChatCompletionRequest,
    real_model: &str,
    provider_name: &str,
    base_url: &str,
    api_key: &str,
    messages: &[Message],
    request_id: &Uuid,
    start: Instant,
) -> Response {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let mut req_headers = HeaderMap::new();
    req_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );
    req_headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let mut json_body = serde_json::json!({
        "model": real_model,
        "messages": messages,
        "stream": true,
    });

    if let Some(v) = body.temperature { json_body["temperature"] = serde_json::json!(v); }
    if let Some(v) = body.max_tokens { json_body["max_tokens"] = serde_json::json!(v); }
    if let Some(v) = body.top_p { json_body["top_p"] = serde_json::json!(v); }
    if let Some(v) = body.frequency_penalty { json_body["frequency_penalty"] = serde_json::json!(v); }
    if let Some(v) = body.presence_penalty { json_body["presence_penalty"] = serde_json::json!(v); }
    if let Some(ref v) = body.stop { json_body["stop"] = serde_json::json!(v); }

    let resp = match state.http_client
        .post(&url)
        .headers(req_headers)
        .json(&json_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &format!("Request failed: {}", e)),
    };

    if !resp.status().is_success() {
        let status_code = resp.status().as_u16() as i32;
        let text = resp.text().await.unwrap_or_default();
        let duration = start.elapsed().as_millis() as i32;
        let _ = record_usage(
            &state.db.pool, user.id, &body.model, provider_name,
            0, 0, 0, duration, true, status_code,
            Some(&text[..text.len().min(500)]), &request_id.to_string(), None, true,
        ).await;
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Provider returned {}: {}", status_code, &text[..text.len().min(500)]),
        );
    }

    // Create SSE stream that forwards upstream SSE chunks
    let user_id = user.id;
    let model = body.model.clone();
    let provider_name = provider_name.to_string();
    let pool = state.db.pool.clone();
    let request_id_str = request_id.to_string();
    let start_time = start;

    // Stream upstream bytes as SSE events
    let sse_stream = resp.bytes_stream().map(|chunk| {
        chunk.map(|bytes| {
            // Convert bytes to string for SSE event data
            let text = String::from_utf8_lossy(&bytes);
            axum::response::sse::Event::default().data(text)
        }).map_err(|e| {
            tracing::warn!("Stream error: {}", e);
            e
        })
    });

    // Spawn background task to record final usage after stream completes
    let pool_bg = pool.clone();
    let uid_bg = user_id;
    let model_bg = model.clone();
    let prov_bg = provider_name.clone();
    let rid_bg = request_id_str.clone();
    let start_bg = start_time;

    tokio::spawn(async move {
        let duration = start_bg.elapsed().as_millis() as i32;
        let _ = record_usage(
            &pool_bg, uid_bg, &model_bg, &prov_bg,
            0, 0, 0, duration, true, 200, None, &rid_bg, None, true,
        ).await;
    });

    Sse::new(sse_stream).into_response()
}

/// GET /v1/models
#[axum::debug_handler]
pub async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token_from_map(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    let user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    // Admin/super_admin/finance: return all real models from providers
    let is_admin = user.role == "admin" || user.role == "super_admin" || user.role == "finance";

    if is_admin {
        let providers = sqlx::query_as::<_, crate::models::Provider>(
            "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
             priority, health_status, rate_limit_qps, created_at, updated_at \
             FROM providers WHERE is_active = true ORDER BY priority"
        )
        .fetch_all(&state.db.pool)
        .await
        .unwrap_or_default();

        let mut models = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for p in &providers {
            for m in &p.models.0 {
                if seen.insert(m.clone()) {
                    models.push(m.clone());
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        return Json(ModelListResponse {
            data: models.into_iter().map(|m| ModelInfo {
                id: m,
                created: now,
                owned_by: "system".to_string(),
            }).collect(),
        }).into_response();
    }

    // Regular user: return their allowed model aliases
    let allowed_models = match user.allowed_models {
        Some(ref a) => a.0.clone(),
        None => return Json(ModelListResponse { data: vec![] }).into_response(),
    };

    if allowed_models.is_empty() {
        return Json(ModelListResponse { data: vec![] }).into_response();
    }

    // Filter by active aliases
    let aliases = sqlx::query_as::<_, crate::models::ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases WHERE alias_name = ANY($1) AND is_active = true"
    )
    .bind(&allowed_models)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default();

    let active_aliases: std::collections::HashSet<String> = aliases
        .into_iter()
        .map(|a| a.alias_name)
        .collect();

    let models: Vec<String> = allowed_models
        .into_iter()
        .filter(|m| active_aliases.contains(m))
        .collect();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Json(ModelListResponse {
        data: models.into_iter().map(|m| ModelInfo {
            id: m,
            created: now,
            owned_by: "system".to_string(),
        }).collect(),
    }).into_response()
}

/// POST /v1/embeddings
#[axum::debug_handler]
pub async fn embeddings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let token = match extract_token_from_map(&headers) {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    let _user = match validate_api_token(&state.db.pool, &token).await {
        Ok(u) => u,
        Err(e) => return error_response(StatusCode::UNAUTHORIZED, &e),
    };

    let model = body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-ada-002");

    let provider = match find_provider(&state.db.pool, model).await {
        Ok(Some(p)) => p,
        Ok(None) => return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("No active provider found for model: {}", model),
        ),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    };

    let (api_key, _) = match state.key_manager.get_next_key(
        &state.db.pool,
        provider.id,
        &state.config.encryption_key,
    ).await {
        Ok(k) => k,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &e),
    };

    let url = format!("{}/v1/embeddings", provider.base_url.trim_end_matches('/'));
    let mut req_headers = HeaderMap::new();
    req_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );
    req_headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let resp = match state.http_client
        .post(&url)
        .headers(req_headers)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, &format!("Request failed: {}", e)),
    };

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Provider error: {}", &text[..text.len().min(500)]),
        );
    }

    let data = match resp.json::<serde_json::Value>().await {
        Ok(d) => d,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("JSON error: {}", e)),
    };

    Json(data).into_response()
}

fn error_response(status: StatusCode, detail: &str) -> Response {
    let body = serde_json::json!({
        "error": {
            "message": detail,
            "type": status.as_str().to_string(),
        }
    });
    (status, Json(body)).into_response()
}
