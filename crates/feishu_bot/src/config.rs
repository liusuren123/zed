/// Rate limiting configuration for the Feishu bot.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum number of requests per minute per user.
    pub max_requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 30,
        }
    }
}

/// Configuration for the Feishu bot integration.
#[derive(Clone, Debug)]
pub struct FeishuBotConfig {
    /// Feishu application ID from the Feishu Open Platform.
    pub app_id: String,
    /// Feishu application secret for API authentication.
    pub app_secret: String,
    /// Verification token for webhook event validation.
    pub verification_token: String,
    /// Optional webhook listen URL (for self-hosted callback).
    pub webhook_url: Option<String>,
    /// Rate limiting configuration.
    pub rate_limit: RateLimitConfig,
    /// Maximum file size in bytes before content is truncated.
    pub max_file_size: u64,
}

impl Default for FeishuBotConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            verification_token: String::new(),
            webhook_url: None,
            rate_limit: RateLimitConfig::default(),
            max_file_size: 524288, // 512 KB
        }
    }
}
