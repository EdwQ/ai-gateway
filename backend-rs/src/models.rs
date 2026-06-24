use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// For DB model fields that need sqlx compatibility
// (sqlx's "bigdecimal" feature provides this type alias)
type SqlxDecimal = sqlx::types::BigDecimal;

/// Convert sqlx BigDecimal to rust_decimal Decimal for API responses.
/// sqlx's "bigdecimal" feature provides BigDecimal, and rust_decimal::Decimal handles serde.
pub fn sqlx_decimal_to_rust(d: &SqlxDecimal) -> Decimal {
    // Use string representation as the reliable bridge between the two Decimal types
    rust_decimal::Decimal::from_str_exact(&d.to_string()).unwrap_or(rust_decimal::Decimal::ZERO)
}

// ============================================================
// DB Models (FromRow)
// ============================================================

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
    pub quota_balance: SqlxDecimal,
    pub quota_used: SqlxDecimal,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub allowed_models: Option<sqlx::types::Json<Vec<String>>>,
}

/// Provider model
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

/// ProviderKey model
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

/// UsageLog model
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
    pub cost_rmb: SqlxDecimal,
    pub duration_ms: i32,
    pub is_stream: bool,
    pub is_success: bool,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub ip_address: Option<String>,
    pub request_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// ApiToken model
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
    pub expires_at: Option<DateTime<Utc>>,
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

/// AuditLog model
#[derive(Debug, Clone, FromRow)]
pub struct AuditLog {
    pub id: i32,
    pub user_id: Uuid,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<sqlx::types::Json<serde_json::Value>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Department model
#[derive(Debug, Clone, FromRow)]
pub struct Department {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub order_num: Option<i32>,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
}

/// PromptAudit model
#[derive(Debug, Clone, FromRow)]
pub struct PromptAudit {
    pub id: i32,
    pub usage_log_id: i32,
    pub save_mode: String,
    pub prompt_content: Option<String>,
    pub prompt_summary: Option<String>,
    pub completion_content: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ============================================================
// OpenAI-compatible request/response schemas
// ============================================================

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

// ============================================================
// Auth API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct DingTalkQRCodeResponse {
    pub qr_code_url: String,
}

#[derive(Debug, Deserialize)]
pub struct DingTalkCallbackRequest {
    pub auth_code: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub role: String,
    pub department_name: Option<String>,
    pub quota_balance: Decimal,
    pub quota_used: Decimal,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub user: UserInfo,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserInfo,
}

// ============================================================
// User API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub union_id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub title: Option<String>,
    pub role: String,
    pub allowed_models: Vec<String>,
    pub is_active: bool,
    pub quota_balance: Decimal,
    pub quota_used: Decimal,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub items: Vec<UserResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub quota_balance: Option<Decimal>,
    pub allowed_models: Option<Vec<String>>,
}

// ============================================================
// Token API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: String,
    pub token_prefix: String,
    pub name: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ApiTokenCreateRequest {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenCreatedResponse {
    pub id: String,
    pub token: String,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenListResponse {
    pub items: Vec<ApiTokenResponse>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenRotateResponse {
    pub id: String,
    pub token: String,
    pub name: String,
}

// ============================================================
// Provider API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct ProviderResponse {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub is_active: bool,
    pub priority: i32,
    pub health_status: String,
    pub rate_limit_qps: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderCreateRequest {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_qps")]
    pub rate_limit_qps: i32,
}

#[derive(Debug, Deserialize)]
pub struct ProviderUpdateRequest {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub models: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub priority: Option<i32>,
    pub rate_limit_qps: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ProviderKeyResponse {
    pub id: String,
    pub is_active: bool,
    pub weight: i32,
    pub fail_count: i32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ProviderListResponse {
    pub items: Vec<ProviderResponse>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub latency_ms: f64,
}

#[derive(Debug, Deserialize)]
pub struct DiscoverModelsRequest {
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct DiscoverModelsResponse {
    pub models: Vec<String>,
    pub error: Option<String>,
}

fn default_true() -> bool { true }
fn default_priority() -> i32 { 100 }
fn default_qps() -> i32 { 60 }

// ============================================================
// Model Alias DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct AliasResponse {
    pub id: String,
    pub alias_name: String,
    pub target_model: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AliasListResponse {
    pub items: Vec<AliasResponse>,
}

#[derive(Debug, Deserialize)]
pub struct AliasCreateRequest {
    pub alias_name: String,
    pub target_model: String,
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct AliasUpdateRequest {
    pub alias_name: Option<String>,
    pub target_model: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================================
// Stats API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub model_rank: Vec<ModelRankItem>,
}

#[derive(Debug, Serialize)]
pub struct ModelRankItem {
    pub model: String,
    pub calls: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct DailyStatsResponse {
    pub items: Vec<DailyStatsItem>,
}

#[derive(Debug, Serialize)]
pub struct DailyStatsItem {
    pub date: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyStatsResponse {
    pub items: Vec<MonthlyStatsItem>,
}

#[derive(Debug, Serialize)]
pub struct MonthlyStatsItem {
    pub month: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

// ============================================================
// Audit API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: i32,
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub items: Vec<AuditLogResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ============================================================
// Query params helpers
// ============================================================

// Parse a string as i64, used with serde(deserialize_with)
fn parse_i64_from_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<i64>().map_err(serde::de::Error::custom)
}

fn parse_opt_i64_from_string<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(v) => v.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn parse_opt_bool_from_string<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(v) => match v.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(Some(true)),
            "false" | "0" | "no" => Ok(Some(false)),
            _ => Err(serde::de::Error::custom(format!("invalid bool: {}", v))),
        },
        None => Ok(None),
    }
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 20 }

#[derive(Debug, Deserialize)]
pub struct UserQueryParams {
    #[serde(default = "default_page", deserialize_with = "parse_i64_from_string")]
    pub page: i64,
    #[serde(default = "default_page_size", deserialize_with = "parse_i64_from_string")]
    pub page_size: i64,
    pub search: Option<String>,
    #[serde(default, deserialize_with = "parse_opt_bool_from_string")]
    pub is_active: Option<bool>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQueryParams {
    #[serde(default = "default_page", deserialize_with = "parse_i64_from_string")]
    pub page: i64,
    #[serde(default = "default_page_size", deserialize_with = "parse_i64_from_string")]
    pub page_size: i64,
    pub action: Option<String>,
    pub user_id: Option<String>,
    pub resource_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DailyStatsParams {
    #[serde(default = "default_days", deserialize_with = "parse_i64_from_string")]
    pub days: i64,
    pub user_id: Option<String>,
    pub model: Option<String>,
}

fn default_days() -> i64 { 30 }

#[derive(Debug, Deserialize)]
pub struct MonthlyStatsParams {
    #[serde(default = "default_months", deserialize_with = "parse_i64_from_string")]
    pub months: i64,
}

fn default_months() -> i64 { 6 }

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub month: String,
}

// ============================================================
// Analysis / Behavior Data Models
// ============================================================

/// CallContent — stores full request/response for behavior analysis
#[derive(Debug, Clone, FromRow)]
pub struct CallContent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_id: Option<Uuid>,
    pub request_id: Option<String>,
    pub model: String,
    pub provider: String,
    pub request_content: sqlx::types::Json<serde_json::Value>,
    pub response_content: Option<sqlx::types::Json<serde_json::Value>>,
    pub file_metadata: sqlx::types::Json<Vec<serde_json::Value>>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub latency_ms: i32,
    pub is_stream: bool,
    pub ip_address: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// ContentMask — records of sensitive data pattern matches
#[derive(Debug, Clone, FromRow)]
pub struct ContentMask {
    pub id: i32,
    pub call_content_id: Uuid,
    pub mask_type: String,
    pub mask_pattern: String,
    pub match_count: i32,
    pub matched_fields: sqlx::types::Json<Vec<String>>,
    pub severity: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================
// Analysis API DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct CallContentResponse {
    pub id: String,
    pub user_id: String,
    pub model: String,
    pub provider: String,
    pub request_content: serde_json::Value,
    pub response_content: Option<serde_json::Value>,
    pub file_metadata: Vec<serde_json::Value>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub latency_ms: i32,
    pub is_stream: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DashboardAnalysisResponse {
    pub total_calls: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub active_users: i64,
}

#[derive(Debug, Serialize)]
pub struct TrendItem {
    pub date: String,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalysisUserRankItem {
    pub user_id: String,
    pub user_name: String,
    pub calls: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalysisModelRankItem {
    pub model: String,
    pub calls: i64,
    pub total_tokens: i64,
    pub cost: f64,
    pub avg_latency_ms: f64,
}

// ============================================================
// Error response
// ============================================================

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub detail: String,
}
