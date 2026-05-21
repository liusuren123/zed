use anyhow::{Context, Result};
use gpui::{AppContext as _, AsyncApp, Entity};
use proto::{
    AgentPromptRequest, AgentPromptResponse, BotBindRequest, BotBindResponse,
    GenerateBotTokenRequest, GenerateBotTokenResponse, GitStatusRequest, GitStatusResponse,
    ListWorkspacesRequest, ListWorkspacesResponse, ReadDirectory, ReadDirectoryResponse, ReadFile,
    ReadFileResponse, RunCommandRequest, RunCommandResponse, SearchFilesRequest,
    SearchFilesResponse, SearchSymbolsRequest, SearchSymbolsResponse,
};
use remote_server::HeadlessProject;
use remote_server::headless_project::collect_directory_entries;
use rpc::AnyProtoClient;

enum Backend {
    /// RPC mode: communicates with a remote_server process via AnyProtoClient.
    Rpc(AnyProtoClient),
    /// Local mode: directly accesses HeadlessProject within the same process.
    Local(Entity<HeadlessProject>),
}

/// Client for communicating with a Zed workspace, either via RPC or locally.
pub struct WorkspaceClient {
    backend: Backend,
}

impl WorkspaceClient {
    /// Create a client that communicates via RPC (for separate process).
    pub fn new_rpc(client: AnyProtoClient) -> Self {
        Self {
            backend: Backend::Rpc(client),
        }
    }

    /// Create a client that directly accesses the project (for same process).
    pub fn new_local(project: Entity<HeadlessProject>) -> Self {
        Self {
            backend: Backend::Local(project),
        }
    }

    /// Read a file from the workspace.
    pub async fn read_file(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
        worktree_id: u64,
        path: &str,
        start_line: Option<u64>,
        end_line: Option<u64>,
        max_bytes: Option<u64>,
    ) -> Result<ReadFileResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(ReadFile {
                    project_id,
                    worktree_id,
                    path: path.to_string(),
                    start_line,
                    end_line,
                    max_bytes,
                })
                .await
                .context("read_file RPC failed"),
            Backend::Local(project) => {
                let fs = cx.read_entity(project, |project, _| project.fs.clone());
                let abs_path = shellexpand::tilde(path).to_string();
                let abs_path = std::path::Path::new(&abs_path);
                const MAX_FILE_SIZE: u64 = 524288;
                let max_bytes = max_bytes.unwrap_or(MAX_FILE_SIZE);

                let content = fs.load(abs_path).await?;
                let total_lines = content.lines().count() as u64;

                let metadata = fs.metadata(abs_path).await?;
                let file_size = metadata
                    .as_ref()
                    .map(|m| m.len)
                    .unwrap_or(content.len() as u64);
                let modified_at = metadata
                    .and_then(|m| m.mtime.to_seconds_and_nanos_for_persistence())
                    .map(|(secs, _nanos)| secs)
                    .unwrap_or(0);

                let (result_content, truncated) = if content.len() as u64 > max_bytes {
                    let truncated_content: String =
                        content.lines().take(100).collect::<Vec<&str>>().join("\n");
                    (truncated_content, true)
                } else {
                    let start_line = start_line.unwrap_or(1).saturating_sub(1);
                    let end_line = end_line.unwrap_or(total_lines);
                    let text: String = content
                        .lines()
                        .skip(start_line as usize)
                        .take(end_line.saturating_sub(start_line) as usize)
                        .collect::<Vec<&str>>()
                        .join("\n");
                    (text, false)
                };

                Ok(ReadFileResponse {
                    content: result_content,
                    total_lines,
                    file_size,
                    modified_at,
                    truncated,
                })
            }
        }
    }

    /// Read a directory listing from the workspace.
    pub async fn read_directory(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
        worktree_id: u64,
        path: &str,
        depth: u32,
        max_entries: u32,
    ) -> Result<ReadDirectoryResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(ReadDirectory {
                    project_id,
                    worktree_id,
                    path: path.to_string(),
                    depth,
                    max_entries,
                })
                .await
                .context("read_directory RPC failed"),
            Backend::Local(project) => {
                let fs = cx.read_entity(project, |project, _| project.fs.clone());
                let abs_path = shellexpand::tilde(path).to_string();
                let abs_path = std::path::Path::new(&abs_path);
                let depth = depth.max(1);
                let max_entries = if max_entries == 0 { 200 } else { max_entries } as usize;

                let entries = collect_directory_entries(&fs, abs_path, depth).await?;
                let truncated = entries.len() >= max_entries;
                let entries = entries.into_iter().take(max_entries).collect::<Vec<_>>();

                Ok(ReadDirectoryResponse { entries, truncated })
            }
        }
    }

    /// Search for files containing a pattern in the workspace.
    pub async fn search_files(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
        pattern: &str,
        path: Option<&str>,
        is_regex: bool,
        case_sensitive: bool,
        max_results: u32,
    ) -> Result<SearchFilesResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(SearchFilesRequest {
                    project_id,
                    pattern: pattern.to_string(),
                    path: path.map(|s| s.to_string()),
                    is_regex,
                    case_sensitive,
                    max_results,
                })
                .await
                .context("search_files RPC failed"),
            Backend::Local(project) => {
                let max_results = max_results.max(1).min(50) as usize;
                let path_filter = path.map(|p| p.to_lowercase());
                let fs = cx.read_entity(project, |project, _| project.fs.clone());
                let file_paths = cx.read_entity(project, |project, cx| {
                    let mut paths: Vec<(u64, String)> = Vec::new();
                    for worktree in project.worktree_store.read(cx).visible_worktrees(cx) {
                        let snapshot = worktree.read(cx).snapshot();
                        let worktree_id = snapshot.id().to_usize() as u64;
                        for entry in snapshot.files(false, 0) {
                            let abs_path = snapshot.absolutize(&entry.path);
                            let path_str = abs_path.display().to_string();
                            if let Some(ref filter) = path_filter {
                                if !path_str.to_lowercase().contains(filter) {
                                    continue;
                                }
                            }
                            paths.push((worktree_id, path_str));
                        }
                    }
                    paths
                });
                let mut matches = Vec::new();
                for (worktree_id, file_path) in &file_paths {
                    if matches.len() >= max_results {
                        break;
                    }
                    let content = match fs.load(std::path::Path::new(file_path)).await {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if content.len() > 524288 {
                        continue;
                    }
                    for (line_idx, line) in content.lines().enumerate() {
                        if matches.len() >= max_results {
                            break;
                        }
                        let found = if is_regex {
                            regex::Regex::new(pattern)
                                .map(|r| r.is_match(line))
                                .unwrap_or(false)
                        } else if case_sensitive {
                            line.contains(pattern)
                        } else {
                            line.to_lowercase().contains(&pattern.to_lowercase())
                        };
                        if found {
                            let match_start = if case_sensitive {
                                line.find(pattern).unwrap_or(0)
                            } else {
                                line.to_lowercase()
                                    .find(&pattern.to_lowercase())
                                    .unwrap_or(0)
                            } as u64;
                            matches.push(proto::FileMatch {
                                worktree_id: *worktree_id,
                                path: file_path.clone(),
                                line_number: line_idx as u64 + 1,
                                line_content: line.to_string(),
                                match_start,
                                match_end: match_start + pattern.len() as u64,
                            });
                        }
                    }
                }
                let truncated = matches.len() >= max_results;
                log::info!(
                    "[AUDIT] Search files (local): pattern={} results={}",
                    pattern,
                    matches.len()
                );
                Ok(SearchFilesResponse { matches, truncated })
            }
        }
    }

    /// Search for symbols in the workspace.
    pub async fn search_symbols(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
        query: &str,
        max_results: u32,
    ) -> Result<SearchSymbolsResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(SearchSymbolsRequest {
                    project_id,
                    query: query.to_string(),
                    max_results,
                })
                .await
                .context("search_symbols RPC failed"),
            Backend::Local(project) => {
                let max_results = max_results.max(1).min(50) as usize;
                let query_lower = query.to_lowercase();
                let fs = cx.read_entity(project, |project, _| project.fs.clone());
                let file_paths = cx.read_entity(project, |project, cx| {
                    let mut paths: Vec<String> = Vec::new();
                    for worktree in project.worktree_store.read(cx).visible_worktrees(cx) {
                        let snapshot = worktree.read(cx).snapshot();
                        for entry in snapshot.files(false, 0) {
                            let abs_path = snapshot.absolutize(&entry.path);
                            paths.push(abs_path.display().to_string());
                        }
                    }
                    paths
                });
                let def_patterns = [
                    ("fn ", "function"),
                    ("pub fn ", "function"),
                    ("class ", "class"),
                    ("struct ", "struct"),
                    ("impl ", "impl"),
                    ("trait ", "trait"),
                    ("enum ", "enum"),
                    ("def ", "function"),
                    ("func ", "function"),
                    ("function ", "function"),
                    ("const ", "constant"),
                    ("let ", "variable"),
                    ("var ", "variable"),
                    ("type ", "type"),
                    ("interface ", "interface"),
                ];
                let mut symbols = Vec::new();
                for file_path in &file_paths {
                    if symbols.len() >= max_results {
                        break;
                    }
                    let content = match fs.load(std::path::Path::new(file_path)).await {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if content.len() > 524288 {
                        continue;
                    }
                    for (line_idx, line) in content.lines().enumerate() {
                        if symbols.len() >= max_results {
                            break;
                        }
                        let line_lower = line.to_lowercase();
                        if !line_lower.contains(&query_lower) {
                            continue;
                        }
                        for (def_prefix, kind) in &def_patterns {
                            if line_lower.contains(def_prefix) {
                                let name = line
                                    .trim()
                                    .split_whitespace()
                                    .nth(1)
                                    .unwrap_or("")
                                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                                if !name.is_empty() && name.to_lowercase().contains(&query_lower) {
                                    symbols.push(proto::SymbolInfo {
                                        name: name.to_string(),
                                        kind: kind.to_string(),
                                        path: file_path.clone(),
                                        line: line_idx as u64 + 1,
                                        column: 0,
                                        container_name: None,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
                log::info!(
                    "[AUDIT] Search symbols (local): query={} results={}",
                    query,
                    symbols.len()
                );
                Ok(SearchSymbolsResponse { symbols })
            }
        }
    }

    /// Get Git status from the workspace.
    pub async fn git_status(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
    ) -> Result<GitStatusResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(GitStatusRequest { project_id })
                .await
                .context("git_status RPC failed"),
            Backend::Local(project) => {
                let (branch, changes) = cx.read_entity(project, |project, cx| {
                    let repos = project.git_store.read(cx).repositories().clone();
                    let mut branch = String::new();
                    let mut changes = Vec::new();
                    for (_repo_id, repo) in repos.iter() {
                        let snapshot = repo.read(cx).snapshot();
                        if branch.is_empty() {
                            if let Some(b) = &snapshot.branch {
                                branch = b.ref_name.to_string();
                            }
                        }
                        for entry in snapshot.status() {
                            let status_str = if entry.status.is_conflicted() {
                                "C"
                            } else if entry.status.is_deleted() {
                                "D"
                            } else if entry.status.is_created() {
                                "A"
                            } else if entry.status.is_modified() {
                                "M"
                            } else if entry.status.is_untracked() {
                                "?"
                            } else {
                                " "
                            };
                            changes.push(proto::GitFileChange {
                                path: entry.repo_path.as_std_path().display().to_string(),
                                status: status_str.to_string(),
                            });
                        }
                    }
                    (branch, changes)
                });
                log::info!(
                    "[AUDIT] Git status (local): branch={} changes={}",
                    branch,
                    changes.len()
                );
                Ok(GitStatusResponse {
                    branch,
                    changes,
                    ahead: 0,
                    behind: 0,
                })
            }
        }
    }

    /// Run a shell command and return its output.
    pub async fn run_command(
        &self,
        cx: &mut AsyncApp,
        project_id: u64,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        timeout_secs: u32,
    ) -> Result<RunCommandResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(RunCommandRequest {
                    project_id,
                    command: command.to_string(),
                    args: args.to_vec(),
                    cwd: cwd.map(|s| s.to_string()),
                    timeout_secs,
                })
                .await
                .context("run_command RPC failed"),
            Backend::Local(_project) => {
                let timeout = std::time::Duration::from_secs(timeout_secs.max(1).min(300) as u64);
                let mut cmd = smol::process::Command::new(command);
                cmd.args(args);
                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());

                let output = smol::future::or(
                    async {
                        let child = cmd.spawn()?;
                        child.output().await
                    },
                    async {
                        smol::Timer::after(timeout).await;
                        Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "command timed out",
                        ))
                    },
                )
                .await;

                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let exit_code = output.status.code().unwrap_or(-1);
                        log::info!(
                            "[AUDIT] Run command (local): cmd={} exit={}",
                            command,
                            exit_code
                        );
                        Ok(RunCommandResponse {
                            exit_code,
                            stdout,
                            stderr,
                            timed_out: false,
                        })
                    }
                    Err(e) => {
                        let timed_out = e.kind() == std::io::ErrorKind::TimedOut;
                        log::warn!("[AUDIT] Run command (local) failed: {}", e);
                        Ok(RunCommandResponse {
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: e.to_string(),
                            timed_out,
                        })
                    }
                }
            }
        }
    }

    /// Bind a Feishu bot user to the workspace.
    pub async fn bot_bind(
        &self,
        _cx: &mut AsyncApp,
        bot_user_id: &str,
        access_token: &str,
    ) -> Result<BotBindResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(BotBindRequest {
                    bot_user_id: bot_user_id.to_string(),
                    access_token: access_token.to_string(),
                })
                .await
                .context("bot_bind RPC failed"),
            Backend::Local(_) => {
                let valid = remote_server::headless_project::BOT_TOKENS
                    .lock()
                    .unwrap()
                    .remove(access_token);
                if valid {
                    log::info!("[AUDIT] Bot bind (local): user={} bound", bot_user_id);
                    Ok(BotBindResponse {
                        success: true,
                        message: None,
                    })
                } else {
                    log::warn!(
                        "[AUDIT] Bot bind (local): user={} invalid token",
                        bot_user_id
                    );
                    Ok(BotBindResponse {
                        success: false,
                        message: Some("Invalid or expired token".to_string()),
                    })
                }
            }
        }
    }

    /// List available workspaces.
    pub async fn list_workspaces(&self, cx: &mut AsyncApp) -> Result<ListWorkspacesResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(ListWorkspacesRequest {})
                .await
                .context("list_workspaces RPC failed"),
            Backend::Local(project) => {
                let worktrees = cx.read_entity(project, |project, cx| {
                    project
                        .worktree_store
                        .read(cx)
                        .visible_worktrees(cx)
                        .map(|w| {
                            let snapshot = w.read(cx).snapshot();
                            (
                                snapshot.abs_path().to_path_buf(),
                                snapshot.root_name_str().to_string(),
                            )
                        })
                        .collect::<Vec<_>>()
                });

                let root_paths: Vec<String> = worktrees
                    .iter()
                    .map(|(abs_path, _)| abs_path.display().to_string())
                    .collect();

                let name = worktrees
                    .first()
                    .map(|(_, name)| name.clone())
                    .unwrap_or_else(|| "workspace".to_string());

                let workspaces = if root_paths.is_empty() {
                    Vec::new()
                } else {
                    vec![proto::WorkspaceInfo {
                        workspace_id: name.clone(),
                        name,
                        host: "localhost".to_string(),
                        root_paths,
                    }]
                };

                log::info!(
                    "[AUDIT] List workspaces (local): {} workspace(s)",
                    workspaces.len()
                );
                Ok(ListWorkspacesResponse { workspaces })
            }
        }
    }

    /// Generate a new bot access token.
    pub async fn generate_bot_token(&self, _cx: &mut AsyncApp) -> Result<GenerateBotTokenResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(GenerateBotTokenRequest {})
                .await
                .context("generate_bot_token RPC failed"),
            Backend::Local(_) => {
                let token = uuid::Uuid::new_v4().to_string();
                remote_server::headless_project::BOT_TOKENS
                    .lock()
                    .unwrap()
                    .insert(token.clone());
                log::info!("[AUDIT] Generated bot token (local): {}", token);
                Ok(GenerateBotTokenResponse { token })
            }
        }
    }

    /// Send a prompt to the AI agent and get the response.
    pub async fn agent_prompt(
        &self,
        _cx: &mut AsyncApp,
        project_id: u64,
        prompt: &str,
    ) -> Result<AgentPromptResponse> {
        match &self.backend {
            Backend::Rpc(client) => client
                .request(AgentPromptRequest {
                    project_id,
                    prompt: prompt.to_string(),
                })
                .await
                .context("agent_prompt RPC failed"),
            Backend::Local(_) => Err(anyhow::anyhow!(
                "Agent prompt requires RPC mode. Run in remote_server mode."
            )),
        }
    }
}
