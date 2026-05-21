use crate::api::FeishuApi;
use crate::api::FeishuMessage;
use crate::commands::{self};
use crate::session::SessionManager;
use crate::workspace_client::WorkspaceClient;
use anyhow::Result;
use gpui::AsyncApp;

/// Handles incoming messages from the Feishu platform,
/// routing them to the appropriate command handler.
pub struct MessageHandler {
    session_manager: SessionManager,
    workspace_client: WorkspaceClient,
}

impl MessageHandler {
    /// Create a new MessageHandler.
    pub fn new(workspace_client: WorkspaceClient) -> Self {
        Self {
            session_manager: SessionManager::new(),
            workspace_client,
        }
    }

    /// Handle a text message from a Feishu user.
    ///
    /// If the message is a command (starts with `/`), it will be parsed
    /// and dispatched to the appropriate handler.
    pub async fn handle_text(
        &mut self,
        cx: &mut AsyncApp,
        bot_user_id: &str,
        text: &str,
        bot_api: &FeishuApi,
    ) -> Result<()> {
        let Some(command) = commands::parse_command(text) else {
            let help = HELP_TEXT;
            bot_api
                .send_message(bot_user_id, &FeishuMessage::Text(help.to_string()))
                .await?;
            return Ok(());
        };

        let session = match command.name.as_str() {
            "bind" | "help" => None,
            _ => match self.session_manager.get(bot_user_id) {
                Some(session) => Some(session.clone()),
                None => {
                    bot_api
                        .send_message(
                            bot_user_id,
                            &FeishuMessage::Text(
                                "Bind to workspace first using /bind <token>".to_string(),
                            ),
                        )
                        .await?;
                    return Ok(());
                }
            },
        };

        match command.name.as_str() {
            "bind" => {
                let token = command.args.first().cloned().unwrap_or_default();
                match self
                    .workspace_client
                    .bot_bind(cx, bot_user_id, &token)
                    .await
                {
                    Ok(response) if response.success => {
                        let workspaces = self.workspace_client.list_workspaces(cx).await?;
                        if let Some(first_ws) = workspaces.workspaces.first() {
                            self.session_manager
                                .bind(bot_user_id, &first_ws.workspace_id, 0, 0);
                        }
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(
                                    "Bound successfully! Use /view, /tree, /search, /git, /symbol."
                                        .to_string(),
                                ),
                            )
                            .await?;
                    }
                    Ok(_) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(
                                    "Bind failed: invalid or expired token.".to_string(),
                                ),
                            )
                            .await?;
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Bind error: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "list" => match self.workspace_client.list_workspaces(cx).await {
                Ok(response) => {
                    if response.workspaces.is_empty() {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text("No workspaces available.".to_string()),
                            )
                            .await?;
                    } else {
                        let mut lines: Vec<String> = response
                            .workspaces
                            .iter()
                            .enumerate()
                            .map(|(i, ws)| {
                                let root = ws.root_paths.first().map(|s| s.as_str()).unwrap_or("");
                                format!("  {}. {} ({})", i + 1, ws.name, root)
                            })
                            .collect();
                        lines.insert(0, "Workspaces:".to_string());
                        bot_api
                            .send_message(bot_user_id, &FeishuMessage::Text(lines.join("\n")))
                            .await?;
                    }
                }
                Err(e) => {
                    bot_api
                        .send_message(
                            bot_user_id,
                            &FeishuMessage::Text(format!("Failed to list workspaces: {}", e)),
                        )
                        .await?;
                }
            },

            "view" => {
                let path = command.args.first().cloned().unwrap_or_default();
                let session = session.unwrap();

                let (start_line, end_line) = command
                    .args
                    .get(1)
                    .and_then(|range| {
                        let parts: Vec<&str> = range.split('-').collect();
                        if parts.len() == 2 {
                            Some((Some(parts[0].parse().ok()?), Some(parts[1].parse().ok()?)))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((None, None));

                match self
                    .workspace_client
                    .read_file(
                        cx,
                        session.project_id,
                        session.default_worktree_id,
                        &path,
                        start_line,
                        end_line,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        let header = format!(
                            "{} ({}B, {} lines)\n",
                            path, response.file_size, response.total_lines
                        );
                        let mut message = header;
                        for (i, line) in response.content.lines().enumerate() {
                            let line_num = start_line.map(|s| s as usize + i).unwrap_or(i + 1);
                            message.push_str(&format!("  {:>4} | {}\n", line_num, line));
                        }
                        if response.truncated {
                            message.push_str("\n(File truncated, showing first 100 lines)");
                        }
                        bot_api
                            .send_message(bot_user_id, &FeishuMessage::Text(message))
                            .await?;
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Failed to read file: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "tree" => {
                let path = command
                    .args
                    .first()
                    .cloned()
                    .unwrap_or_else(|| ".".to_string());
                let session = session.unwrap();
                match self
                    .workspace_client
                    .read_directory(
                        cx,
                        session.project_id,
                        session.default_worktree_id,
                        &path,
                        2,
                        200,
                    )
                    .await
                {
                    Ok(response) => {
                        let mut lines = Vec::new();
                        for entry in &response.entries {
                            format_tree_entry(entry, &mut lines, "");
                        }
                        if response.truncated {
                            lines.push("(directory listing truncated)".to_string());
                        }
                        if lines.is_empty() {
                            lines.push("(empty directory)".to_string());
                        }
                        lines.insert(0, format!("{} ({} entries)", path, response.entries.len()));
                        bot_api
                            .send_message(bot_user_id, &FeishuMessage::Text(lines.join("\n")))
                            .await?;
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Failed to read directory: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "search" => {
                let pattern = command.args.first().cloned().unwrap_or_default();
                let session = session.unwrap();
                match self
                    .workspace_client
                    .search_files(cx, session.project_id, &pattern, None, false, false, 20)
                    .await
                {
                    Ok(response) => {
                        if response.matches.is_empty() {
                            bot_api
                                .send_message(
                                    bot_user_id,
                                    &FeishuMessage::Text(format!("No results for '{}'", pattern)),
                                )
                                .await?;
                        } else {
                            let mut lines = vec![format!(
                                "Search '{}': {} results\n",
                                pattern,
                                response.matches.len()
                            )];
                            for m in &response.matches {
                                lines.push(format!(
                                    "  {}:{} - {}",
                                    m.path, m.line_number, m.line_content
                                ));
                            }
                            bot_api
                                .send_message(bot_user_id, &FeishuMessage::Text(lines.join("\n")))
                                .await?;
                        }
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Search failed: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "symbol" => {
                let name = command.args.first().cloned().unwrap_or_default();
                let session = session.unwrap();
                match self
                    .workspace_client
                    .search_symbols(cx, session.project_id, &name, 30)
                    .await
                {
                    Ok(response) => {
                        if response.symbols.is_empty() {
                            bot_api
                                .send_message(
                                    bot_user_id,
                                    &FeishuMessage::Text(format!(
                                        "No symbols found for '{}'",
                                        name
                                    )),
                                )
                                .await?;
                        } else {
                            let mut lines = vec![format!(
                                "Symbol search '{}': {} results\n",
                                name,
                                response.symbols.len()
                            )];
                            for sym in &response.symbols {
                                let container = sym
                                    .container_name
                                    .as_ref()
                                    .map(|c| format!(" (in {})", c))
                                    .unwrap_or_default();
                                lines.push(format!(
                                    "  {} {}{} - {}:{}",
                                    sym.kind, sym.name, container, sym.path, sym.line
                                ));
                            }
                            bot_api
                                .send_message(bot_user_id, &FeishuMessage::Text(lines.join("\n")))
                                .await?;
                        }
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Symbol search failed: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "git" => {
                let session = session.unwrap();
                match self
                    .workspace_client
                    .git_status(cx, session.project_id)
                    .await
                {
                    Ok(response) => {
                        if response.changes.is_empty() {
                            bot_api
                                .send_message(
                                    bot_user_id,
                                    &FeishuMessage::Text("Git: no changes.".to_string()),
                                )
                                .await?;
                        } else {
                            let mut lines = vec!["Git status:\n".to_string()];
                            if !response.branch.is_empty() {
                                lines.push(format!("Branch: {}\n", response.branch));
                            }
                            for change in &response.changes {
                                lines.push(format!("  [{}] {}", change.status, change.path));
                            }
                            bot_api
                                .send_message(bot_user_id, &FeishuMessage::Text(lines.join("\n")))
                                .await?;
                        }
                    }
                    Err(e) => {
                        bot_api
                            .send_message(
                                bot_user_id,
                                &FeishuMessage::Text(format!("Git status failed: {}", e)),
                            )
                            .await?;
                    }
                }
            }

            "help" => {
                bot_api
                    .send_message(bot_user_id, &FeishuMessage::Text(HELP_TEXT.to_string()))
                    .await?;
            }

            "prompt" => {
                let prompt_text = command.args.join(" ");
                if prompt_text.is_empty() {
                    bot_api
                        .send_message(
                            bot_user_id,
                            &FeishuMessage::Text("Usage: /prompt <message>".to_string()),
                        )
                        .await?;
                } else {
                    let session = session.unwrap();
                    bot_api
                        .send_message(
                            bot_user_id,
                            &FeishuMessage::Text(format!("Thinking...\n\n> {}", prompt_text)),
                        )
                        .await?;
                    match self
                        .workspace_client
                        .agent_prompt(cx, session.project_id, &prompt_text)
                        .await
                    {
                        Ok(response) => {
                            let mut output = response.text;
                            if output.is_empty() {
                                output =
                                    format!("(no response, stop reason: {})", response.stop_reason);
                            }
                            bot_api
                                .send_message(bot_user_id, &FeishuMessage::Text(output))
                                .await?;
                        }
                        Err(e) => {
                            bot_api
                                .send_message(
                                    bot_user_id,
                                    &FeishuMessage::Text(format!("Agent prompt failed: {}", e)),
                                )
                                .await?;
                        }
                    }
                }
            }

            "run" => {
                let command_line = command.args.join(" ");
                if command_line.is_empty() {
                    bot_api
                        .send_message(
                            bot_user_id,
                            &FeishuMessage::Text("Usage: /run <command>".to_string()),
                        )
                        .await?;
                } else {
                    let session = session.unwrap();
                    match self
                        .workspace_client
                        .run_command(cx, session.project_id, &command_line, &[], None, 30)
                        .await
                    {
                        Ok(response) => {
                            let mut output = String::new();
                            if !response.stdout.is_empty() {
                                output.push_str(&response.stdout);
                            }
                            if !response.stderr.is_empty() {
                                if !output.is_empty() {
                                    output.push('\n');
                                }
                                output.push_str("STDERR:\n");
                                output.push_str(&response.stderr);
                            }
                            if output.is_empty() {
                                output = format!("(exit code: {})", response.exit_code);
                            }
                            if response.timed_out {
                                output.push_str(
                                    "
(command timed out)",
                                );
                            }
                            bot_api
                                .send_message(bot_user_id, &FeishuMessage::Text(output))
                                .await?;
                        }
                        Err(e) => {
                            bot_api
                                .send_message(
                                    bot_user_id,
                                    &FeishuMessage::Text(format!("Run failed: {}", e)),
                                )
                                .await?;
                        }
                    }
                }
            }

            _ => {
                bot_api
                    .send_message(
                        bot_user_id,
                        &FeishuMessage::Text(format!(
                            "Unknown command: /{}\nType /help for available commands.",
                            command.name
                        )),
                    )
                    .await?;
            }
        }

        Ok(())
    }
}

const HELP_TEXT: &str = "\
Zed Bot commands:
/bind <token> - Bind to workspace
/list - List available workspaces
/view <path> [L1-L2] - View file contents
/tree <path> - View directory structure
/search <pattern> - Search files
/symbol <name> - Search symbols
/git - Git status
/run <command> - Run a shell command
/prompt <message> - Ask the AI agent
/help - Show this help";

fn format_tree_entry(entry: &proto::DirectoryEntry, lines: &mut Vec<String>, prefix: &str) {
    let icon = if entry.is_directory { "[D]" } else { "[F]" };
    let size_str = entry
        .file_size
        .map(|s| format!(" ({}B)", s))
        .unwrap_or_default();
    let is_last = true; // simplified; real impl would track position
    let connector = if prefix.is_empty() {
        if is_last { "L " } else { "| " }
    } else if is_last {
        "  L "
    } else {
        "  | "
    };

    lines.push(format!(
        "{}{}{} {}{}",
        prefix, connector, icon, entry.name, size_str
    ));

    if !entry.children.is_empty() {
        let child_prefix = if prefix.is_empty() {
            if is_last { "  " } else { "| " }
        } else if is_last {
            "    "
        } else {
            "  | "
        };
        for child in &entry.children {
            format_tree_entry(child, lines, child_prefix);
        }
    }
}
