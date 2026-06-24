use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::ContentCaptureConfig;

pub type CaptureSender = Arc<ContentCapture>;

const CHANNEL_CAPACITY: usize = 2048;
const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone)]
pub struct CaptureEvent {
    pub user_id: Uuid,
    pub token_id: Option<Uuid>,
    pub request_id: String,
    pub model: String,
    pub provider: String,
    pub request_content: Value,
    pub response_content: Option<Value>,
    pub file_metadata: Vec<Value>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub latency_ms: i32,
    pub is_stream: bool,
    pub ip_address: Option<String>,
}

pub struct ContentCapture {
    pub config: ContentCaptureConfig,
    sender: mpsc::Sender<CaptureEvent>,
}

impl ContentCapture {
    pub fn new(pool: PgPool, config: ContentCaptureConfig) -> Self {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        tokio::spawn(Self::worker(rx, pool, config.retention_days));
        Self { config, sender: tx }
    }

    async fn worker(
        mut rx: mpsc::Receiver<CaptureEvent>,
        pool: PgPool,
        retention_days: u32,
    ) {
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    batch.push(event);
                    if batch.len() >= BATCH_SIZE {
                        Self::flush_batch(&batch, &pool, retention_days).await;
                        batch.clear();
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(FLUSH_INTERVAL_MS)) => {
                    if !batch.is_empty() {
                        Self::flush_batch(&batch, &pool, retention_days).await;
                        batch.clear();
                    }
                }
                else => break,
            }
        }
    }

    async fn flush_batch(batch: &[CaptureEvent], pool: &PgPool, retention_days: u32) {
        let expires_at = Utc::now() + Duration::days(retention_days as i64);
        for event in batch {
            let file_metadata = sqlx::types::Json(&event.file_metadata);
            let result = sqlx::query(
                r#"INSERT INTO call_contents
                   (user_id, token_id, request_id, model, provider,
                    request_content, response_content, file_metadata,
                    input_tokens, output_tokens, latency_ms, is_stream,
                    ip_address, created_at, expires_at)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,NOW(),$14)"#,
            )
            .bind(event.user_id)
            .bind(event.token_id)
            .bind(&event.request_id)
            .bind(&event.model)
            .bind(&event.provider)
            .bind(&event.request_content)
            .bind(&event.response_content)
            .bind(file_metadata)
            .bind(event.input_tokens)
            .bind(event.output_tokens)
            .bind(event.latency_ms)
            .bind(event.is_stream)
            .bind(&event.ip_address)
            .bind(expires_at)
            .execute(pool)
            .await;

            if let Err(e) = result {
                tracing::warn!("content_capture insert failed: {}", e);
            }
        }
    }

    pub fn send(&self, event: CaptureEvent) {
        if !self.config.enabled {
            return;
        }
        if let Err(e) = self.sender.try_send(event) {
            tracing::warn!("content_capture channel full: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_event_creation() {
        let event = CaptureEvent {
            user_id: Uuid::new_v4(),
            token_id: None,
            request_id: "req-123".to_string(),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            request_content: serde_json::json!({"messages": [{"role": "user", "content": "hello"}]}),
            response_content: Some(serde_json::json!({"choices": [{"message": {"content": "hi"}}]})),
            file_metadata: vec![],
            input_tokens: 10,
            output_tokens: 5,
            latency_ms: 150,
            is_stream: false,
            ip_address: Some("127.0.0.1".to_string()),
        };

        assert_eq!(event.model, "gpt-4");
        assert_eq!(event.input_tokens, 10);
        assert_eq!(event.output_tokens, 5);
        assert_eq!(event.is_stream, false);
        assert!(event.response_content.is_some());
    }

    #[test]
    fn test_capture_config_defaults() {
        let config = ContentCaptureConfig {
            enabled: true,
            retention_days: 30,
            mask_enabled: true,
        };
        assert!(config.enabled);
        assert_eq!(config.retention_days, 30);
    }

    #[test]
    fn test_file_metadata_defaults_to_empty() {
        let event = CaptureEvent {
            user_id: Uuid::new_v4(),
            token_id: None,
            request_id: "req-456".to_string(),
            model: "claude-3".to_string(),
            provider: "anthropic".to_string(),
            request_content: serde_json::json!({"messages": []}),
            response_content: None,
            file_metadata: vec![],
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            is_stream: true,
            ip_address: None,
        };

        assert!(event.file_metadata.is_empty());
        assert!(event.response_content.is_none());
        assert!(event.ip_address.is_none());
    }
}
