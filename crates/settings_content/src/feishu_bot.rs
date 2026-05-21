use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct FeishuBotSettingsContent {
    /// Whether the Feishu Bot integration is enabled.
    ///
    /// Default: false
    pub enabled: Option<bool>,
    /// The Feishu application ID from the Feishu Open Platform.
    pub app_id: Option<String>,
    /// The Feishu application secret for API authentication.
    pub app_secret: Option<String>,
    /// Whether the bot is currently bound to a user/session.
    pub bound: Option<bool>,
    /// The Feishu user identifier that the bot is bound to.
    pub bound_user_id: Option<String>,
    /// The workspace identifier the bot is bound to.
    pub bound_workspace: Option<String>,
}
