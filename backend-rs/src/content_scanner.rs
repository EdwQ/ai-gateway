use regex::Regex;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

const SENSITIVE_PATTERNS: &[(&str, &str, &str)] = &[
    ("api_key", r#"(?i)(sk-|sk-ant-|pk-)[a-zA-Z0-9_-]{20,}"#, "critical"),
    ("password", r#"(?i)("password"|"passwd"|"secret")[\s]*:[\s]*"[^"]{6,}""#, "critical"),
    ("phone", r#"1[3-9]\d{9}"#, "warning"),
    ("id_card", r#"\d{17}[\dXx]"#, "critical"),
    ("email", r#"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"#, "warning"),
    ("bank_card", r#"\d{16,19}"#, "warning"),
    ("token", r#"(?i)(token|api[_-]?key|access[_-]?key)[\s]*[:=][\s]*['"][a-zA-Z0-9_\-]{8,}['"]"#, "critical"),
];

pub struct ScanResult {
    pub mask_type: String,
    pub mask_pattern: String,
    pub match_count: i32,
    pub severity: String,
}

pub fn scan_content(content: &Value) -> Vec<ScanResult> {
    let text = match content.as_str() {
        Some(s) => s.to_string(),
        None => match serde_json::to_string(content) {
            Ok(s) => s,
            Err(_) => return vec![],
        },
    };

    let mut results = Vec::new();

    for (mask_type, pattern, severity) in SENSITIVE_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            let matches: Vec<_> = re.find_iter(&text).collect();
            if !matches.is_empty() {
                results.push(ScanResult {
                    mask_type: mask_type.to_string(),
                    mask_pattern: pattern.to_string(),
                    match_count: matches.len() as i32,
                    severity: severity.to_string(),
                });
            }
        }
    }

    results
}

pub async fn save_scan_results(
    pool: &PgPool,
    call_content_id: Uuid,
    results: &[ScanResult],
) {
    for result in results {
        let r = sqlx::query(
            "INSERT INTO content_masks (call_content_id, mask_type, mask_pattern, match_count, severity, created_at) \
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(call_content_id)
        .bind(&result.mask_type)
        .bind(&result.mask_pattern)
        .bind(result.match_count)
        .bind(&result.severity)
        .execute(pool)
        .await;

        if let Err(e) = r {
            tracing::warn!("Failed to save scan result: {}", e);
        }
    }
}

/// Check if a request is a duplicate (same prompt in last 5 minutes)
pub async fn detect_duplicate_request(
    pool: &PgPool,
    user_id: Uuid,
    prompt_preview: &str,
) -> bool {
    if prompt_preview.len() < 20 {
        return false;
    }
    let preview = &prompt_preview[..20.min(prompt_preview.len())];

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM call_contents \
         WHERE user_id = $1 \
           AND request_content::text LIKE $2 \
           AND created_at >= NOW() - INTERVAL '5 minutes'",
    )
    .bind(user_id)
    .bind(format!("%{}%", preview))
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    count > 5
}

/// Detect high frequency calls from a user in the last hour
pub async fn detect_high_frequency(
    pool: &PgPool,
    user_id: Uuid,
    threshold: i64,
) -> bool {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM call_contents \
         WHERE user_id = $1 AND created_at >= NOW() - INTERVAL '1 hour'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    count > threshold
}

/// Detect off-hours calls (0:00-6:00 local time)
pub async fn detect_off_hours(pool: &PgPool, user_id: Uuid) -> bool {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM call_contents \
         WHERE user_id = $1 \
           AND EXTRACT(HOUR FROM created_at) >= 0 \
           AND EXTRACT(HOUR FROM created_at) < 6 \
           AND created_at >= NOW() - INTERVAL '7 days'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    count > 20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_api_key() {
        let content = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": "My API key is sk-abc123def456ghi789jkl"
            }]
        });
        let results = scan_content(&content);
        let api_key_hits: Vec<_> = results.iter().filter(|r| r.mask_type == "api_key").collect();
        assert!(!api_key_hits.is_empty());
    }

    #[test]
    fn test_scan_phone() {
        let content = serde_json::json!({"text": "联系我 13800138000"});
        let results = scan_content(&content);
        let phone_hits: Vec<_> = results.iter().filter(|r| r.mask_type == "phone").collect();
        assert!(!phone_hits.is_empty());
    }

    #[test]
    fn test_scan_clean_content() {
        let content = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": "What is the capital of France?"
            }]
        });
        let results = scan_content(&content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_email() {
        let content = serde_json::json!({"email": "user@example.com"});
        let results = scan_content(&content);
        let email_hits: Vec<_> = results.iter().filter(|r| r.mask_type == "email").collect();
        assert!(!email_hits.is_empty());
    }
}
