use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::models::{ApiToken, User};

/// SHA256 hash of an API token (same as Python `hash_token`)
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Validate an API token (sk-company-xxx) and return the associated user.
pub async fn validate_api_token(
    pool: &PgPool,
    token: &str,
) -> Result<User, String> {
    // Must start with sk-company-
    if !token.starts_with("sk-company-") {
        return Err("Invalid API token format".to_string());
    }

    let token_hash = hash_token(token);

    // Find matching token
    let api_token = sqlx::query_as::<_, ApiToken>(
        "SELECT id, user_id, token_hash, token_prefix, name, is_active, last_used_at, created_at, updated_at \
         FROM api_tokens \
         WHERE token_hash = $1 AND is_active = true"
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?
    .ok_or_else(|| "Invalid or expired API token".to_string())?;

    // Get user
    let user = sqlx::query_as::<_, User>(
        "SELECT id, union_id, user_id, name, email, avatar, department_id, department_name, \
         title, role, is_active, quota_balance, quota_used, last_login_at, created_at, updated_at, \
         allowed_models \
         FROM users WHERE id = $1 AND is_active = true"
    )
    .bind(&api_token.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?
    .ok_or_else(|| "User not found or disabled".to_string())?;

    // Update last_used_at
    let _ = sqlx::query(
        "UPDATE api_tokens SET last_used_at = NOW() WHERE id = $1"
    )
    .bind(&api_token.id)
    .execute(pool)
    .await;

    Ok(user)
}
