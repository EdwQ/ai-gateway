//! PII masking utilities.
//!
//! Ported from `backend/app/utils/mask.py`

use regex::Regex;

/// Mask Chinese phone numbers: 138****1234
/// Matches 11-digit Chinese mobile numbers starting with 1[3-9]
pub fn mask_phone(text: &str) -> String {
    let re = Regex::new(r"(1[3-9]\d)\d{4}(\d{4})").unwrap();
    re.replace_all(text, "$1****$2").to_string()
}

/// Mask Chinese ID card numbers
pub fn mask_id_card(text: &str) -> String {
    let re = Regex::new(r"\b\d{17}[\dXx]\b").unwrap();
    re.replace_all(text, |caps: &regex::Captures| {
        let s = &caps[0];
        if s.len() == 18 {
            format!("{}********{}", &s[..6], &s[s.len() - 4..])
        } else {
            s.to_string()
        }
    })
    .to_string()
}

/// Mask email addresses
pub fn mask_email(text: &str) -> String {
    let re = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    re.replace_all(text, |caps: &regex::Captures| {
        let email = &caps[0];
        if let Some(at_pos) = email.find('@') {
            let local = &email[..at_pos];
            let domain = &email[at_pos..];
            let masked_local = if local.len() <= 2 {
                format!("{}***", &local[..1])
            } else {
                format!("{}***{}", &local[..1], &local[local.len() - 1..])
            };
            format!("{}{}", masked_local, domain)
        } else {
            email.to_string()
        }
    })
    .to_string()
}

/// Mask bank card numbers: 6222********1234
pub fn mask_bank_card(text: &str) -> String {
    let re = Regex::new(r"\b(\d{4})\d{8,11}(\d{4})\b").unwrap();
    re.replace_all(text, "$1********$2").to_string()
}

/// Mask API keys (sk-xxx format)
/// Captures the prefix (sk-xxx, up to 8 chars) and last 4 chars, masks everything in between.
/// Uses explicit length limits to work correctly with Rust's regex engine (no backtracking).
pub fn mask_api_key(text: &str) -> String {
    // group3 = last 4 chars, {16,} = middle part (masked), group1 = prefix up to 8 chars
    let re = Regex::new(r"(?i)(sk-[a-z0-9]{1,6})([a-z0-9]{16,})([a-z0-9]{4})\b").unwrap();
    re.replace_all(text, "$1****$3").to_string()
}

/// Apply all masking functions
pub fn mask_all(text: &str) -> String {
    let text = mask_phone(text);
    let text = mask_id_card(&text);
    let text = mask_email(&text);
    let text = mask_bank_card(&text);
    let text = mask_api_key(&text);
    text
}

/// Mask prompt based on mode. Returns (content, summary).
///
/// - `full`: content = full prompt
/// - `masked`: content = masked prompt (PII removed)
/// - `summary`: summary = first 100 chars
/// - `off`: both None
pub fn mask_prompt(prompt: &str, mode: &str) -> (Option<String>, Option<String>) {
    match mode {
        "full" => (Some(prompt.to_string()), None),
        "masked" => (Some(mask_all(prompt)), None),
        "summary" => {
            let summary = if prompt.len() > 100 {
                format!("{}...", &prompt[..100])
            } else {
                prompt.to_string()
            };
            (None, Some(summary))
        }
        _ => (None, None), // "off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_phone() {
        assert_eq!(mask_phone("13812345678"), "138****5678");
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(mask_email("test@example.com"), "t***t@example.com");
        assert_eq!(mask_email("ab@example.com"), "a***@example.com");
    }

    #[test]
    fn test_mask_api_key() {
        assert_eq!(
            mask_api_key("sk-abcdef1234567890abcdef1234567890abcd1234"),
            "sk-abcdef****1234"
        );
        // Short API key (e.g., from env config)
        let short_result = mask_api_key("my api key is sk-abcdef1234567890abcdef1a2b3c4d5e, keep it safe");
        assert!(short_result.contains("****"));
        assert!(!short_result.contains("sk-abcdef1234567890abcdef1a2b3c4d5e"));
    }

    #[test]
    fn test_mask_all() {
        let input = "Phone: 13812345678, Email: test@example.com";
        let result = mask_all(input);
        assert!(!result.contains("13812345678"));
        assert!(!result.contains("test@example.com"));
    }
}
