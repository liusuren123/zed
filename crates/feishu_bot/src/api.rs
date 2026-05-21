use crate::config::FeishuBotConfig;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::Mutex;

/// Represents a message that can be sent via the Feishu API.
#[derive(Clone, Debug)]
pub enum FeishuMessage {
    /// A plain text message.
    Text(String),
    /// A markdown-formatted message (using Feishu's markdown subset).
    Markdown(String),
}

struct TokenCache {
    token: String,
    expires_at: Instant,
}

/// Client for interacting with the Feishu Open Platform API.
pub struct FeishuApi {
    config: FeishuBotConfig,
    http_client: reqwest::Client,
    token_cache: Arc<Mutex<Option<TokenCache>>>,
}

impl FeishuApi {
    /// Base URL for the Feishu Open Platform API.
    const BASE_URL: &'static str = "https://open.feishu.cn/open-apis";

    /// Create a new FeishuApi instance.
    pub fn new(config: FeishuBotConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            token_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Send a text message to a specific user via the Feishu Bot.
    ///
    /// Uses the Feishu Open Platform "Send Message" API:
    /// POST /im/v1/messages?receive_id_type=open_id
    pub async fn send_message(&self, user_id: &str, message: &FeishuMessage) -> Result<()> {
        let token = self.get_valid_token().await?;
        let (msg_type, content) = match message {
            FeishuMessage::Text(text) => {
                let escaped =
                    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{}\"", text));
                let content = format!("{{\"text\":{}}}", escaped);
                ("text", content)
            }
            FeishuMessage::Markdown(md) => {
                let escaped = serde_json::to_string(md).unwrap_or_else(|_| format!("\"{}\"", md));
                let content = format!("{{\"text\":{}}}", escaped);
                ("text", content)
            }
        };

        let body = serde_json::json!({
            "receive_id": user_id,
            "msg_type": msg_type,
            "content": content,
        });

        let url = format!("{}/im/v1/messages?receive_id_type=open_id", Self::BASE_URL);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .context("failed to send message to Feishu API")?;

        let status = response.status();
        let response_body: serde_json::Value = response
            .json()
            .await
            .context("failed to parse Feishu API response")?;

        let code = response_body
            .get("code")
            .and_then(|c| c.as_i64())
            .unwrap_or(-1);
        if code != 0 {
            let msg = response_body
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            log::warn!("Feishu API error: code={} msg={}", code, msg);
            anyhow::bail!("Feishu API error: code={} msg={}", code, msg);
        }

        log::info!(
            "[Feishu Bot] Message sent: status={} type={}",
            status,
            msg_type
        );
        Ok(())
    }

    /// Get a valid tenant access token, reusing cached token if still valid.
    async fn get_valid_token(&self) -> Result<String> {
        {
            let cache = self.token_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if cached.expires_at > Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let new_token = self.fetch_tenant_access_token().await?;
        let mut cache = self.token_cache.lock().unwrap();
        *cache = Some(TokenCache {
            token: new_token.clone(),
            // Tokens are valid for ~2 hours; refresh after 1.5 hours
            expires_at: Instant::now() + Duration::from_secs(5400),
        });
        Ok(new_token)
    }

    /// Fetch a new tenant access token from the Feishu API.
    ///
    /// POST /auth/v3/tenant_access_token/internal
    async fn fetch_tenant_access_token(&self) -> Result<String> {
        let url = format!("{}/auth/v3/tenant_access_token/internal", Self::BASE_URL);

        let body = serde_json::json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .context("failed to fetch tenant access token")?;

        let response_body: serde_json::Value = response
            .json()
            .await
            .context("failed to parse token response")?;

        let code = response_body
            .get("code")
            .and_then(|c| c.as_i64())
            .unwrap_or(-1);
        if code != 0 {
            let msg = response_body
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("Feishu auth error: code={} msg={}", code, msg);
        }

        let token = response_body
            .get("tenant_access_token")
            .and_then(|t| t.as_str())
            .context("missing tenant_access_token in response")?;

        log::info!("[Feishu Bot] Obtained tenant access token");
        Ok(token.to_string())
    }
}
