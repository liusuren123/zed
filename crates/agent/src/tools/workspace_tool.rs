use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use anyhow::Result;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Query workspace-related information and send messages between open workspaces.
///
/// This tool enables the agent to discover other open workspaces in the current window,
/// understand their status, and send inter-workspace messages.
///
/// Actions:
/// - `list`: List all open workspaces with their root paths, active item, and agent status.
/// - `send_message`: Send a message to another workspace's agent conversation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum WorkspaceToolInput {
    /// List all open workspaces with their current state.
    List,
    /// Send a message to another workspace's agent conversation.
    SendMessage {
        /// The workspace identifier (as shown in the list_workspaces output).
        workspace_id: String,
        /// The message content to send to the other workspace's agent.
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceToolOutput {
    List {
        workspaces: Vec<WorkspaceInfo>,
    },
    SendMessage {
        workspace_id: String,
        success: bool,
        error: Option<String>,
    },
    Error {
        error: String,
    },
}

/// Information about a single open workspace.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInfo {
    /// Unique identifier for the workspace
    pub id: String,
    /// Root paths of the project in this workspace
    pub paths: Vec<String>,
    /// Path of the currently focused file/item, if any
    pub active_item_path: Option<String>,
    /// Current status of the agent in this workspace (e.g., "idle", "working", "no agent panel")
    pub agent_status: String,
    /// Whether this is the currently active workspace in the window
    pub is_active: bool,
    /// Title of the active agent conversation thread, if any
    pub active_thread_title: Option<String>,
}

/// Global registry storing snapshots of all open workspaces.
/// Updated from the agent_ui layer whenever workspace state changes.
#[derive(Clone, Default)]
pub struct WorkspaceRegistry {
    workspaces: Vec<WorkspaceInfo>,
}

impl WorkspaceRegistry {
    pub fn update(&mut self, workspaces: Vec<WorkspaceInfo>) {
        self.workspaces = workspaces;
    }

    pub fn list(&self) -> Vec<WorkspaceInfo> {
        self.workspaces.clone()
    }

    pub fn find(&self, id: &str) -> Option<WorkspaceInfo> {
        self.workspaces.iter().find(|w| w.id == id).cloned()
    }
}

impl From<WorkspaceToolOutput> for LanguageModelToolResultContent {
    fn from(output: WorkspaceToolOutput) -> Self {
        match output {
            WorkspaceToolOutput::List { workspaces } => {
                if workspaces.is_empty() {
                    "No other open workspaces found in this window.".into()
                } else {
                    serde_json::to_string(&serde_json::json!({
                        "workspaces": workspaces,
                    }))
                    .unwrap_or_else(|e| format!("Failed to serialize workspace list: {e}"))
                    .into()
                }
            }
            WorkspaceToolOutput::SendMessage {
                workspace_id,
                success,
                error,
            } => {
                if success {
                    format!("Message sent to workspace {workspace_id} successfully.").into()
                } else {
                    format!(
                        "Failed to send message to workspace {workspace_id}: {}",
                        error.as_deref().unwrap_or("unknown error")
                    )
                    .into()
                }
            }
            WorkspaceToolOutput::Error { error } => format!("Error: {error}").into(),
        }
    }
}

/// Tool for cross-workspace awareness and communication.
pub struct WorkspaceTool {
    registry: Arc<std::sync::Mutex<WorkspaceRegistry>>,
    /// Callback to send a message to a workspace's agent panel.
    send_message_fn:
        Arc<dyn Fn(String, String, &mut App) -> Task<Result<()>> + Send + Sync + 'static>,
    /// Callback to refresh workspace snapshots before listing.
    refresh_fn: Arc<dyn Fn(&mut App) + Send + Sync + 'static>,
}

impl WorkspaceTool {
    pub fn new(
        registry: Arc<std::sync::Mutex<WorkspaceRegistry>>,
        send_message_fn: Arc<
            dyn Fn(String, String, &mut App) -> Task<Result<()>> + Send + Sync + 'static,
        >,
        refresh_fn: Arc<dyn Fn(&mut App) + Send + Sync + 'static>,
    ) -> Self {
        Self {
            registry,
            send_message_fn,
            refresh_fn,
        }
    }
}

impl AgentTool for WorkspaceTool {
    type Input = WorkspaceToolInput;
    type Output = WorkspaceToolOutput;

    const NAME: &'static str = "workspace_tool";

    fn description() -> SharedString {
        "Discover other open workspaces, check their status, and send messages between them.".into()
    }

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(WorkspaceToolInput::List) => "Listing workspaces…".into(),
            Ok(WorkspaceToolInput::SendMessage { workspace_id, .. }) => {
                format!("Sending message to {workspace_id}…").into()
            }
            Err(_) => "Workspace operation".into(),
        }
    }

    fn supports_provider(_provider: &language_model::LanguageModelProviderId) -> bool {
        true
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        _event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let registry = self.registry.clone();
        let send_message_fn = self.send_message_fn.clone();
        let refresh_fn = self.refresh_fn.clone();

        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|e| WorkspaceToolOutput::Error {
                error: e.to_string(),
            })?;

            match input {
                WorkspaceToolInput::List => {
                    // Refresh snapshots from the UI layer
                    cx.update(|cx| refresh_fn(cx));
                    let workspaces = {
                        let registry = registry.lock().unwrap();
                        registry.list()
                    };
                    Ok(WorkspaceToolOutput::List { workspaces })
                }
                WorkspaceToolInput::SendMessage {
                    workspace_id,
                    message,
                } => {
                    let task = cx.update(|cx| send_message_fn(workspace_id.clone(), message, cx));
                    let result = task.await;
                    match result {
                        Ok(()) => Ok(WorkspaceToolOutput::SendMessage {
                            workspace_id,
                            success: true,
                            error: None,
                        }),
                        Err(err) => Ok(WorkspaceToolOutput::SendMessage {
                            workspace_id,
                            success: false,
                            error: Some(err.to_string()),
                        }),
                    }
                }
            }
        })
    }
}
