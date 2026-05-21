#!/usr/bin/env python3
"""
Claude Code ACP 代理 —— 真实转发到 Claude Code CLI

架构：
  Zed ──ACP──► claude_proxy.py ──ACP──► Claude Code CLI（真实 AI）

工作方式：
  1. Zed 通过 ACP 协议与本脚本通信（stdin/stdout）
  2. 本脚本把消息原样转发给 Claude Code CLI 子进程（stdin/stdout）
  3. Claude Code 的真实回复原样传回给 Zed

  本脚本可以在中间做：日志、鉴权、流量控制、消息拦截/修改等。

启动（直接测试）：
  python claude_proxy.py

配置到 Zed settings.json：
  {
    "agent_servers": {
      "claude-proxy": {
        "type": "custom",
        "command": {
          "command": "python",
          "args": ["/path/to/claude_proxy.py"]
        },
        "env": {
          "ANTHROPIC_API_KEY": "sk-ant-..."
        }
      }
    }
  }

依赖：
  pip install anthropic  (可选，用于直接调用 API 的备用模式)
"""

import json
import logging
import os
import queue
import shutil
import subprocess
import sys
import threading
import uuid

LOG = logging.getLogger("claude-proxy")
LOG.setLevel(logging.DEBUG)
_h = logging.StreamHandler(sys.stderr)
_h.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
LOG.addHandler(_h)


# ─── 工具函数 ────────────────────────────────────────────────────────


def find_claude_code() -> str:
    """查找 Claude Code CLI 可执行文件"""
    # 优先环境变量
    env_exe = os.environ.get("CLAUDE_CODE_EXECUTABLE")
    if env_exe:
        if os.path.isfile(env_exe):
            return env_exe
        LOG.warning("CLAUDE_CODE_EXECUTABLE 指向的文件不存在: %s", env_exe)

    # 尝试 npx
    npx = shutil.which("npx")
    if npx:
        LOG.info("使用 npx 启动 Claude Code")
        return npx

    # 尝试直接找 claude 命令
    claude = shutil.which("claude")
    if claude:
        LOG.info("找到本地 claude: %s", claude)
        return claude

    raise FileNotFoundError(
        "未找到 Claude Code CLI。请安装:\n"
        "  npm install -g @anthropic/claude-code\n"
        "或设置环境变量:\n"
        "  CLAUDE_CODE_EXECUTABLE=/path/to/claude"
    )


# ─── Claude Code 代理 ────────────────────────────────────────────────


class ClaudeCodeProxy:
    """
    把 stdin/stdout 的 ACP 消息双向转发到 Claude Code 子进程。

    工作方式：
    - stdin 线程：读取 Zed 的输入 → 写入 Claude Code
    - stdout 线程：读取 Claude Code 的输出 → 写入 Zed
    - 两个方向完全独立，不解析消息内容
    """

    def __init__(self, claude_cmd: list[str] | None = None):
        self.claude_cmd = claude_cmd or self._build_command()
        self.proc: subprocess.Popen | None = None
        self._running = False

    def _build_command(self) -> list[str]:
        """构建 Claude Code 启动命令"""
        exe = find_claude_code()
        if os.path.basename(exe) == "npx":
            return [exe, "--yes", "@anthropic/claude-code", "--acp"]
        return [exe, "--acp"]

    def start(self):
        """启动 Claude Code 子进程"""
        cmd = self.claude_cmd
        LOG.info("启动 Claude Code: %s", " ".join(cmd))

        # 继承父进程的环境变量（如 ANTHROPIC_API_KEY）
        env = os.environ.copy()
        self.proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            text=True,
            bufsize=1,  # 行缓冲
        )
        self._running = True
        LOG.info("Claude Code 已启动 (PID=%d)", self.proc.pid)

    def stop(self):
        """停止 Claude Code 子进程"""
        self._running = False
        if self.proc and self.proc.poll() is None:
            LOG.info("正在停止 Claude Code (PID=%d)...", self.proc.pid)
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
            LOG.info("Claude Code 已停止 (exit=%d)", self.proc.returncode)

    def run(self):
        """主循环：启动 Claude Code，双向转发消息"""
        self.start()

        # ── stderr 日志线程 ──
        def log_stderr():
            for line in self.proc.stderr:
                if line.strip():
                    LOG.debug("[claude] %s", line.rstrip())

        threading.Thread(target=log_stderr, daemon=True).start()

        # ── 方向 A：Zed → Claude Code ──
        # 读取 sys.stdin，写入 proc.stdin
        def forward_to_claude():
            try:
                for line in sys.stdin:
                    if not self._running:
                        break
                    line_stripped = line.strip()
                    if not line_stripped:
                        continue
                    LOG.debug("  >>> %s", line_stripped[:200])
                    self.proc.stdin.write(line)
                    self.proc.stdin.flush()
            except (BrokenPipeError, OSError):
                pass
            finally:
                LOG.info("Zed 输入流已关闭")
                self.stop()

        # ── 方向 B：Claude Code → Zed ──
        # 读取 proc.stdout，写入 sys.stdout
        def forward_to_zed():
            try:
                for line in self.proc.stdout:
                    if not self._running:
                        break
                    line_stripped = line.strip()
                    if not line_stripped:
                        continue
                    LOG.debug("  <<< %s", line_stripped[:200])
                    sys.stdout.write(line)
                    sys.stdout.flush()
            except (BrokenPipeError, OSError):
                pass
            finally:
                LOG.info("Claude Code 输出流已关闭")
                self.stop()

        t1 = threading.Thread(target=forward_to_claude, daemon=True)
        t2 = threading.Thread(target=forward_to_zed, daemon=True)
        t1.start()
        t2.start()

        # 等待 Claude Code 进程结束
        self.proc.wait()
        self._running = False
        LOG.info("Claude Code 进程已退出 (exit=%d)", self.proc.returncode)


# ─── 备用模式：直接调用 Anthropic API ──────────────────────────────
#
# 如果不想依赖 Claude Code CLI，也可以用这个模式，
# 直接调用 Anthropic API。这是一个完整的 ACP Agent 实现。
# ────────────────────────────────────────────────────────────────────


class DirectApiAgent:
    """
    直接调用 Anthropic API 的 ACP Agent。
    不依赖 Claude Code CLI，需要设置 ANTHROPIC_API_KEY。

    这是一个完整的 ACP → API 适配器，包含：
    - ACP 协议握手
    - 流式回复
    - 工具调用（文件读写通过 Zed 执行）
    - 取消处理
    """

    def __init__(self):
        self.api_key = os.environ.get("ANTHROPIC_API_KEY", "")
        self.sessions: dict[str, dict] = {}
        self._pending: dict[str, queue.Queue] = {}
        self._write_lock = threading.Lock()
        self._running = True

    # ── ACP IO ─────────────────────────────────────────────────────

    def _send(self, obj: dict):
        line = json.dumps(obj, ensure_ascii=False) + "\n"
        with self._write_lock:
            sys.stdout.write(line)
            sys.stdout.flush()

    def send_result(self, msg_id, result: dict):
        self._send({"jsonrpc": "2.0", "id": msg_id, "result": result})

    def send_error(self, msg_id, code: int, message: str):
        self._send(
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
        self._send(msg)

    def session_update(self, sid: str, update: dict):
        self.send_notification("session/update", {"sessionId": sid, "update": update})

    def send_assistant_chunk(self, sid: str, text: str):
        self.session_update(sid, {"content": text, "type": "assistantMessageChunk"})

    def send_assistant_complete(self, sid: str):
        self.session_update(sid, {"type": "assistantMessageComplete"})

    # ── Agent → Zed 请求 ───────────────────────────────────────────

    def send_request(self, method: str, params: dict) -> dict:
        req_id = str(uuid.uuid4())
        q: queue.Queue = queue.Queue()
        self._pending[req_id] = q
        self._send({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
        result = q.get(timeout=300)
        if isinstance(result, dict) and "error" in result:
            err = result["error"]
            raise RuntimeError(f"[{err.get('code')}] {err.get('message')}")
        return result

    def read_file(self, sid: str, path: str) -> str:
        r = self.send_request(
            "readTextFile", {"sessionId": sid, "path": path, "line": 1, "limit": 200}
        )
        return r.get("content", "")

    def request_permission(self, sid: str, tool_id: str, title: str) -> dict:
        self.session_update(
            sid,
            {
                "id": tool_id,
                "title": title,
                "kind": "execute",
                "content": [],
                "locations": [],
                "meta": {},
                "status": "waitingForConfirmation",
            },
        )
        return self.send_request(
            "requestPermission",
            {
                "sessionId": sid,
                "toolCall": {
                    "id": tool_id,
                    "title": title,
                    "kind": "execute",
                    "content": [],
                    "locations": [],
                    "meta": {},
                    "status": "waitingForConfirmation",
                },
                "options": [
                    {"id": "allow", "label": "允许", "kind": "allowOnce"},
                    {"id": "deny", "label": "拒绝", "kind": "denyOnce"},
                ],
            },
        )

    # ── ACP 请求处理 ──────────────────────────────────────────────

    def handle_initialize(self, msg_id, params):
        LOG.info("初始化请求")
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
                "agentInfo": {"name": "claude-direct-api", "version": "1.0.0"},
            },
        )

    def handle_new_session(self, msg_id, params):
        sid = str(uuid.uuid4())
        self.sessions[sid] = {
            "id": sid,
            "cwd": params.get("cwd", "/"),
            "messages": [],
            "cancelled": False,
        }
        LOG.info("新建会话: %s", sid)
        self.send_result(
            msg_id,
            {
                "sessionId": sid,
                "modes": {
                    "currentModeId": "default",
                    "availableModes": [{"id": "default", "name": "Default"}],
                },
                "models": {
                    "currentModelId": "claude-sonnet-4",
                    "availableModels": [
                        {"modelId": "claude-sonnet-4", "name": "Claude Sonnet 4"},
                    ],
                },
                "configOptions": [],
            },
        )

    def handle_close_session(self, msg_id, params):
        self.sessions.pop(params.get("sessionId"), None)
        self.send_result(msg_id, {})

    def handle_cancel(self, sid: str):
        if sid in self.sessions:
            self.sessions[sid]["cancelled"] = True
            LOG.info("取消会话: %s", sid)

    def handle_prompt(self, msg_id, params):
        """核心：调用真实的 Anthropic API"""
        sid = params.get("sessionId")
        session = self.sessions.get(sid)
        if not session:
            self.send_error(msg_id, -32002, "session not found")
            return

        if not self.api_key:
            self.send_error(msg_id, -32603, "请设置 ANTHROPIC_API_KEY 环境变量")
            return

        # 提取用户消息
        user_text = ""
        for m in params.get("messages", []):
            if isinstance(m, dict):
                user_text += m.get("text", "")

        session["messages"].append({"role": "user", "content": user_text})
        session["cancelled"] = False

        try:
            # 调用 Anthropic API（流式）
            import anthropic

            client = anthropic.Anthropic(api_key=self.api_key)

            with client.messages.stream(
                model="claude-sonnet-4-20250514",
                max_tokens=4096,
                messages=session["messages"],
            ) as stream:
                for chunk in stream:
                    if session["cancelled"]:
                        LOG.info("检测到取消，停止生成")
                        break
                    if chunk.type == "content_block_delta" and chunk.delta.text:
                        self.send_assistant_chunk(sid, chunk.delta.text)

            self.send_assistant_complete(sid)
            stop = "cancelled" if session["cancelled"] else "endTurn"
            self.send_result(msg_id, {"stopReason": stop})

        except ImportError:
            self.send_error(msg_id, -32603, "需要安装: pip install anthropic")
        except Exception as e:
            LOG.exception("API 调用失败")
            self.send_error(msg_id, -32603, str(e))

    # ── 主循环 ─────────────────────────────────────────────────────

    def run(self):
        LOG.info("Direct API Agent 启动")
        if not self.api_key:
            LOG.warning("ANTHROPIC_API_KEY 未设置，API 调用将失败")

        handlers = {
            "initialize": self.handle_initialize,
            "newSession": self.handle_new_session,
            "closeSession": self.handle_close_session,
            "prompt": self.handle_prompt,
        }

        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                LOG.warning("JSON 解析失败: %s", line[:100])
                continue

            msg_id = msg.get("id")
            method = msg.get("method")

            if msg_id and method:  # 请求
                handler = handlers.get(method)
                if handler:
                    t = threading.Thread(
                        target=handler, args=(msg_id, msg.get("params")), daemon=True
                    )
                    t.start()
                else:
                    self.send_error(msg_id, -32601, f"unknown: {method}")
            elif msg_id and not method:  # 响应
                q = self._pending.pop(str(msg_id), None)
                if q:
                    q.put(msg)
            elif method == "cancel":  # 通知
                self.handle_cancel((msg.get("params") or {}).get("sessionId"))
            elif method == "initialized":
                pass


# ══════════════════════════════════════════════════════════════════════
# 入口
# ══════════════════════════════════════════════════════════════════════


def main():
    import sys

    # 模式选择
    if "--direct" in sys.argv:
        # 直接调用 Anthropic API（不依赖 Claude Code CLI）
        DirectApiAgent().run()
    else:
        # 默认：启动 Claude Code CLI 并双向转发
        proxy = ClaudeCodeProxy()
        try:
            proxy.run()
        except KeyboardInterrupt:
            LOG.info("用户中断")
        except FileNotFoundError as e:
            LOG.error("%s", e)
            print(str(e), file=sys.stderr)
            sys.exit(1)
        finally:
            proxy.stop()


if __name__ == "__main__":
    main()
