use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Message, ModelAlias, Provider, ProviderKey, User,
};

/// AES-256-GCM decrypt (matching Python `decrypt_value`)
fn decrypt_value(ciphertext_b64: &str, encryption_key: &str) -> Result<String, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    let cipher_bytes = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    if cipher_bytes.len() < 12 {
        return Err("Ciphertext too short".to_string());
    }

    let nonce = &cipher_bytes[..12];
    let ciphertext = &cipher_bytes[12..];

    // Derive 32-byte key via SHA256 (matching Python)
    let mut hasher = Sha256::new();
    hasher.update(encryption_key.as_bytes());
    let key_bytes = hasher.finalize();

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decrypt error: {:?}", e))?;

    Ok(String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))?)
}

/// Model pricing: $/1M tokens (input, output) - same as Python `MODEL_PRICES`
const MODEL_PRICES: &[(&str, f64, f64)] = &[
    ("gpt-4o", 2.50, 10.00),
    ("gpt-4o-mini", 0.15, 0.60),
    ("gpt-4", 30.00, 60.00),
    ("gpt-4-turbo", 10.00, 30.00),
    ("gpt-3.5-turbo", 0.50, 1.50),
    ("claude-3-5-sonnet", 3.00, 15.00),
    ("claude-3-5-haiku", 0.80, 4.00),
    ("claude-3-opus", 15.00, 75.00),
    ("gemini-2.0-flash", 0.10, 0.40),
    ("gemini-2.0-pro", 2.00, 8.00),
    ("deepseek-chat", 0.14, 0.28),
    ("deepseek-reasoner", 0.55, 2.19),
    ("qwen-max", 1.60, 4.80),
    ("qwen-plus", 0.40, 1.20),
    ("qwen-turbo", 0.15, 0.60),
];

const USD_TO_RMB: f64 = 7.25;

pub fn calculate_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> Decimal {
    let model_lower = model.to_lowercase();

    let mut input_price = 1.0_f64;
    let mut output_price = 3.0_f64;
    for (name, input_p, output_p) in MODEL_PRICES {
        if model_lower == *name
            || model_lower.starts_with(name)
            || name.starts_with(&model_lower)
        {
            input_price = *input_p;
            output_price = *output_p;
            break;
        }
    }

    let cost_usd = (prompt_tokens as f64 / 1_000_000.0 * input_price)
        + (completion_tokens as f64 / 1_000_000.0 * output_price);
    let cost_rmb = cost_usd * USD_TO_RMB;

    let rounded = (cost_rmb * 1_000_000.0).round() / 1_000_000.0;
    Decimal::from_f64(rounded).unwrap_or(Decimal::ZERO)
}

pub fn calculate_cost_f64(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
    let cost = calculate_cost(model, prompt_tokens, completion_tokens);
    cost.to_f64().unwrap_or(0.0)
}

/// Resolve a model alias to the real model name
pub async fn resolve_alias(pool: &PgPool, model: &str) -> Result<String, String> {
    let alias = sqlx::query_as::<_, ModelAlias>(
        "SELECT id, alias_name, target_model, description, is_active, created_at, updated_at \
         FROM model_aliases WHERE alias_name = $1 AND is_active = true"
    )
    .bind(model)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    Ok(alias.map(|a| a.target_model).unwrap_or_else(|| model.to_string()))
}

/// Find best provider for the given model
pub async fn find_provider(pool: &PgPool, model: &str) -> Result<Option<Provider>, String> {
    let providers = sqlx::query_as::<_, Provider>(
        "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
         priority, health_status, rate_limit_qps, created_at, updated_at \
         FROM providers \
         WHERE is_active = true \
           AND health_status IN ('unknown', 'healthy', 'degraded') \
         ORDER BY priority"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    for provider in &providers {
        let models = &provider.models.0;
        if models.contains(&model.to_string()) {
            return Ok(Some(provider.clone()));
        }
        // Wildcard matching
        for pm in models {
            if pm.ends_with('*') && model.starts_with(&pm[..pm.len() - 1]) {
                return Ok(Some(provider.clone()));
            }
            if model.starts_with(pm) {
                return Ok(Some(provider.clone()));
            }
        }
    }

    // Return first active provider if no specific match
    Ok(providers.into_iter().next())
}

/// Get next available API key using round-robin with failover
#[derive(Clone)]
pub struct KeyManager {
    index: Arc<std::sync::Mutex<std::collections::HashMap<String, usize>>>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            index: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn get_next_key(
        &self,
        pool: &PgPool,
        provider_id: Uuid,
        encryption_key: &str,
    ) -> Result<(String, Provider), String> {
        // Try getting ProviderKeys first
        let keys = sqlx::query_as::<_, ProviderKey>(
            "SELECT pk.id, pk.provider_id, pk.key_encrypted, pk.is_active, pk.weight, \
             pk.fail_count, pk.max_fail_count, pk.last_success_at, pk.created_at \
             FROM provider_keys pk \
             JOIN providers p ON pk.provider_id = p.id \
             WHERE pk.provider_id = $1 AND pk.is_active = true \
             ORDER BY pk.weight"
        )
        .bind(provider_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

        if !keys.is_empty() {
            // Filter out failed keys
            let available: Vec<&ProviderKey> = keys
                .iter()
                .filter(|k| k.fail_count < k.max_fail_count)
                .collect();

            let available = if available.is_empty() {
                // Reset fail counts if all are down
                for k in &keys {
                    let _ = sqlx::query("UPDATE provider_keys SET fail_count = 0 WHERE id = $1")
                        .bind(k.id)
                        .execute(pool)
                        .await;
                }
                keys.iter().collect()
            } else {
                available
            };

            // Round-robin (drop lock before await)
            let idx;
            {
                let mut map = self.index.lock().map_err(|e| format!("Lock error: {}", e))?;
                idx = map.get(&provider_id.to_string()).copied().unwrap_or(0) % available.len();
                map.insert(provider_id.to_string(), idx + 1);
            } // MutexGuard dropped here

            let selected = available[idx];
            let api_key = decrypt_value(&selected.key_encrypted, encryption_key)?;

            // Get provider info
            let provider = sqlx::query_as::<_, Provider>(
                "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
                 priority, health_status, rate_limit_qps, created_at, updated_at \
                 FROM providers WHERE id = $1"
            )
            .bind(provider_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

            return Ok((api_key, provider));
        }

        // Fallback: use provider.api_key_encrypted directly
        let provider = sqlx::query_as::<_, Provider>(
            "SELECT id, name, display_name, base_url, api_key_encrypted, models, is_active, \
             priority, health_status, rate_limit_qps, created_at, updated_at \
             FROM providers WHERE id = $1"
        )
        .bind(provider_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

        let api_key = decrypt_value(&provider.api_key_encrypted, encryption_key)?;
        Ok((api_key, provider))
    }
}

/// Record usage log and update user quota
pub async fn record_usage(
    pool: &PgPool,
    user_id: Uuid,
    model: &str,
    provider_name: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    duration_ms: i32,
    is_success: bool,
    status_code: i32,
    error_message: Option<&str>,
    request_id: &str,
    cost_f64: Option<f64>,
    is_stream: bool,
) -> Result<(), String> {
    let cost = cost_f64.unwrap_or_else(|| calculate_cost_f64(model, prompt_tokens, completion_tokens));

    // Insert usage log
    sqlx::query(
        "INSERT INTO usage_logs (user_id, model, provider, prompt_tokens, completion_tokens, \
         total_tokens, cost_rmb, duration_ms, is_success, status_code, error_message, request_id, \
         is_stream, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())"
    )
    .bind(user_id)
    .bind(model)
    .bind(provider_name)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(cost)
    .bind(duration_ms)
    .bind(is_success)
    .bind(status_code)
    .bind(error_message)
    .bind(request_id)
    .bind(is_stream)
    .execute(pool)
    .await
    .map_err(|e| format!("DB insert error: {}", e))?;

    // Update user quota
    if is_success {
        sqlx::query(
            "UPDATE users SET quota_used = quota_used + $1, quota_balance = quota_balance - $1 \
             WHERE id = $2"
        )
        .bind(cost)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB update error: {}", e))?;
    }

    Ok(())
}

/// Check if user is allowed to use this model
pub async fn check_user_allowed(user: &User, model: &str) -> Result<(), String> {
    if user.role == "admin" || user.role == "super_admin" || user.role == "finance" {
        return Ok(());
    }

    if let Some(ref allowed) = user.allowed_models {
        if !allowed.0.contains(&model.to_string()) {
            return Err(format!(
                "Model '{}' is not in your allowed models list. Allowed: {}",
                model,
                allowed.0.join(", ")
            ));
        }
    }

    Ok(())
}

/// Forward non-streaming request to upstream provider
pub async fn proxy_non_stream(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[Message],
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<Vec<String>>,
) -> Result<(serde_json::Value, i32), String> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| format!("Header error: {}", e))?,
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });

    if let Some(v) = temperature { body["temperature"] = serde_json::json!(v); }
    if let Some(v) = max_tokens { body["max_tokens"] = serde_json::json!(v); }
    if let Some(v) = top_p { body["top_p"] = serde_json::json!(v); }
    if let Some(v) = frequency_penalty { body["frequency_penalty"] = serde_json::json!(v); }
    if let Some(v) = presence_penalty { body["presence_penalty"] = serde_json::json!(v); }
    if let Some(v) = stop { body["stop"] = serde_json::json!(v); }

    let resp = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = resp.status();
    let status_code = status.as_u16() as i32;

    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let truncated = &text[..text.len().min(500)];
        return Err(format!("Provider returned {}: {}", status_code, truncated));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    Ok((data, status_code))
}
