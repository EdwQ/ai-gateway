//! DingTalk OAuth & API client for internal enterprise apps.
//!
//! Correct flow for QR code login (扫码登录第三方网站):
//!   1. GET  /gettoken?appkey=xxx&appsecret=xxx           → access_token
//!   2. POST /sns/getuserinfo_bycode?accessKey=xxx&timestamp=xxx&signature=xxx
//!      Body: {"tmp_auth_code": "CODE"}                    → {unionid, nick, openid}
//!      NOTE: Uses appKey + appSecret + HMAC signature, NOT access_token.
//!   3. POST /topapi/user/getbyunionid?access_token=TOKEN
//!      Body: {"unionid": "UNIONID"}                      → {userid}
//!   4. POST /topapi/v2/user/get?access_token=TOKEN
//!      Body: {"userid": "USERID"}                        → full user details
//!
//! Ported from `backend/app/services/dingtalk_service.py`

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use urlencoding::encode as url_encode;

use crate::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

/// DingTalk user info returned from the full OAuth flow
#[derive(Debug, Clone, Default)]
pub struct DingTalkUser {
    pub unionid: String,
    pub userid: String,
    pub name: String,
    pub nick: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub title: Option<String>,
    pub openid: Option<String>,
    pub dept_id_list: Option<Vec<String>>,
    pub mobile: Option<String>,
}

/// DingTalk API client
pub struct DingTalkClient {
    http_client: reqwest::Client,
    access_token: tokio::sync::Mutex<Option<AccessTokenCache>>,
}

struct AccessTokenCache {
    token: String,
    expires_at: u64, // unix seconds
}

impl DingTalkClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("Failed to create HTTP client"),
            access_token: tokio::sync::Mutex::new(None),
        }
    }

    /// Get internal app access token with caching (expires 10 min early)
    async fn get_access_token(&self, config: &AppConfig) -> Result<String, String> {
        // Check cache
        {
            let cache = self.access_token.lock().await;
            if let Some(cached) = cache.as_ref() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| format!("Time error: {}", e))?
                    .as_secs();
                if now < cached.expires_at {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Fetch new token
        let url = format!(
            "https://oapi.dingtalk.com/gettoken?appkey={}&appsecret={}",
            config.dingtalk_app_key,
            config.dingtalk_app_secret,
        );

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("DingTalk token request error: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("DingTalk token parse error: {}", e))?;

        let errcode = data["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            let errmsg = data["errmsg"].as_str().unwrap_or("unknown");
            return Err(format!("DingTalk token error: {}", errmsg));
        }

        let token = data["access_token"]
            .as_str()
            .ok_or("Missing access_token in DingTalk response")?
            .to_string();

        let expires_in = data["expires_in"].as_i64().unwrap_or(7200);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))?
            .as_secs();

        // Cache it (expire 10 minutes early)
        let mut cache = self.access_token.lock().await;
        *cache = Some(AccessTokenCache {
            token: token.clone(),
            expires_at: now + expires_in as u64 - 600,
        });

        Ok(token)
    }

    /// Compute HMAC-SHA256 signature for sns/getuserinfo_bycode
    fn compute_signature(app_secret: &str, timestamp: &str) -> Result<String, String> {
        let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
            .map_err(|e| format!("HMAC error: {}", e))?;
        mac.update(timestamp.as_bytes());
        let result = mac.finalize();
        let code = result.into_bytes();
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &code,
        ))
    }

    /// Get user info via auth_code from DingTalk QR code login (full 4-step flow)
    pub async fn get_user_info(
        &self,
        auth_code: &str,
        config: &AppConfig,
    ) -> Result<DingTalkUser, String> {
        let token = self.get_access_token(config).await?;

        // Step 1: exchange sns temporary auth_code for unionid
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))?
            .as_millis() as u64;
        let timestamp = now_ms.to_string();
        let signature = Self::compute_signature(&config.dingtalk_app_secret, &timestamp)?;

        let sns_resp = self
            .http_client
            .post("https://oapi.dingtalk.com/sns/getuserinfo_bycode")
            .query(&[
                ("accessKey", config.dingtalk_app_key.as_str()),
                ("timestamp", timestamp.as_str()),
                ("signature", signature.as_str()),
            ])
            .json(&serde_json::json!({
                "tmp_auth_code": auth_code,
            }))
            .send()
            .await
            .map_err(|e| format!("DingTalk sns request error: {}", e))?;

        let sns_data: serde_json::Value = sns_resp
            .json()
            .await
            .map_err(|e| format!("DingTalk sns parse error: {}", e))?;

        let errcode = sns_data["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            let errmsg = sns_data["errmsg"].as_str().unwrap_or("unknown");
            return Err(format!("DingTalk sns error: {}", errmsg));
        }

        let user_info_node = &sns_data["user_info"];
        let union_id = user_info_node["unionid"]
            .as_str()
            .ok_or("Failed to get unionid from DingTalk sns API")?
            .to_string();

        let mut user = DingTalkUser {
            unionid: union_id.clone(),
            nick: user_info_node["nick"].as_str().unwrap_or("").to_string(),
            openid: user_info_node["openid"].as_str().map(|s| s.to_string()),
            ..Default::default()
        };

        // Step 2: unionid → userid via topapi/user/getbyunionid
        let unionid_resp = self
            .http_client
            .post("https://oapi.dingtalk.com/topapi/user/getbyunionid")
            .query(&[("access_token", token.as_str())])
            .json(&serde_json::json!({
                "unionid": user.unionid,
            }))
            .send()
            .await
            .map_err(|e| format!("DingTalk getbyunionid error: {}", e))?;

        let unionid_data: serde_json::Value = unionid_resp
            .json()
            .await
            .map_err(|e| format!("DingTalk getbyunionid parse error: {}", e))?;

        let errcode2 = unionid_data["errcode"].as_i64().unwrap_or(-1);
        if errcode2 != 0 {
            let errmsg = unionid_data["errmsg"].as_str().unwrap_or("unknown");
            return Err(format!("DingTalk getbyunionid error: {}", errmsg));
        }

        let unionid_result = &unionid_data["result"];
        let userid = unionid_result["userid"]
            .as_str()
            .ok_or("Failed to get userid from DingTalk getbyunionid")?
            .to_string();

        user.userid = userid;
        user.name = unionid_result["name"]
            .as_str()
            .unwrap_or(&user.nick)
            .to_string();
        user.avatar = unionid_result["avatar"].as_str().map(|s| s.to_string());

        // Step 3: get full user details by userid
        let detail_resp = self
            .http_client
            .post("https://oapi.dingtalk.com/topapi/v2/user/get")
            .query(&[("access_token", token.as_str())])
            .json(&serde_json::json!({
                "userid": user.userid,
                "language": "zh_CN",
            }))
            .send()
            .await
            .map_err(|e| format!("DingTalk user detail error: {}", e))?;

        let detail_data: serde_json::Value = detail_resp
            .json()
            .await
            .map_err(|e| format!("DingTalk user detail parse error: {}", e))?;

        let errcode3 = detail_data["errcode"].as_i64().unwrap_or(-1);
        if errcode3 == 0 {
            let detail_result = &detail_data["result"];
            if let Some(name) = detail_result["name"].as_str() {
                user.name = name.to_string();
            }
            user.email = detail_result["email"].as_str().map(|s| s.to_string());
            if let Some(avatar) = detail_result["avatar"].as_str() {
                user.avatar = Some(avatar.to_string());
            }
            user.title = detail_result["title"].as_str().map(|s| s.to_string());

            // Parse dept_id_list
            if let Some(dept_ids) = detail_result["dept_id_list"].as_array() {
                let ids: Vec<String> = dept_ids
                    .iter()
                    .filter_map(|v| v.as_i64())
                    .map(|v| v.to_string())
                    .collect();
                if !ids.is_empty() {
                    user.dept_id_list = Some(ids);
                }
            }
            user.mobile = detail_result["mobile"].as_str().map(|s| s.to_string());
            user.unionid = detail_result["unionid"]
                .as_str()
                .unwrap_or(&user.unionid)
                .to_string();
        }

        Ok(user)
    }

    /// Generate DingTalk QR code login URL (方式一: standalone page)
    pub fn get_qrcode_url(config: &AppConfig, redirect_uri: &str, state: &str) -> String {
        let encoded_redirect = url_encode(redirect_uri);
        format!(
            "https://oapi.dingtalk.com/connect/qrconnect\
             ?appid={}\
             &response_type=code\
             &scope=snsapi_login\
             &state={}\
             &redirect_uri={}",
            config.dingtalk_app_key, state, encoded_redirect,
        )
    }

    /// Get department list
    pub async fn get_departments(
        &self,
        dept_id: &str,
        config: &AppConfig,
    ) -> Result<Vec<serde_json::Value>, String> {
        let token = self.get_access_token(config).await?;

        let resp = self
            .http_client
            .post("https://oapi.dingtalk.com/topapi/v2/department/listsub")
            .query(&[("access_token", token.as_str())])
            .json(&serde_json::json!({
                "dept_id": dept_id,
            }))
            .send()
            .await
            .map_err(|e| format!("DingTalk dept list error: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("DingTalk dept list parse error: {}", e))?;

        let errcode = data["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            let errmsg = data["errmsg"].as_str().unwrap_or("unknown");
            return Err(format!("DingTalk dept list error: {}", errmsg));
        }

        Ok(data["result"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Get department detail
    pub async fn get_department_detail(
        &self,
        dept_id: &str,
        config: &AppConfig,
    ) -> Result<serde_json::Value, String> {
        let token = self.get_access_token(config).await?;

        let resp = self
            .http_client
            .post("https://oapi.dingtalk.com/topapi/v2/department/get")
            .query(&[("access_token", token.as_str())])
            .json(&serde_json::json!({
                "dept_id": dept_id,
            }))
            .send()
            .await
            .map_err(|e| format!("DingTalk dept detail error: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("DingTalk dept detail parse error: {}", e))?;

        let errcode = data["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            let errmsg = data["errmsg"].as_str().unwrap_or("unknown");
            return Err(format!("DingTalk dept detail error: {}", errmsg));
        }

        Ok(data["result"].clone())
    }
}
