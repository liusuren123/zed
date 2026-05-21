#!/usr/bin/env python3
"""
内部服务 ACP Agent —— 生产可用模板

把 Zed 接入你的内部服务：API、数据库、内部脚本、运维工具等。

架构：
  Zed ──ACP──► internal_agent.py ──HTTP/CLI/DB──► 你的内部服务

启动：
  python internal_agent.py

配置到 Zed settings.json：
  {
    "agent_servers": {
      "internal-tools": {
        "type": "custom",
        "command": {
          "command": "python",
          "args": ["/path/to/internal_agent.py"],
          "env": {
            "INTERNAL_API_KEY": "xxx",
            "API_BASE_URL": "http://internal-api.company.com"
          }
        }
      }
    }
  }

依赖（可选，按需安装）：
  pip install requests     # 调用 HTTP API
  pip install httpx        # 流式 HTTP
  pip install psycopg2     # PostgreSQL
"""

import json
import logging
import os
import queue
import subprocess
import sys
import threading
import uuid
from datetime import datetime
from typing import Any, Callable

# ─── 配置 ──────────────────────────────────────────────────────────────

# 这些在 Agent 启动时自动设置
API_BASE_URL = os.environ.get("API_BASE_URL", "")
INTERNAL_API_KEY = os.environ.get("INTERNAL_API_KEY", "")

LOG = logging.getLogger("internal-agent")
LOG.setLevel(logging.DEBUG)
_handler = logging.StreamHandler(sys.stderr)
_handler.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
LOG.addHandler(_handler)


# ─── 核心 ACP 协议 ────────────────────────────────────────────────────


class AcpAgent:
    """
    ACP Agent 核心引擎

    线程安全，支持：
    - 请求/响应（双向）
    - 通知（cancel/initialized）
    - 流式 session/update 通知
    - 工具调用（文件、终端、权限）
    """

    def __init__(self, name: str = "internal-agent"):
        self.name = name
        self.sessions: dict[str, dict] = {}
        self._pending_requests: dict[str, queue.Queue] = {}
        self._running = True
        self._write_lock = threading.Lock()
        self._request_handlers: dict[str, Callable] = {}

        self._register_handlers()

    # ── 注册请求处理器 ─────────────────────────────────────────────

    def _register_handlers(self):
        self._request_handlers["initialize"] = self.handle_initialize
        self._request_handlers["newSession"] = self.handle_new_session
        self._request_handlers["loadSession"] = self.handle_load_session
        self._request_handlers["closeSession"] = self.handle_close_session
        self._request_handlers["prompt"] = self.handle_prompt

    # ── IO ─────────────────────────────────────────────────────────

    def _send_line(self, obj: dict):
        """发送一行 JSON 到 stdout"""
        line = json.dumps(obj, ensure_ascii=False) + "\n"
        with self._write_lock:
            sys.stdout.write(line)
            sys.stdout.flush()

    def send_result(self, msg_id: str | int, result: dict):
        self._send_line({"jsonrpc": "2.0", "id": msg_id, "result": result})

    def send_error(self, msg_id: str | int, code: int, message: str):
        self._send_line(
            {
                "jsonrpc": "2.0",
                "id": msg_id,
                "error": {"code": code, "message": message},
            }
        )

    def send_notification(self, method: str, params: dict = None):
        msg = {"jsonrpc": "2.0", "method": method}
        if params:
            msg["params"] = params
        self._send_line(msg)

    def send_session_update(self, session_id: str, update: dict):
        self.send_notification(
            "session/update",
            {
                "sessionId": session_id,
                "update": update,
            },
        )

    def send_assistant_chunk(self, session_id: str, text: str):
        """流式发送助手消息块"""
        self.send_session_update(
            session_id,
            {
                "content": text,
                "type": "assistantMessageChunk",
            },
        )

    def send_assistant_complete(self, session_id: str):
        self.send_session_update(session_id, {"type": "assistantMessageComplete"})

    def send_tool_call(
        self,
        session_id: str,
        tool_id: str,
        title: str,
        kind: str = "execute",
        status: str = "inProgress",
        content: list = None,
    ):
        self.send_session_update(
            session_id,
            {
                "id": tool_id,
                "title": title,
                "kind": kind,
                "content": content or [],
                "locations": [],
                "meta": {},
                "status": status,
            },
        )

    def send_tool_call_update(
        self,
        session_id: str,
        tool_id: str,
        status: str = None,
        content: list = None,
        raw_output: str = None,
    ):
        update = {"id": tool_id}
        if status:
            update["status"] = status
        if content is not None:
            update["content"] = content
        if raw_output:
            update["rawOutput"] = raw_output
        self.send_session_update(session_id, update)

    # ── Agent → Zed 请求（同步等待响应） ─────────────────────────

    def send_request(self, method: str, params: dict) -> dict:
        """向 Zed 发起请求并等待响应（阻塞）"""
        req_id = str(uuid.uuid4())
        q: queue.Queue = queue.Queue()
        self._pending_requests[req_id] = q
        self._send_line(
            {
                "jsonrpc": "2.0",
                "id": req_id,
                "method": method,
                "params": params,
            }
        )
        result = q.get(timeout=300)

        # 如果是错误响应，抛异常
        if isinstance(result, dict) and "error" in result:
            err = result["error"]
            raise RuntimeError(f"[{err.get('code')}] {err.get('message')}")
        return result

    def request_permission(
        self, session_id: str, tool_id: str, title: str, kind: str = "execute"
    ) -> dict:
        """请求用户授权某项操作"""
        # 先发 toolCall 通知让 UI 显示
        self.send_tool_call(
            session_id, tool_id, title, kind, status="waitingForConfirmation"
        )
        # 发请求等用户确认
        return self.send_request(
            "requestPermission",
            {
                "sessionId": session_id,
                "toolCall": {
                    "id": tool_id,
                    "title": title,
                    "kind": kind,
                    "content": [],
                    "locations": [],
                    "meta": {},
                    "status": "waitingForConfirmation",
                },
                "options": [
                    {"id": "allow", "label": "允许一次", "kind": "allowOnce"},
                    {"id": "deny", "label": "拒绝", "kind": "denyOnce"},
                ],
            },
        )

    def read_text_file(
        self, session_id: str, path: str, line: int = 1, limit: int = 200
    ) -> str:
        """请求 Zed 读取文件"""
        result = self.send_request(
            "readTextFile",
            {
                "sessionId": session_id,
                "path": path,
                "line": line,
                "limit": limit,
            },
        )
        return result.get("content", "")

    def write_text_file(self, session_id: str, path: str, content: str):
        """请求 Zed 写入文件"""
        self.send_request(
            "writeTextFile",
            {
                "sessionId": session_id,
                "path": path,
                "content": content,
            },
        )

    def create_terminal(
        self, session_id: str, command: str, args: list = None, cwd: str = None
    ) -> dict:
        """请求 Zed 创建终端"""
        params = {"sessionId": session_id, "command": command}
        if args:
            params["args"] = args
        if cwd:
            params["cwd"] = cwd
        return self.send_request("createTerminal", params)

    # ── ACP 请求处理器 ──────────────────────────────────────────

    def handle_initialize(self, msg_id, params):
        LOG.info("初始化: protocolVersion=%s", params.get("protocolVersion"))
        self.send_result(
            msg_id,
            {
                "protocolVersion": "v1",
                "agentCapabilities": {
                    "promptCapabilities": {"modes": False, "modelSelection": False},
                    "sessionCapabilities": {
                        "close": {"supports": True},
                        "resume": {"supports": False},
                        "list": {"supports": False},
                    },
                    "loadSession": True,
                },
                "authMethods": [],
                "agentInfo": {"name": self.name, "version": "1.0.0"},
            },
        )

    def handle_new_session(self, msg_id, params):
        sid = str(uuid.uuid4())
        self.sessions[sid] = {
            "id": sid,
            "cwd": params.get("cwd", "/"),
            "cancelled": False,
            "created_at": datetime.now(),
        }
        LOG.info("新建会话: %s  cwd=%s", sid, params.get("cwd"))
        self.send_result(
            msg_id,
            {
                "sessionId": sid,
                "modes": {
                    "currentModeId": "default",
                    "availableModes": [{"id": "default", "name": "默认"}],
                },
                "models": {
                    "currentModelId": "default-model",
                    "availableModels": [
                        {
                            "modelId": "default-model",
                            "name": "Internal Service",
                        }
                    ],
                },
                "configOptions": [],
            },
        )

    def handle_load_session(self, msg_id, params):
        sid = params.get("sessionId")
        if sid not in self.sessions:
            self.sessions[sid] = {
                "id": sid,
                "cwd": params.get("cwd"),
                "cancelled": False,
                "created_at": datetime.now(),
            }
        self.send_result(
            msg_id,
            {
                "modes": {"currentModeId": "default", "availableModes": []},
                "models": {"currentModelId": "default-model", "availableModels": []},
                "configOptions": [],
            },
        )

    def handle_close_session(self, msg_id, params):
        sid = params.get("sessionId")
        self.sessions.pop(sid, None)
        LOG.info("关闭会话: %s", sid)
        self.send_result(msg_id, {})

    def handle_cancel(self, session_id: str):
        if session_id in self.sessions:
            self.sessions[session_id]["cancelled"] = True
            LOG.info("取消会话: %s", session_id)

    def handle_prompt(self, msg_id, params):
        """
        核心方法 —— 在这里接入你的内部服务。

        在独立线程中运行，不会阻塞主循环。
        """
        session_id = params.get("sessionId")
        messages = params.get("messages", [])
        session = self.sessions.get(session_id)

        if not session:
            self.send_error(msg_id, -32002, "session not found")
            return

        try:
            user_text = self._extract_text(messages)
            self.on_prompt(session_id, user_text)
            self.send_result(msg_id, {"stopReason": "endTurn"})
        except Exception as e:
            LOG.exception("prompt 处理出错")
            self.send_error(msg_id, -32603, str(e))

    def _extract_text(self, messages: list) -> str:
        texts = []
        for m in messages:
            if isinstance(m, dict):
                t = m.get("text", "")
                if t:
                    texts.append(t)
            elif isinstance(m, str):
                texts.append(m)
        return " ".join(texts)

    # ─── 子类重写这个方法 ────────────────────────────────────────

    def on_prompt(self, session_id: str, user_text: str):
        """
        处理用户消息。子类应重写此方法。

        可以调用:
        - self.send_assistant_chunk(sid, text)  — 流式回复
        - self.read_text_file(sid, path)         — 读文件
        - self.write_text_file(sid, path, text)  — 写文件
        - self.create_terminal(sid, cmd)          — 创建终端
        - self.request_permission(sid, ...)       — 请求授权
        """
        self.send_assistant_chunk(
            session_id,
            f"收到消息: {user_text[:100]}\n\n请继承 AcpAgent 并重写 on_prompt 方法。",
        )
        self.send_assistant_complete(session_id)

    # ─── 主循环 ─────────────────────────────────────────────────

    def run(self):
        """启动 Agent 主循环"""
        LOG.info("ACP Agent '%s' 启动", self.name)
        if API_BASE_URL:
            LOG.info("内部 API: %s", API_BASE_URL)

        read_thread = threading.Thread(target=self._stdin_reader, daemon=True)
        read_thread.start()
        read_thread.join()

    def _stdin_reader(self):
        """后台线程：持续读取 stdin"""
        try:
            for line in sys.stdin:
                line = line.strip()
                if not line:
                    continue
                try:
                    self._dispatch(json.loads(line))
                except json.JSONDecodeError:
                    LOG.warning("无法解析 JSON: %s", line[:100])
        except EOFError:
            pass
        finally:
            self._running = False

    def _dispatch(self, msg: dict):
        """分发消息到对应处理器"""
        msg_id = msg.get("id")
        method = msg.get("method")

        # 请求（有 id + method）
        if msg_id and method:
            handler = self._request_handlers.get(method)
            if handler:
                t = threading.Thread(
                    target=handler, args=(msg_id, msg.get("params")), daemon=True
                )
                t.start()
            else:
                LOG.warning("未知方法: %s", method)
                self.send_error(msg_id, -32601, f"unknown method: {method}")

        # 响应（有 id 无 method）
        elif msg_id and not method:
            q = self._pending_requests.pop(
                str(msg_id), None
            ) or self._pending_requests.pop(msg_id, None)
            if q:
                q.put(msg)

        # 通知（无 id）
        elif method:
            params = msg.get("params") or {}
            if method == "cancel":
                self.handle_cancel(params.get("sessionId"))
            elif method == "initialized":
                LOG.info("Zed 客户端已就绪")

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self._running = False


# ══════════════════════════════════════════════════════════════════════
# 你的内部服务接入代码
# ══════════════════════════════════════════════════════════════════════
#
# 继承 AcpAgent，重写 on_prompt 方法。
# 可以在这里做任何事：调 API、查数据库、执行脚本。
# ══════════════════════════════════════════════════════════════════════


class InternalServiceAgent(AcpAgent):
    """接入内部服务的 ACP Agent"""

    def __init__(self):
        super().__init__(name="internal-tools")
        self.contexts: dict[str, list] = {}

    def on_prompt(self, session_id: str, user_text: str):
        """处理用户消息 —— 在这里写你的业务逻辑"""
        if session_id not in self.contexts:
            self.contexts[session_id] = []
        self.contexts[session_id].append({"role": "user", "content": user_text})

        text = user_text.lower().strip()
        if text.startswith("search ") or text.startswith("搜索"):
            self._handle_search(session_id, text)
        elif text.startswith("deploy ") or text.startswith("部署"):
            self._handle_deploy(session_id, text)
        elif text.startswith("db ") or text.startswith("数据库"):
            self._handle_database(session_id, text)
        elif text.startswith("run ") or text.startswith("执行"):
            self._handle_execute(session_id, text)
        elif text.startswith("status") or text.startswith("状态"):
            self._handle_status(session_id)
        elif text.startswith("help") or text.startswith("帮助"):
            self._handle_help(session_id)
        else:
            self._handle_default(session_id, user_text)

        self.contexts[session_id].append({"role": "assistant", "content": "(done)"})
        self.send_assistant_complete(session_id)

    def _handle_search(self, session_id: str, text: str):
        self.send_assistant_chunk(session_id, "搜索内部知识库...\n\n")
        query = text.split(" ", 1)[1] if " " in text else text
        results = self._query_api("search", {"q": query})
        if results:
            self.send_assistant_chunk(session_id, f"找到 {len(results)} 条结果:\n\n")
            for r in results[:5]:
                self.send_assistant_chunk(
                    session_id,
                    f"- **{r.get('title', '?')}**: {r.get('summary', '')[:100]}\n",
                )
        else:
            self.send_assistant_chunk(session_id, "未找到结果。\n")

    def _handle_deploy(self, session_id: str, text: str):
        service = text.split(" ", 1)[1] if " " in text else "unknown"
        tid = f"d-{uuid.uuid4().hex[:6]}"
        self.send_assistant_chunk(session_id, f"准备部署 {service}...\n")
        r = self.request_permission(session_id, tid, title=f"部署 {service}")
        if r.get("optionId") == "allow":
            self.send_tool_call_update(session_id, tid, status="inProgress")
            ok = self._exec_api("deploy", {"service": service})
            st = "completed" if ok else "failed"
            self.send_tool_call_update(
                session_id, tid, status=st, raw_output=f"deploy {service}: {st}"
            )
            self.send_assistant_chunk(
                session_id, f"{'✅' if ok else '❌'} 部署{'成功' if ok else '失败'}\n"
            )
        else:
            self.send_assistant_chunk(session_id, "已取消。\n")

    def _handle_database(self, session_id: str, text: str):
        sql = text.split(" ", 1)[1] if " " in text else ""
        tid = f"db-{uuid.uuid4().hex[:6]}"
        self.send_assistant_chunk(session_id, f"SQL: {sql}\n")
        r = self.request_permission(session_id, tid, title=f"SQL: {sql[:40]}")
        if r.get("optionId") == "allow":
            rows = self._query_api("query", {"sql": sql})
            if isinstance(rows, list):
                self.send_assistant_chunk(session_id, f"{len(rows)} 行\n")
                for row in rows[:10]:
                    self.send_assistant_chunk(
                        session_id, f"  {json.dumps(row, ensure_ascii=False)}\n"
                    )
        else:
            self.send_assistant_chunk(session_id, "已取消\n")

    def _handle_execute(self, session_id: str, text: str):
        cmd = text.split(" ", 1)[1] if " " in text else ""
        tid = f"x-{uuid.uuid4().hex[:6]}"
        self.send_assistant_chunk(session_id, f"准备执行: {cmd}\n")
        r = self.request_permission(session_id, tid, title=f"run: {cmd[:40]}")
        if r.get("optionId") == "allow":
            self.send_tool_call_update(session_id, tid, status="inProgress")
            try:
                p = subprocess.run(
                    cmd, shell=True, capture_output=True, text=True, timeout=120
                )
                out = (p.stdout + p.stderr)[:2000]
                self.send_tool_call_update(
                    session_id,
                    tid,
                    status="completed" if p.returncode == 0 else "failed",
                    raw_output=out[:500],
                )
                self.send_assistant_chunk(
                    session_id, f"exit={p.returncode}\n```\n{out}\n```\n"
                )
            except subprocess.TimeoutExpired:
                self.send_assistant_chunk(session_id, "超时\n")

    def _handle_status(self, session_id: str):
        self.send_assistant_chunk(session_id, "系统状态:\n\n")
        data = self._query_api("status", {}) if API_BASE_URL else {}
        if data:
            for k, v in data.items():
                self.send_assistant_chunk(session_id, f"  {k}: {v}\n")
        else:
            import platform

            self.send_assistant_chunk(
                session_id,
                f"  主机: {platform.node()}\n"
                f"  Python: {platform.python_version()}\n"
                f"  API: {'已配置' if API_BASE_URL else '未配置'}\n",
            )

    def _handle_help(self, session_id: str):
        self.send_assistant_chunk(
            session_id,
            """
可用命令:
  search <关键词>  - 搜索内部知识库
  deploy <服务名>  - 部署服务 (需授权)
  db <SQL>        - 数据库查询 (需授权)
  run <命令>      - 执行命令 (需授权)
  status          - 系统状态
  help            - 本帮助
""",
        )

    def _handle_default(self, session_id: str, user_text: str):
        self.send_assistant_chunk(
            session_id, f'收到: "{user_text}"\n输入 help 查看可用命令。\n'
        )

    # ── 内部 API 调用封装 ──────────────────────────────────────

    def _query_api(self, endpoint: str, params: dict) -> Any:
        if not API_BASE_URL:
            return []
        try:
            import requests

            resp = requests.get(
                f"{API_BASE_URL}/{endpoint}",
                params=params,
                headers={"Authorization": f"Bearer {INTERNAL_API_KEY}"},
                timeout=10,
            )
            resp.raise_for_status()
            return resp.json()
        except ImportError:
            return []
        except Exception as e:
            LOG.error("API 调用失败: %s %s", endpoint, e)
            return []

    def _exec_api(self, endpoint: str, data: dict) -> bool:
        """调用内部 API (POST)"""
        if not API_BASE_URL:
            return False
        try:
            import requests

            resp = requests.post(
                f"{API_BASE_URL}/{endpoint}",
                json=data,
                headers={"Authorization": f"Bearer {INTERNAL_API_KEY}"},
                timeout=60,
            )
            return resp.ok
        except Exception as e:
            LOG.error("API 调用失败: %s %s", endpoint, e)
            return False


# ══════════════════════════════════════════════════════════════════════
# 启动
# ══════════════════════════════════════════════════════════════════════

# ══════════════════════════════════════════════════════════════════════
# 自测试
# ══════════════════════════════════════════════════════════════════════


def run_self_test():
    """
    在内存中运行自测试，模拟 Zed 客户端，验证 ACP 协议握手完整流程。
    不依赖真实后端 API。
    """
    import queue
    import time

    agent = InternalServiceAgent()

    # 用内存队列替代 stdin/stdout
    to_agent: queue.Queue = queue.Queue()
    from_agent: queue.Queue = queue.Queue()
    original_send = agent._send_line
    agent._send_line = lambda obj: from_agent.put(json.dumps(obj))

    class TestClient:
        """模拟的 Zed 客户端"""

        def __init__(self):
            self._pending = {}
            self._callbacks = []
            self._id = 0

        def send(self, msg: dict):
            to_agent.put(json.dumps(msg))

        def request(self, method: str, params: dict = None) -> dict:
            self._id += 1
            msg_id = self._id
            q: queue.Queue = queue.Queue()
            self._pending[msg_id] = q
            self.send(
                {
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "method": method,
                    "params": params or {},
                }
            )
            return q.get(timeout=10)

        def notify(self, method: str, params: dict = None):
            self.send({"jsonrpc": "2.0", "method": method, "params": params or {}})

        def on_session_update(self, callback):
            self._callbacks.append(callback)

    client = TestClient()

    # 后台线程：把 agent 的输出转发给 client
    def agent_output_loop():
        while True:
            try:
                line = from_agent.get(timeout=15)
            except queue.Empty:
                break
            msg = json.loads(line)
            msg_id = msg.get("id")
            method = msg.get("method")

            if msg_id and not method:  # 响应
                q = client._pending.pop(msg_id, None)
                if q:
                    q.put(msg)
            elif method == "session/update":  # 通知
                for cb in client._callbacks:
                    cb(msg.get("params", {}))

    import threading

    t = threading.Thread(target=agent_output_loop, daemon=True)
    t.start()

    # 启动 Agent（在另一个线程中读 stdin）
    def agent_stdin_loop():
        while True:
            try:
                line = to_agent.get(timeout=15)
            except queue.Empty:
                break
            agent._dispatch(json.loads(line))

    t2 = threading.Thread(target=agent_stdin_loop, daemon=True)
    t2.start()

    results = {"passed": 0, "failed": 0}
    collected_updates = []
    client.on_session_update(lambda p: collected_updates.append(p))

    def check(name: str, ok: bool, detail: str = ""):
        if ok:
            results["passed"] += 1
            print(f"  ✅ {name}")
        else:
            results["failed"] += 1
            print(f"  ❌ {name} — {detail}")

    print("=" * 50)
    print("  Internal Agent 自测试")
    print("=" * 50)

    # 1. 初始化
    print("\n[测试 1/6] 初始化握手")
    r = client.request(
        "initialize",
        {
            "protocolVersion": "v1",
            "clientCapabilities": {
                "fs": {"readTextFile": True, "writeTextFile": True},
                "terminal": True,
                "auth": {"terminal": True},
            },
            "clientInfo": {"name": "test", "version": "1.0"},
        },
    )
    check("协议版本", r.get("result", {}).get("protocolVersion") == "v1")
    check(
        "Agent 名称",
        r.get("result", {}).get("agentInfo", {}).get("name") == "internal-tools",
    )
    client.notify("initialized")

    # 2. 创建会话
    print("\n[测试 2/6] 创建会话")
    r = client.request("newSession", {"cwd": "/test"})
    sid = r.get("result", {}).get("sessionId", "")
    check("返回 sessionId", bool(sid))
    check("sessionId 是 UUID 格式", len(sid) > 10)

    # 3. 基本提示词
    print("\n[测试 3/6] 基本提示词")
    collected_updates.clear()
    r = client.request(
        "prompt", {"sessionId": sid, "messages": [{"type": "text", "text": "hello"}]}
    )
    check("返回 endTurn", r.get("result", {}).get("stopReason") == "endTurn")
    updates = [u.get("update", {}).get("type") for u in collected_updates]
    has_chunk = "assistantMessageChunk" in updates
    has_complete = "assistantMessageComplete" in updates
    check("有流式消息块", has_chunk)
    check("有完成标记", has_complete)

    # 4. 帮助命令
    print("\n[测试 4/6] help 命令")
    collected_updates.clear()
    r = client.request(
        "prompt", {"sessionId": sid, "messages": [{"type": "text", "text": "help"}]}
    )
    check("返回正常", r.get("result", {}).get("stopReason") == "endTurn")

    # 5. 搜索命令（模拟模式，无后端）
    print("\n[测试 5/6] 搜索命令 (无后端)")
    collected_updates.clear()
    r = client.request(
        "prompt",
        {"sessionId": sid, "messages": [{"type": "text", "text": "search something"}]},
    )
    check("返回正常", r.get("result", {}).get("stopReason") == "endTurn")

    # 6. 加载/关闭会话
    print("\n[测试 6/6] 会话管理")
    r = client.request("loadSession", {"sessionId": sid, "cwd": "/test"})
    check("加载会话", r.get("result", {}).get("modes") is not None)
    r = client.request("closeSession", {"sessionId": sid})
    check("关闭会话", r.get("result") == {})

    # 结果
    total = results["passed"] + results["failed"]
    print(f"\n{'=' * 50}")
    print(f"  结果: {results['passed']}/{total} 通过")
    if results["failed"] > 0:
        print(f"  {results['failed']} 个失败")
        return 1
    else:
        print("  全部通过!")
        return 0


if __name__ == "__main__":
    import sys

    if "--self-test" in sys.argv:
        sys.exit(run_self_test())
    else:
        agent = InternalServiceAgent()
        agent.run()
