use agent_client_protocol::schema as acp;
use futures::FutureExt as _;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::{AgentTool, ToolCallEventStream, ToolInput};

const PICK_TIMEOUT: Duration = Duration::from_secs(30);

/// Opens the GPUI element inspector in pick mode and waits for the user to click a UI element.
///
/// When the user clicks a UI element, this tool returns the element's source code location
/// (file, line, column), its global element ID, and instance ID. The agent can use this
/// information to locate the UI code that renders the clicked element.
///
/// This is useful when the user describes a UI they want to change but cannot specify
/// exactly where the code lives. After receiving the result, use `read_file` and `grep`
/// to find and understand the relevant code.
///
/// Only available in debug builds of Zed.
///
/// <example>
/// {}
/// </example>
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InspectElementToolInput {}

/// Result of an element inspection pick.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InspectElementToolOutput {
    /// The source file path of the element's construction site, relative to the repo root.
    pub source_file: String,
    /// The source line number (1-based).
    pub source_line: u32,
    /// The source column number (1-based).
    pub source_column: u32,
    /// The nearest ancestor element's GlobalElementId. Can be used to find related code.
    pub global_element_id: String,
    /// Instance discriminator for elements from the same source location.
    pub instance_id: usize,
}

pub struct InspectElementTool;

impl InspectElementTool {
    pub fn new() -> Self {
        Self
    }
}

impl AgentTool for InspectElementTool {
    type Input = InspectElementToolInput;
    type Output = InspectElementToolOutput;

    const NAME: &'static str = "inspect_element";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Read
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Click a UI element to inspect...".into()
    }

    fn run(
        self: Arc<Self>,
        _input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        #[cfg(any(feature = "inspector", debug_assertions))]
        {
            let active_window = cx.active_window();
            let enabled = active_window
                .and_then(|window| {
                    window
                        .update(cx, |_, window, cx| {
                            window.start_inspector_picking(cx);
                            true
                        })
                        .ok()
                })
                .unwrap_or(false);

            let (tx, rx) = futures::channel::oneshot::channel();
            if enabled {
                cx.set_global(gpui::PendingInspectorPick { sender: Some(tx) });
            }

            cx.spawn(async move |cx| {
                if !enabled {
                    return Err(InspectElementToolOutput::cannot_inspect(
                        "No active window available. Make sure a Zed window is focused.",
                    ));
                }

                let inspector_id = futures::select! {
                    result = rx.fuse() => {
                        match result {
                            Ok(id) => id,
                            Err(_canceled) => {
                                return Err(InspectElementToolOutput::cannot_inspect(
                                    "Element pick was cancelled — the inspector may have been closed.",
                                ));
                            }
                        }
                    }
                    _ = cx.background_executor().timer(PICK_TIMEOUT).fuse() => {
                        return Err(InspectElementToolOutput::cannot_inspect(
                            "Timed out waiting for element pick (30s). The user did not click a UI element.",
                        ));
                    }
                    _ = event_stream.cancelled_by_user().fuse() => {
                        return Err(InspectElementToolOutput::cannot_inspect(
                            "Inspect element cancelled by user.",
                        ));
                    }
                };

                // Cleanup: close inspector and remove global.
                cx.update(|cx| {
                    if cx.has_global::<gpui::PendingInspectorPick>() {
                        cx.remove_global::<gpui::PendingInspectorPick>();
                    }
                    if let Some(window) = cx.active_window() {
                        let _ = window.update(cx, |_, window, cx| {
                            if window.is_inspector_picking(cx) {
                                window.toggle_inspector(cx);
                            }
                        });
                    }
                });

                let source_location = inspector_id.path.source_location;
                let source_file = strip_repo_prefix(source_location.file());
                let global_element_id = inspector_id.path.global_id.to_string();

                Ok(InspectElementToolOutput {
                    source_file,
                    source_line: source_location.line(),
                    source_column: source_location.column(),
                    global_element_id,
                    instance_id: inspector_id.instance_id,
                })
            })
        }

        #[cfg(not(any(feature = "inspector", debug_assertions)))]
        {
            cx.spawn(async move |_cx| {
                Err(InspectElementToolOutput::cannot_inspect(
                    "The element inspector is only available in debug builds of Zed.",
                ))
            })
        }
    }
}

fn strip_repo_prefix(path: &str) -> String {
    if let Some(repo_dir) = option_env!("ZED_REPO_DIR") {
        if let Some(stripped) = path.strip_prefix(repo_dir) {
            return stripped.trim_start_matches('/').to_string();
        }
    }
    path.to_string()
}

impl From<InspectElementToolOutput> for LanguageModelToolResultContent {
    fn from(output: InspectElementToolOutput) -> Self {
        use std::fmt::Write;
        let mut text = String::new();
        if output.source_file.is_empty() {
            // Error case.
            let _ = write!(text, "{}", output.global_element_id);
        } else {
            let _ = write!(
                text,
                "Element located at {}:{}:{} (instance {})\nGlobal ID: {}",
                output.source_file,
                output.source_line,
                output.source_column,
                output.instance_id,
                output.global_element_id,
            );
        }
        LanguageModelToolResultContent::Text(text.into())
    }
}

impl InspectElementToolOutput {
    fn cannot_inspect(reason: &str) -> Self {
        Self {
            source_file: String::new(),
            source_line: 0,
            source_column: 0,
            global_element_id: format!("Could not inspect element: {reason}"),
            instance_id: 0,
        }
    }
}
