# 飞书远程控制 Zed 技术方案

## 1. 项目概述

### 1.1 目标

实现通过飞书（Feishu/Lark）机器人远程连接 Zed 工作区，实现**只读代码查看**能力。用户无需在本地打开 Zed，即可通过飞书发送命令来浏览项目代码、搜索符号、查看目录结构等。

### 1.2 范围

- **第一阶段**：飞书 Bot 网关 + 远程只读文件查看核心能力
- **第二阶段**：目录浏览、符号搜索、Git 状态查询
- **第三阶段**：交互卡片、多工作区管理、认证系统完善
- **未来**：钉钉/企微/Slack 等多 Bot 支持、写入支持、终端访问

---

## 2. 现有基础设施分析

Zed 已具备大量可复用的远程协作能力：

| 组件 | 路径 | 作用 | 本方案如何复用 |
|---|---|---|---|
| `remote_server` | `crates/remote_server/` | 无头 Zed 服务端，管理项目、缓冲区、LSP、DAP | **核心依赖**：在其上新增只读 API |
| `remote` | `crates/remote/` | SSH/Docker/WSL远程连接、协议编解码 | 复用 `read_message`/`write_message` 协议层 |
| `rpc` | `crates/rpc/` | Protobuf 消息协议、`Envelope`、`Peer`、连接管理 | 新增消息类型，复用编解码框架 |
| `client` | `crates/client/` | 客户端连接管理 | 参考其 RPC 客户端模式 |
| `collab` | `crates/collab/` | 协作服务端（用户管理、房间） | 可选的中间层（HTTP→RPC 桥接） |
| `proto` | `crates/proto/proto/` | Protobuf 消息定义 | 新增 `remote_control.proto` |
| `project` | `crates/project/` | 项目管理、Worktree/Buffer/LSP 集成 | HeadlessProject 已有完整集成 |

---

## 3. 整体架构

```
┌───────────────────────────────────────────────────────────────┐
│                     飞书开放平台                               │
│  ┌────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ 飞书客户端   │  │ Bot API (HTTP)│  │ 事件回调 (Webhook)  │   │
│  └─────┬──────┘  └──────┬───────┘  └─────────┬───────────┘   │
└────────┼─────────────────┼────────────────────┼───────────────┘
         │                 │                    │
         │  HTTPS/WebSocket                     │
         ▼                 ▼                    ▼
┌───────────────────────────────────────────────────────────────┐
│              feishu_bot (新增 crate)                           │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  message_handler  — 消息路由 & 命令解析                    │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │  commands/  — view / tree / search / symbol / git / list  │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │  session     — 用户 <-> 工作区 绑定映射                    │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │  api         — 飞书开放平台 HTTP API 封装                  │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │  workspace_client — 通过 WebSocket 连接 remote_server     │ │
│  └──────────────────────────────────────────────────────────┘ │
└───────────────┬───────────────────────────────────────────────┘
                │
            ┌───┴───┐
            │  RPC  │  (Protobuf Envelope, WebSocket)
            └───┬───┘
                │
┌───────────────┴───────────────────────────────────────────────┐
│           remote_server 扩展                                   │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  HeadlessProject                                         │ │
│  │  + read_file(path, range?) -> String                     │ │
│  │  + read_directory(path, depth) -> DirectoryTree          │ │
│  │  + search_files(query, path?) -> Vec<Match>              │ │
│  │  + search_symbols(query) -> Vec<SymbolInfo>              │ │
│  │  + git_status() -> GitStatus                             │ │
│  │  + file_metadata(path) -> FileMetadata                   │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│              本地 Zed 工作区（代码、LSP、Git 全可用）           │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. 新增 Crate: `feishu_bot`

### 4.1 目录结构

```
crates/feishu_bot/
├── Cargo.toml
├── src/
│   ├── feishu_bot.rs          # 库入口，初始化配置和启动
│   ├── config.rs              # 配置：App ID, Secret, Token, RateLimit
│   ├── api.rs                 # 飞书 HTTP API: 发送消息/获取用户/验证签名
│   ├── message_handler.rs     # 消息路由、命令分派
│   ├── session.rs             # SessionManager: 用户绑定工作区管理
│   ├── workspace_client.rs    # RPC 客户端代理（WebSocket <=> remote_server）
│   └── commands/
│       ├── mod.rs             # 命令注册表
│       ├── view.rs            # /view — 查看文件内容
│       ├── tree.rs            # /tree — 目录结构
│       ├── search.rs          # /search — 文件内容搜索
│       ├── symbol.rs          # /symbol — LSP 符号搜索
│       ├── git.rs             # /git — Git 状态/日志
│       └── list.rs            # /list — 列出可用工作区
```

### 4.2 核心接口设计

#### Config

```rust
/// 飞书 Bot 配置
pub struct FeishuBotConfig {
    /// 飞书应用 ID
    pub app_id: String,
    /// 飞书应用密钥
    pub app_secret: String,
    /// 事件校验 Token（Webhook）
    pub verification_token: String,
    /// Webhook 监听地址
    pub webhook_url: Option<String>,
    /// 请求频率限制（每用户）
    pub rate_limit: RateLimitConfig,
    /// 文件大小限制（超过此大小需用户确认）
    pub max_file_size: u64,
}
```

#### WorkspaceClient

```rust
/// 通过 WebSocket 连接到 remote_server 的 RPC 客户端
pub struct WorkspaceClient {
    /// 消息流（复用 rpc 协议）
    message_stream: MessageStream<WebSocket>,
    /// 待处理的请求
    pending_requests: HashMap<u32, oneshot::Sender<Envelope>>,
}

impl WorkspaceClient {
    /// 连接到指定的 remote_server
    pub async fn connect(host: &str, port: u16) -> Result<Self>;
    /// 读取文件内容
    pub async fn read_file(&mut self, path: &str, range: Option<(u64, u64)>) -> Result<String>;
    /// 读取目录结构
    pub async fn read_directory(&mut self, path: &str, depth: u32) -> Result<DirectoryTree>;
    /// 搜索文件内容
    pub async fn search_files(&mut self, pattern: &str, path: Option<&str>) -> Result<Vec<MatchResult>>;
    /// 搜索符号
    pub async fn search_symbols(&mut self, query: &str) -> Result<Vec<SymbolInfo>>;
}
```

#### CommandHandler

```rust
/// 每个命令实现此 trait
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// 命令名称（用于路由）
    fn command_name(&self) -> &'static str;
    /// 命令参数说明
    fn usage(&self) -> &'static str;
    /// 执行命令
    async fn execute(
        &self,
        session: &Session,
        args: &[String],
        bot_api: &FeishuApi,
    ) -> Result<Vec<FeishuMessage>>;
}
```

---

## 5. 扩展 RPC Protocol

### 5.1 新增文件: `crates/proto/proto/remote_control.proto`

```protobuf
syntax = "proto3";
package zed.messages;

import "worktree.proto";

message ReadFile {
    uint64 project_id = 1;
    uint64 worktree_id = 2;
    string path = 3;
    optional uint64 start_line = 4;
    optional uint64 end_line = 5;
}

message ReadFileResponse {
    string content = 1;
    uint64 total_lines = 2;
    uint64 file_size = 3;
    uint64 modified_at = 4;
}

message ReadDirectory {
    uint64 project_id = 1;
    uint64 worktree_id = 2;
    string path = 3;
    uint32 depth = 4;
}

message DirectoryEntry {
    string name = 1;
    bool is_directory = 2;
    optional uint64 file_size = 3;
    repeated DirectoryEntry children = 4;
}

message ReadDirectoryResponse {
    repeated DirectoryEntry entries = 1;
}

message SearchFilesRequest {
    uint64 project_id = 1;
    string pattern = 2;
    optional string path = 3;
    bool is_regex = 4;
    bool case_sensitive = 5;
    uint32 max_results = 6;
}

message SearchFilesResponse {
    repeated FileMatch matches = 1;
    bool truncated = 2;
}

message FileMatch {
    uint64 worktree_id = 1;
    string path = 2;
    uint64 line_number = 3;
    string line_content = 4;
    uint64 match_start = 5;
    uint64 match_end = 6;
}

message SearchSymbolsRequest {
    uint64 project_id = 1;
    string query = 2;
}

message SymbolInfo {
    string name = 1;
    string kind = 2;
    string path = 3;
    uint64 line = 4;
    uint64 column = 5;
    optional string container_name = 6;
}

message SearchSymbolsResponse {
    repeated SymbolInfo symbols = 1;
}

message GitStatusRequest {
    uint64 project_id = 1;
}

message GitStatusResponse {
    string branch = 1;
    repeated GitFileChange changes = 2;
    uint32 ahead = 3;
    uint32 behind = 4;
}

message GitFileChange {
    string path = 1;
    string status = 2;
}

message BotBindRequest {
    string bot_user_id = 1;
    string access_token = 2;
}

message BotBindResponse {
    bool success = 1;
    optional string message = 2;
}

message ListWorkspacesRequest {}

message WorkspaceInfo {
    string workspace_id = 1;
    string name = 2;
    string host = 3;
    repeated string root_paths = 4;
}

message ListWorkspacesResponse {
    repeated WorkspaceInfo workspaces = 1;
}
```

### 5.2 注册到 `zed.proto` Envelope

在 `crates/proto/proto/zed.proto` 的 `Envelope` 消息中新增消息类型。

---

## 6. 飞书命令设计

| 命令 | 用法 | 说明 |
|---|---|---|
| `/bind <token>` | `/bind zed_ws_abc123` | 绑定飞书用户到指定工作区 |
| `/list` | `/list` | 列出当前账户下所有在线工作区 |
| `/tree [path]` | `/tree src/` | 查看目录树（默认深度2层） |
| `/view <path> [L1-L2]` | `/view src/main.rs 1-50` | 查看文件内容（支持行范围） |
| `/search <pattern>` | `/search "fn handle_"` | 在项目中搜索文本 |
| `/symbol <name>` | `/symbol UserService` | 搜索函数/类/变量定义 |
| `/git` | `/git` | 查看当前分支和文件变更 |
| `/help` | `/help` | 显示帮助信息 |

### 消息格式规范

#### 代码展示

```
📄 src/main.rs (2.3KB · 42行 · 10分钟前修改)

    1 │ use std::sync::Arc;
    2 │ use gpui::*;
    3 │
    4 │ fn main() {
    5 │     App::new().run(|cx: &mut AppContext| {
    6 │         cx.open_window(WindowOptions::default(), |cx| {
    7 │             cx.new_view(|_cx| MyView)
    8 │         });
    9 │     });
   10 │ }

输入 /view src/main.rs 11-30 翻页
```

#### 目录展示

```
📁 src/
├── 📄 main.rs (420B)
├── 📁 components/
│   ├── 📄 mod.rs (120B)
│   ├── 📄 button.rs (2.1KB)
│   └── 📄 input.rs (1.8KB)
├── 📁 services/
│   ├── 📄 mod.rs (89B)
│   └── 📄 api.rs (5.6KB)
└── 📄 lib.rs (340B)

共 7 项 | 输入 /tree src/components/ 查看子目录
```

---

## 7. 认证流程

```
飞书用户                        飞书 Bot                    Remote Server
   │                              │                           │
   │ 1. /bind <workspace_token>   │                           │
   │ ────────────────────────────▶│                           │
   │                              │ 2. BotBindRequest         │
   │                              │   {bot_user_id, token}    │
   │                              │ ─────────────────────────▶│
   │                              │                           │
   │                              │ 3. BotBindResponse        │
   │                              │   {success: true}         │
   │                              │ ◀─────────────────────────│
   │                              │                           │
   │ 4. 绑定成功！                │                           │
   │    输入 /list 或 /view ...   │                           │
   │ ◀────────────────────────────│                           │
```

**Token 生成方式**：用户在 Zed 中执行 `bot:generate token` 命令，获得一次性绑定令牌（有效期 5 分钟），在飞书中输入 `/bind <token>` 完成绑定。

---

## 8. 安全策略

| 策略 | 说明 |
|---|---|
| **只读限制** | 所有飞书 Bot 操作仅允许读取，禁止编辑/删除/运行 |
| **Token 绑定** | 每个飞书用户必须使用一次性的 workspace token 绑定 |
| **Token 过期** | 绑定 token 有效期 5 分钟，绑定后的 session 有效期可配置（默认 24 小时） |
| **`.feishuignore`** | 支持在工作区根目录添加 `.feishuignore` 文件，按 `.gitignore` 语法排除敏感路径 |
| **文件大小限制** | 超过 500KB 的文件默认只返回前 100 行，需用户主动请求查看更多 |
| **Rate Limit** | 每个飞书用户每分钟最多 30 次请求 |
| **审计日志** | 所有操作记录到 `~/.zed/feishu_audit.log` |

---

## 9. 实现阶段

### 阶段 1: 基础设施（Proto + RPC 层）

- [ ] 新增 `remote_control.proto`，定义所有消息类型
- [ ] 在 `zed.proto` 的 `Envelope` 中注册新消息
- [ ] 在 `remote_server` 的 `HeadlessProject` 中实现基础只读 API: `read_file()`, `read_directory()`
- [ ] 在 `server.rs` 中添加新消息的 handler
- [ ] 新增 `feishu_bot` crate 骨架：`config.rs`, `workspace_client.rs`, `api.rs`

### 阶段 2: 核心命令实现

- [ ] 命令路由框架（`message_handler.rs`）
- [ ] `/bind` `/view` `/tree` `/list` 命令实现
- [ ] Session 管理（`session.rs`）

### 阶段 3: 搜索与扩展

- [ ] `/search` `/symbol` `/git` 命令实现
- [ ] 分页和交互卡片支持

### 阶段 4: 安全性 & 稳定性

- [ ] 认证系统、`.feishuignore` 过滤、Rate limiting、审计日志

---

## 10. 对现有代码的修改影响

| 文件 | 修改类型 | 说明 |
|---|---|---|
| `crates/proto/proto/remote_control.proto` | **新增** | 远程控制消息定义 |
| `crates/proto/proto/zed.proto` | 修改 | 在 Envelope oneof 中注册新消息 |
| `crates/remote_server/src/headless_project.rs` | 修改 | 新增只读 API 方法 |
| `crates/remote_server/src/server.rs` | 修改 | 添加新消息类型处理 |
| `crates/feishu_bot/` | **新增** | 飞书 Bot 网关 |

**不修改的文件**: `crates/remote/`, `crates/client/`, `crates/collab/`, `crates/project/`, `crates/editor/`, `crates/gpui/`。

---

## 11. 未来扩展

- **写入支持**：在飞书中编辑代码并提交 PR
- **终端访问**：执行终端命令并查看输出（如 `cargo test`）
- **诊断推送**：构建失败时主动推送到飞书
- **多平台 Bot**：支持钉钉、企业微信、Slack、Discord 等
- **代码审查集成**：飞书内直接查看 PR diff 和评论
