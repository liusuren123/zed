use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Represents a bound session between a Feishu user and a Zed workspace.
#[derive(Clone, Debug)]
pub struct Session {
    /// The Feishu user identifier (open_id or user_id).
    pub bot_user_id: String,
    /// The workspace identifier this user is bound to.
    pub workspace_id: String,
    /// The RPC project ID on the remote server.
    pub project_id: u64,
    /// Default worktree ID for path lookups (0 means use raw filesystem paths).
    pub default_worktree_id: u64,
    /// When the session was created.
    pub created_at: Instant,
    /// Session expiry duration.
    pub ttl: Duration,
}

impl Session {
    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Manages active user-to-workspace sessions.
pub struct SessionManager {
    /// Active sessions, keyed by bot_user_id.
    sessions: HashMap<String, Session>,
    /// Default session TTL.
    default_ttl: Duration,
}

impl SessionManager {
    /// Create a new SessionManager with the default TTL of 24 hours.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            default_ttl: Duration::from_secs(86400), // 24 hours
        }
    }

    /// Bind a Feishu user to a workspace, creating a new session.
    pub fn bind(
        &mut self,
        bot_user_id: &str,
        workspace_id: &str,
        project_id: u64,
        default_worktree_id: u64,
    ) -> Session {
        let session = Session {
            bot_user_id: bot_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            project_id,
            default_worktree_id,
            created_at: Instant::now(),
            ttl: self.default_ttl,
        };
        self.sessions
            .insert(bot_user_id.to_string(), session.clone());
        log::info!("Bound user {} to workspace {}", bot_user_id, workspace_id);
        session
    }

    /// Look up the session for a given Feishu user.
    pub fn get(&self, bot_user_id: &str) -> Option<&Session> {
        self.sessions.get(bot_user_id)
    }

    /// Remove expired sessions.
    pub fn cleanup_expired(&mut self) {
        self.sessions.retain(|_, session| !session.is_expired());
    }

    /// List all active workspace IDs.
    pub fn list_workspaces(&self) -> Vec<String> {
        let mut workspace_ids: Vec<String> = self
            .sessions
            .values()
            .map(|s| s.workspace_id.clone())
            .collect();
        workspace_ids.sort();
        workspace_ids.dedup();
        workspace_ids
    }
}
