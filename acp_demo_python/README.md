# ACP Python 演示项目

这是一个 **Agent Communication Protocol (ACP)** 的 Python 参考实现演示，
展示 Zed 编辑器如何与外部 AI Agent 通过 ACP 协议通信。

## 项目文件

```
acp_demo_python/
├── acp_protocol.py      # ACP 核心协议实现 (JSON-RPC 2.0 over stdin/stdout)
├── demo_agent.py         # 功能丰富的 ACP Agent 演示
└── README.md             # 本文件
```

## 环境要求

- Python 3.8+（推荐 Python 3.10+）

## 运行方式

### 1. 自测试模式（模拟 Zed 客户端）

```bash
cd acp_demo_python
python demo_agent.py --self-test
```

这会运行一个完整的 ACP 会话交互测试，包括：
- 初始化握手
- 会话创建
- 基本提示词
- 工具调用（带权限请求）
- 取消处理
- 会话关闭

### 2. 直接运行（作为 Zed 的 ACP Agent）

```bash
python demo_agent.py
```

这会启动 Agent 并等待 stdin 的 JSON-RPC 消息（由 Zed 或其他 ACP 客户端发送）。

### 3. 配置到 Zed 中

在 Zed 的 `settings.json` 中添加：

```json
{
  "agent_servers": {
    "python-demo": {
      "command": {
        "command": "python",
        "args": ["/path/to/acp_demo_python/demo_agent.py"]
      }
    }
  }
}
```

然后在 Zed 的 Agent 面板中选择 "python-demo" 作为 Agent。

## ACP 协议概览

### 通信方式

- **传输层**: stdin/stdout，逐行 JSON
- **协议**: JSON-RPC 2.0
- **消息格式**: 每行一个 JSON 对象，以 `\n` 分隔

### JSON-RPC 消息格式

**请求**（Zed → Agent 或 Agent → Zed）:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "method": "initialize",
  "params": {"protocolVersion": "v1"}
}
```

**响应**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "result": {"protocolVersion": "v1", ...}
}
```

**错误响应**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "error": {"code": -32603, "message": "Internal error"}
}
```

**通知**（无需响应）:
```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {"sessionId": "...", "update": {...}}
}
```

### 完整会话流程

```
Zed                              Agent
 │                                 │
 │──── initialize 请求 ───────────►│    协议协商
 │◄─── initialize 响应 ────────────│    
 │                                 │
 │──── initialized 通知 ──────────►│    客户端就绪
 │                                 │
 │──── newSession 请求 ───────────►│    创建会话
 │◄─── newSession 响应 ────────────│
 │                                 │
 │──── prompt 请求 ───────────────►│    发送用户消息
 │     │                           │
 │     │◄── session/update 通知 ───│    流式助手消息块
 │     │  (type: assistantMessageChunk) 
 │     │◄── session/update 通知 ───│    更多块
 │     │  (type: assistantMessageChunk) 
 │     │◄── session/update 通知 ───│    助手消息完成
 │     │  (type: assistantMessageComplete) 
 │                                 │
 │     │◄── session/update 通知 ───│    工具调用
 │     │  (type: toolCall)         │    
 │◄──── requestPermission 请求 ────│    请求权限
 │──── requestPermission 响应 ────►│
 │     │◄── session/update 通知 ───│    工具调用更新
 │     │  (type: toolCallUpdate)   │
 │◄──  prompt 响应 ───────────────│    当前轮结束
 │                                 │
 │──── cancel 通知 ───────────────►│    取消
 │                                 │
 │──── closeSession 请求 ─────────►│    关闭会话
 │◄─── closeSession 响应 ──────────│
```

### 方法列表

| 方法 | 方向 | 用途 |
|---------|----------|---------|
| `initialize` | Client → Agent | 协议版本协商 + 能力声明 |
| `newSession` | Client → Agent | 创建新会话 |
| `loadSession` | Client → Agent | 加载现有会话 |
| `closeSession` | Client → Agent | 关闭会话 |
| `prompt` | Client → Agent | 发送用户提示词 |
| `cancel` | Client → Agent (通知) | 取消当前处理 |
| `session/update` | Agent → Client (通知) | 流式会话更新 |

Agent → Client 的请求（由 Zed 处理）：

| 方法 | 用途 |
|---------|---------|
| `requestPermission` | 请求用户授权 |
| `writeTextFile` | 写入文件 |
| `readTextFile` | 读取文件 |
| `createTerminal` | 创建终端 |

### 会话更新类型

| 类型 | 描述 |
|------|-------------|
| `userMessageChunk` | 用户消息增量 |
| `userMessageComplete` | 用户消息完成 |
| `assistantMessageChunk` | 助手消息块（流式） |
| `assistantMessageComplete` | 助手消息完成 |
| `toolCall` | 开始工具调用 |
| `toolCallUpdate` | 工具调用进度更新 |
| `toolCallComplete` | 工具调用完成 |

### 能力声明

Agent 在 `initialize` 响应中声明能力:

```json
{
  "agentCapabilities": {
    "promptCapabilities": {
      "modes": true,
      "modelSelection": true
    },
    "sessionCapabilities": {
      "close": {"supports": true},
      "resume": {"supports": false},
      "list": {"supports": false}
    },
    "loadSession": true
  }
}
```

## Demo 命令

启动 self-test 或连接到 Zed 后，可以向 Agent 发送以下消息：

| 消息 | 演示功能 |
|---------|----------------|
| `hello` | 基础流式响应 |
| `echo <text>` | 工具调用 + 权限请求 |
| `write <content>` | 文件写入 |
| `read file` | 文件读取 |
| `run <command>` | 终端命令 |
| `think` | 思维过程 |
| `delay` | 取消处理 |
| `error` | 错误处理 |
| `tools` | ACP 工具类型概览 |

## Zed 中的 ACP 实现参考

- **Crate**: `agent-client-protocol` v0.11.1
- **关键源文件**: `crates/agent_servers/src/acp.rs`, `crates/acp_thread/src/`
- **其他 ACP Agent**: `gemini-cli`, `claude-acp`, `codex-acp`
