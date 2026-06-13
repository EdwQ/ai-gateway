//! Security utilities: JWT create/decode, API token generation, AES encryption.
//!
//! Ported from `backend/app/core/security.py`

use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AppConfig;

// ============================================================
// JWT
// ============================================================

/// Claims in our JWT tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user id)
    pub sub: String,
    /// JWT ID (for blacklisting)
    pub jti: String,
    /// Token type: "access" or "refresh"
    #[serde(rename = "type")]
    pub token_type: String,
    /// Expiration (unix timestamp)
    pub exp: usize,
    /// Issued at (unix timestamp)
    pub iat: usize,
    /// User role (only for access tokens)
    pub role: Option<String>,
}

/// Create a JWT access token (30 min default)
pub fn create_access_token(
    data: serde_json::Value,
    config: &AppConfig,
) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs() as usize;

    let expires_in = config.jwt_access_token_expire_minutes as usize * 60;

    let sub = data["sub"].as_str().unwrap_or_default().to_string();
    let role = data["role"].as_str().map(|s| s.to_string());

    let claims = Claims {
        sub,
        jti: Uuid::new_v4().to_string(),
        token_type: "access".to_string(),
        exp: now + expires_in,
        iat: now,
        role,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret_key.as_bytes()),
    )
    .map_err(|e| format!("JWT encode error: {}", e))
}

/// Create a JWT refresh token (7 days default)
pub fn create_refresh_token(
    data: serde_json::Value,
    config: &AppConfig,
) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs() as usize;

    let expires_in = config.jwt_refresh_token_expire_days as usize * 86400;

    let sub = data["sub"].as_str().unwrap_or_default().to_string();
    let role = data["role"].as_str().map(|s| s.to_string());

    let claims = Claims {
        sub,
        jti: Uuid::new_v4().to_string(),
        token_type: "refresh".to_string(),
        exp: now + expires_in,
        iat: now,
        role,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret_key.as_bytes()),
    )
    .map_err(|e| format!("JWT encode error: {}", e))
}

/// Decode and validate a JWT token. Returns the claims.
pub fn decode_token(token: &str, config: &AppConfig) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret_key.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    Ok(token_data.claims)
}

/// Redis key for JWT blacklist entry
pub fn get_token_blacklist_key(jti: &str) -> String {
    format!("jwt_blacklist:{}", jti)
}

// ============================================================
// API Token generation & hashing
// ============================================================

/// Generate an API token in the format `sk-company-{40 hex chars}`
pub fn generate_api_token() -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 20] = rng.gen();
    let hex_str = hex::encode(random_bytes);
    format!("sk-company-{}", hex_str)
}

/// SHA256 hash of a token
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a token against its stored hash
pub fn verify_token_hash(token: &str, token_hash: &str) -> bool {
    hash_token(token) == token_hash
}

// ============================================================
// AES-256-GCM encryption/decryption (matching Python implementation)
// ============================================================

/// AES-256-GCM encrypt. Returns base64(nonce + ciphertext).
/// Uses SHA256 to derive a 32-byte key from the encryption_key string.
pub fn encrypt_value(plaintext: &str, encryption_key: &str) -> Result<String, String> {
    // Derive 32-byte key via SHA256
    let mut hasher = Sha256::new();
    hasher.update(encryption_key.as_bytes());
    let key_bytes = hasher.finalize();

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encrypt error: {:?}", e))?;

    // Concatenate nonce + ciphertext and base64 encode
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&result))
}

/// AES-256-GCM decrypt. Input is base64(nonce + ciphertext).
pub fn decrypt_value(ciphertext_b64: &str, encryption_key: &str) -> Result<String, String> {
    let cipher_bytes = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    if cipher_bytes.len() < 12 {
        return Err("Ciphertext too short".to_string());
    }

    let nonce = &cipher_bytes[..12];
    let ciphertext = &cipher_bytes[12..];

    // Derive 32-byte key via SHA256
    let mut hasher = Sha256::new();
    hasher.update(encryption_key.as_bytes());
    let key_bytes = hasher.finalize();

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decrypt error: {:?}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))
}

// ============================================================
// Utility
// ============================================================

/// Mask middle of string: show first N and last M chars
pub fn mask_sensitive(value: &str, show_prefix: usize, show_suffix: usize) -> String {
    let len = value.len();
    if len <= show_prefix + show_suffix + 4 {
        return value.to_string();
    }
    let prefix = &value[..show_prefix];
    let suffix = &value[len - show_suffix..];
    let stars = "*".repeat(len - show_prefix - show_suffix);
    format!("{}{}{}", prefix, stars, suffix)
}
