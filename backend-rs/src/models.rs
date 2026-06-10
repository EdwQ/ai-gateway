use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::BigDecimal;
use uuid::Uuid;

/// User model matching Python `app/models/user.py`
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub union_id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub title: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub quota_balance: BigDecimal,
    pub quota_used: BigDecimal,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub allowed_models: Option<sqlx::types::Json<Vec<String>>>,
}

/// Provider model matching Python `app/models/provider.py`
#[derive(Debug, Clone, FromRow)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub models: sqlx::types::Json<Vec<String>>,
    pub is_active: bool,
    pub priority: i32,
    pub health_status: String,
    pub rate_limit_qps: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// ProviderKey model matching Python `app/models/provider.py`
#[derive(Debug, Clone, FromRow)]
pub struct ProviderKey {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub key_encrypted: String,
    pub is_active: bool,
    pub weight: i32,
    pub fail_count: i32,
    pub max_fail_count: i32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

/// UsageLog model matching Python `app/models/usage.py`
#[derive(Debug, Clone, FromRow)]
pub struct UsageLog {
    pub id: i32,
    pub user_id: Uuid,
    pub token_id: Option<Uuid>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_rmb: BigDecimal,
    pub duration_ms: i32,
    pub is_stream: bool,
    pub is_success: bool,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub ip_address: Option<String>,
    pub request_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// ApiToken model matching Python `app/models/token.py`
#[derive(Debug, Clone, FromRow)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub token_prefix: String,
    pub name: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// ModelAlias
#[derive(Debug, Clone, FromRow)]
pub struct ModelAlias {
    pub id: Uuid,
    pub alias_name: String,
    pub target_model: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// --- OpenAI-compatible request/response schemas ---

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct Choice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// Model list
#[derive(Debug, Serialize)]
pub struct ModelListResponse {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub created: i64,
    pub owned_by: String,
}

// Health
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: f64,
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub database: String,
    pub redis: String,
    pub timestamp: f64,
}
