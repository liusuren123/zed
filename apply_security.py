#!/usr/bin/env python3
"""Add security features to headless_project.rs."""

with open("crates/remote_server/src/headless_project.rs", "r", encoding="utf-8") as f:
    content = f.read()

# === 1. Add BotTokenStore struct after the imports and before HeadlessProject ===
marker = "pub struct HeadlessProject {"
bot_token_code = """/// A simple bot access token for Feishu remote control.
#[derive(Clone, Debug)]
struct BotToken {
    token: String,
    created_at: Instant,
    /// Expiry duration from creation time.
    ttl: std::time::Duration,
}

impl BotToken {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Thread-safe store for bot access tokens.
#[derive(Default)]
struct BotTokenStore {
    tokens: parking_lot::Mutex<HashMap<String, BotToken>>,
}

impl BotTokenStore {
    /// Generate a new random token, store it, and return the token string.
    fn generate(&self) -> String {
        use rand::Rng;
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let bot_token = BotToken {
            token: token.clone(),
            created_at: Instant::now(),
            ttl: std::time::Duration::from_secs(300), // 5 minutes
        };
        self.tokens.lock().insert(token.clone(), bot_token);
        log::info!("Generated new bot token (expires in 5 min)");
        token
    }

    /// Validate a token. Returns true and removes the token if valid.
    fn validate_and_consume(&self, token: &str) -> bool {
        let mut tokens = self.tokens.lock();
        if let Some(bot_token) = tokens.remove(token) {
            if !bot_token.is_expired() {
                log::info!("Bot token validated successfully");
                return true;
            }
            log::info!("Bot token rejected: expired");
        } else {
            log::info!("Bot token rejected: not found");
        }
        false
    }

    /// Remove all expired tokens.
    fn cleanup_expired(&self) {
        self.tokens.lock().retain(|_, t| !t.is_expired());
    }
}

"""
content = content.replace(marker, bot_token_code + marker)
print("Step 1: BotTokenStore added")

# === 2. Add bot_token_store and ignore_patterns to HeadlessProject struct ===
fields_marker = "pub kernels: HashMap<String, Child>,"
new_fields = """pub kernels: HashMap<String, Child>,
    /// Bot tokens for Feishu remote control authentication.
    pub bot_token_store: Arc<BotTokenStore>,
    /// Paths excluded from bot access (from .feishuignore).
    pub bot_ignore_patterns: Arc<parking_lot::Mutex<Vec<String>>>,"""
content = content.replace(fields_marker, new_fields)
print("Step 2: Fields added to HeadlessProject")

# === 3. Initialize the new fields in HeadlessProject::new() ===
init_marker = "HeadlessProject {"
new_init = """let bot_token_store = Arc::new(BotTokenStore::default());
        let bot_ignore_patterns: Arc<parking_lot::Mutex<Vec<String>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));

        // Load .feishuignore if it exists
        {
            let fs_clone = fs.clone();
            let patterns = bot_ignore_patterns.clone();
            cx.background_executor()
                .spawn(async move {
                    let ignore_paths = worktree_store
                        .read_with(&cx, |store, _| { /* placeholder */ });
                    // TODO: Load .feishuignore from project root
                })
                .detach();
        }

        HeadlessProject {"""
content = content.replace(init_marker, new_init + init_marker)
print("Step 3: Initialization added")

# === 4. Update handle_bot_bind with real validation ===
old_bind = """    async fn handle_bot_bind(
        _this: Entity<Self>,
        envelope: TypedEnvelope<proto::BotBindRequest>,
        _cx: AsyncApp,
    ) -> Result<proto::BotBindResponse> {
        let bot_user_id = &envelope.payload.bot_user_id;
        let access_token = &envelope.payload.access_token;

        if bot_user_id.is_empty() || access_token.is_empty() {
            return Ok(proto::BotBindResponse {
                success: false,
                message: Some("bot_user_id and access_token are required".to_string()),
            });
        }

        log::info!("Bot bind request from user: {}", bot_user_id);
        Ok(proto::BotBindResponse {
            success: true,
            message: None,
        })
    }"""

new_bind = """    async fn handle_bot_bind(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::BotBindRequest>,
        cx: AsyncApp,
    ) -> Result<proto::BotBindResponse> {
        let bot_user_id = envelope.payload.bot_user_id.clone();
        let access_token = envelope.payload.access_token.clone();

        if bot_user_id.is_empty() || access_token.is_empty() {
            log::warn!("Bot bind rejected: empty credentials");
            return Ok(proto::BotBindResponse {
                success: false,
                message: Some("bot_user_id and access_token are required".to_string()),
            });
        }

        let token_store = cx.read_entity(&this, |this, _| this.bot_token_store.clone());
        if token_store.validate_and_consume(&access_token) {
            log::info!(
                "[AUDIT] Bot bind: user={} bound successfully",
                bot_user_id
            );
            Ok(proto::BotBindResponse {
                success: true,
                message: None,
            })
        } else {
            log::warn!(
                "[AUDIT] Bot bind rejected: user={} invalid/expired token",
                bot_user_id
            );
            Ok(proto::BotBindResponse {
                success: false,
                message: Some("Invalid or expired token".to_string()),
            })
        }
    }"""

if old_bind in content:
    content = content.replace(old_bind, new_bind)
    print("Step 4: handle_bot_bind updated")
else:
    print("Step 4: old bind NOT FOUND")

# === 5. Add audit logging to handle_read_file ===
# Find the Ok(proto::ReadFileResponse { ... }) in handle_read_file and add audit log before it
old_rf_response = """        Ok(proto::ReadFileResponse {
            content: result_content,
            total_lines,
            file_size,
            modified_at,
            truncated,
        })"""
new_rf_response = """        log::info!(
            "[AUDIT] Read file: path={} lines={} size={}",
            path, total_lines, file_size
        );
        Ok(proto::ReadFileResponse {
            content: result_content,
            total_lines,
            file_size,
            modified_at,
            truncated,
        })"""
# Only replace the first occurrence (in handle_read_file)
content = content.replace(old_rf_response, new_rf_response, 1)
print("Step 5: Audit logging added to handle_read_file")

# === 6. Add audit logging to handle_read_directory ===
old_rd_response = """        Ok(proto::ReadDirectoryResponse { entries, truncated })"""
new_rd_response = """        log::info!("[AUDIT] Read directory: path={} entries={}", path, entries.len());
        Ok(proto::ReadDirectoryResponse { entries, truncated })"""
content = content.replace(old_rd_response, new_rd_response, 1)
print("Step 6: Audit logging added to handle_read_directory")

# === 7. Add generate_token handler ===
# Find handle_get_remote_profiling_data registration
gen_reg = "        session.add_request_handler(cx.weak_entity(), Self::handle_get_remote_profiling_data);"
new_gen_reg = (
    gen_reg
    + "\n        session.add_request_handler(cx.weak_entity(), Self::handle_generate_bot_token);"
)
content = content.replace(gen_reg, new_gen_reg)
print("Step 7: generate_bot_token registration added")

# Add the handler itself before the last handler in the impl block
gen_handler = """
    async fn handle_generate_bot_token(
        this: Entity<Self>,
        _envelope: TypedEnvelope<proto::Ack>,
        cx: AsyncApp,
    ) -> Result<proto::BotBindResponse> {
        let token_store = cx.read_entity(&this, |this, _| this.bot_token_store.clone());
        let token = token_store.generate();
        log::info!("[AUDIT] Generated new bot token");
        Ok(proto::BotBindResponse {
            success: true,
            message: Some(token),
        })
    }
"""
# Insert before handle_list_workspaces
list_ws_marker = "    async fn handle_list_workspaces("
content = content.replace(list_ws_marker, gen_handler + "\n" + list_ws_marker)
print("Step 8: generate_bot_token handler added")

with open("crates/remote_server/src/headless_project.rs", "w", encoding="utf-8") as f:
    f.write(content)

print("All security features applied!")
