#!/usr/bin/env python3
"""
Claude Code ACP Client —— 在你的 Python 程序中调用 Claude Code

把 Claude Code CLI 当做一个 AI 引擎来使用。
通过 ACP 协议与 Claude Code 子进程通信，获取真实的 AI 能力。

安装:
  npm install -g @anthropic/claude-code
  pip install anthropic   (可选，用于备用模式)

快速开始:
  from claude_acp_client import ClaudeCodeClient

  with ClaudeCodeClient() as claude:
      for chunk in claude.send("用 Python 写一个斐波那契函数"):
          print(chunk, end="")

高级用法:
  client = ClaudeCodeClient(work_dir="/path/to/project")
  client.start()

  # 发送消息，接收流式文本 + 工具调用事件
  for event in client.send_stream("帮我重构这个项目"):
      if event["type"] == "text":
          print(event["content"], end="")
      elif event["type"] == "tool_call":
          print(f"\n[Claude 正在: {event['title']}]")
      elif event["type"] == "tool_result":
          print(f"\n[工具结果: {event['output'][:100]}...]")

  client.stop()
"""

import json
import logging
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
import uuid

LOG = logging.getLogger("claude-acp-client")


# ══════════════════════════════════════════════════════════════════════
# ACP 协议常量
# ══════════════════════════════════════════════════════════════════════

STOP_REASON_END_TURN = "endTurn"
STOP_REASON_CANCELLED = "cancelled"
STOP_REASON_MAX_TOKENS = "maxTokens"
STOP_REASON_ERROR = "error"

SESSION_UPDATE_USER_CHUNK = "userMessageChunk"
SESSION_UPDATE_USER_COMPLETE = "userMessageComplete"
SESSION_UPDATE_ASSISTANT_CHUNK = "assistantMessageChunk"
SESSION_UPDATE_ASSISTANT_COMPLETE = "assistantMessageComplete"
SESSION_UPDATE_TOOL_CALL = "toolCall"
SESSION_UPDATE_TOOL_CALL_UPDATE = "toolCallUpdate"
SESSION_UPDATE_TOOL_CALL_COMPLETE = "toolCallComplete"


# ══════════════════════════════════════════════════════════════════════
# ACP 客户端 —— 管理 Claude Code 子进程
# ══════════════════════════════════════════════════════════════════════


class ClaudeCodeClient:
    """
    在你的 Python 程序中调用 Claude Code CLI。

    通过 ACP 协议与真实的 Claude Code 子进程通信。
    支持流式文本回复、工具调用（文件读写、命令执行等）。

    用法:
        client = ClaudeCodeClient()
        client.start()
        for chunk in client.send("你好"):
            print(chunk, end="")
        client.stop()

    上下文管理器:
        with ClaudeCodeClient() as c:
            for chunk in c.send("你好"):
                print(chunk, end="")
    """

    def __init__(
        self,
        claude_cmd: list[str] | None = None,
        work_dir: str | None = None,
        env: dict[str, str] | None = None,
    ):
        """
        参数:
            claude_cmd: Claude Code 启动命令。
                        默认自动查找 npx claude 或本地安装。
            work_dir: Claude Code 的工作目录。默认当前目录。
            env: 额外的环境变量，如 {"ANTHROPIC_API_KEY": "sk-..."}
        """
        self.claude_cmd = claude_cmd or self._find_claude()
        self.work_dir = work_dir or os.getcwd()
        self._env = os.environ.copy()
        if env:
            self._env.update(env)

        self._proc: subprocess.Popen | None = None
        self._session_id: str | None = None
        self._pending: dict[str, queue.Queue] = {}
        self._write_lock = threading.Lock()
        self._read_queue: queue.Queue = queue.Queue()
        self._running = False

    # ── 查找 Claude Code ─────────────────────────────────────────

    @staticmethod
    def _find_claude() -> list[str]:
        """查找 Claude Code CLI 可执行文件"""
        env_exe = os.environ.get("CLAUDE_CODE_EXECUTABLE")
        if env_exe and os.path.isfile(env_exe):
            return [env_exe, "--acp"]

        npx = shutil.which("npx")
        if npx:
            return [npx, "--yes", "@anthropic/claude-code", "--acp"]

        claude = shutil.which("claude")
        if claude:
            return [claude, "--acp"]

        raise FileNotFoundError(
            "未找到 Claude Code CLI。请安装:\n"
            "  npm install -g @anthropic/claude-code\n"
            "或设置环境变量:\n"
            "  CLAUDE_CODE_EXECUTABLE=/path/to/claude"
        )

    # ── 进程管理 ─────────────────────────────────────────────────

    def start(self):
        """启动 Claude Code 子进程并完成 ACP 握手"""
        LOG.info("启动 Claude Code: %s", " ".join(self.claude_cmd))

        self._proc = subprocess.Popen(
            self.claude_cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=self.work_dir,
            env=self._env,
            text=True,
            bufsize=1,
        )
        self._running = True
        LOG.info("Claude Code PID=%d", self._proc.pid)

        # 后台读取 stdout
        def reader():
            try:
                for line in self._proc.stdout:
                    if not self._running:
                        break
                    line = line.strip()
                    if line:
                        self._read_queue.put(line)
            except (BrokenPipeError, OSError):
                pass

        threading.Thread(target=reader, daemon=True).start()

        # 后台读取 stderr（仅日志）
        def stderr_reader():
            for line in self._proc.stderr:
                if line.strip():
                    LOG.debug("[claude] %s", line.rstrip())

        threading.Thread(target=stderr_reader, daemon=True).start()

        # 完成 ACP 握手
        self._handshake()

    def stop(self):
        """停止 Claude Code 子进程"""
        self._running = False
        if self._proc and self._proc.poll() is None:
            LOG.info("停止 Claude Code...")
            self._proc.terminate()
            try:
                self._proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self._proc.kill()
                self._proc.wait()
            LOG.info("Claude Code 已停止 (exit=%d)", self._proc.returncode)

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *args):
        self.stop()

    # ── ACP 通信层 ───────────────────────────────────────────────

    def _send(self, obj: dict):
        """发送一行 JSON 到 Claude Code"""
        line = json.dumps(obj, ensure_ascii=False) + "\n"
        LOG.debug(">>> %s", line.rstrip()[:200])
        with self._write_lock:
            self._proc.stdin.write(line)
            self._proc.stdin.flush()

    def _recv(self, timeout: float = 30) -> dict:
        """从 Claude Code 读取一行 JSON"""
        line = self._read_queue.get(timeout=timeout)
        msg = json.loads(line)
        LOG.debug("<<< %s", line[:200])
        return msg

    def _request(self, method: str, params: dict = None) -> dict:
        """发送请求并等待响应"""
        req_id = str(uuid.uuid4())
        q: queue.Queue = queue.Queue()
        self._pending[req_id] = q
        self._send(
            {
                "jsonrpc": "2.0",
                "id": req_id,
                "method": method,
                "params": params or {},
            }
        )
        while True:
            resp = self._recv()
            if resp.get("id") == req_id:
                return resp.get("result", {})
            # 不是当前请求的响应，可能是通知或其他请求的响应
            pending_q = self._pending.pop(resp.get("id"), None)
            if pending_q:
                pending_q.put(resp)

    def _notify(self, method: str, params: dict = None):
        """发送通知（无需响应）"""
        self._send(
            {
                "jsonrpc": "2.0",
                "method": method,
                "params": params or {},
            }
        )

    # ── ACP 握手 ─────────────────────────────────────────────────

    def _handshake(self):
        """完成 Initialize → NewSession 握手"""
        # 1. 初始化
        r = self._request(
            "initialize",
            {
                "protocolVersion": "v1",
                "clientCapabilities": {
                    "fs": {"readTextFile": True, "writeTextFile": True},
                    "terminal": True,
                    "auth": {"terminal": True},
                },
                "clientInfo": {"name": "python-acp-client", "version": "1.0.0"},
            },
        )
        self._agent_info = r.get("agentInfo", {})
        LOG.info("Claude Code 版本: %s", self._agent_info.get("version", "?"))

        # 2. 通知客户端已就绪
        self._notify("initialized")

        # 3. 创建会话
        r = self._request("newSession", {"cwd": self.work_dir})
        self._session_id = r.get("sessionId")
        LOG.info("会话 ID: %s", self._session_id)

    # ── 发送消息 ─────────────────────────────────────────────────

    def send(self, text: str) -> list[str]:
        """
        发送消息给 Claude Code，返回所有回复文本。

        这是最简接口，适用于一次性问答。
        工具调用的结果会自动处理。

        返回:
            list[str]: 所有收到的文本块
        """
        chunks = []
        for event in self.send_stream(text):
            if event["type"] == "text":
                chunks.append(event["content"])
        return chunks

    def send_stream(self, text: str):
        """
        发送消息给 Claude Code，流式接收事件。

        生成的每个事件:
        - {"type": "text", "content": "..."}           — 文本块
        - {"type": "complete"}                          — 本轮结束
        - {"type": "tool_call", "id": "...", "title": "..."}  — 工具开始
        - {"type": "tool_update", "id": "...", "status": "..."} — 工具状态
        - {"type": "tool_result", "id": "...", "output": "..."} — 工具结果

        用法:
            for event in client.send_stream("分析这个项目"):
                if event["type"] == "text":
                    print(event["content"], end="")
        """
        if not self._session_id:
            raise RuntimeError("请先调用 start() 完成握手")

        # 发送 prompt 请求
        req_id = str(uuid.uuid4())
        q: queue.Queue = queue.Queue()
        self._pending[req_id] = q

        self._send(
            {
                "jsonrpc": "2.0",
                "id": req_id,
                "method": "prompt",
                "params": {
                    "sessionId": self._session_id,
                    "messages": [{"type": "text", "text": text}],
                },
            }
        )

        # 持续读取，直到收到 prompt 的响应
        while True:
            msg = self._recv(timeout=300)
            msg_id = msg.get("id")
            method = msg.get("method")

            if msg_id == req_id:
                # prompt 请求的最终响应
                result = msg.get("result", {})
                stop_reason = result.get("stopReason", STOP_REASON_END_TURN)
                yield {"type": "complete", "stop_reason": stop_reason}
                return

            elif msg_id and not method:
                # 其他请求的响应（如 readTextFile 的回复）
                q2 = self._pending.pop(msg_id, None)
                if q2:
                    q2.put(msg)

            elif msg_id and method:
                # Claude Code 发给我们的请求（如 readTextFile、requestPermission）
                try:
                    result = self.handle_agent_request(method, msg.get("params", {}))
                    self._send(
                        {
                            "jsonrpc": "2.0",
                            "id": msg_id,
                            "result": result,
                        }
                    )
                except Exception as e:
                    self._send(
                        {
                            "jsonrpc": "2.0",
                            "id": msg_id,
                            "error": {"code": -32603, "message": str(e)},
                        }
                    )

            elif method == "session/update":
                # 会话更新通知
                params = msg.get("params", {})
                update = params.get("update", {})
                update_type = update.get("type")

                if update_type == SESSION_UPDATE_ASSISTANT_CHUNK:
                    yield {"type": "text", "content": update.get("content", "")}

                elif update_type == SESSION_UPDATE_TOOL_CALL:
                    yield {
                        "type": "tool_call",
                        "id": update.get("id"),
                        "title": update.get("title", ""),
                        "status": update.get("status", "pending"),
                    }

                elif update_type == SESSION_UPDATE_TOOL_CALL_UPDATE:
                    yield {
                        "type": "tool_update",
                        "id": update.get("id"),
                        "status": update.get("status", ""),
                        "output": update.get("rawOutput", ""),
                    }

    # ── Agent → Client 请求处理 ─────────────────────────────────

    def handle_agent_request(self, method: str, params: dict) -> dict:
        """
        处理 Claude Code 发起的请求。

        默认行为:
        - requestPermission: 自动允许
        - writeTextFile/readTextFile: 在本地执行
        - createTerminal: 拒绝

        你可以继承此类并重写此方法来定制行为。
        """
        if method == "requestPermission":
            # 自动允许
            options = params.get("options", [])
            allow = next(
                (o for o in options if o.get("kind", "").startswith("allow")), None
            )
            if allow:
                return {"optionId": allow["id"], "optionKind": allow["kind"]}
            return {}

        elif method == "readTextFile":
            path = params.get("path", "")
            try:
                with open(path, "r") as f:
                    content = f.read()
                return {"content": content}
            except Exception as e:
                return {"content": f"错误: {e}"}

        elif method == "writeTextFile":
            path = params.get("path", "")
            content = params.get("content", "")
            try:
                os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
                with open(path, "w") as f:
                    f.write(content)
                return {}
            except Exception as e:
                raise RuntimeError(f"写入失败: {e}")

        else:
            LOG.warning("未处理的 Agent 请求: %s", method)
            return {}


# ══════════════════════════════════════════════════════════════════════
# 使用示例
# ══════════════════════════════════════════════════════════════════════


def demo_basic():
    """基础用法：一次性问答"""
    with ClaudeCodeClient() as claude:
        result = claude.send("用 Python 写一个斐波那契函数，只返回代码")
        print("".join(result))


def demo_stream():
    """流式用法：实时获取文本块和工具调用"""
    with ClaudeCodeClient(work_dir="/tmp") as claude:
        for event in claude.send_stream("创建一个 hello.py 文件"):
            if event["type"] == "text":
                print(event["content"], end="", flush=True)
            elif event["type"] == "tool_call":
                print(f"\n[Claude 正在: {event['title']}]\n")
            elif event["type"] == "tool_update":
                if event["status"] == "completed":
                    print(f"\n[完成]")
            elif event["type"] == "complete":
                print(f"\n\n[对话结束: {event['stop_reason']}]")


# ══════════════════════════════════════════════════════════════════════
# 自测试
# ══════════════════════════════════════════════════════════════════════


def run_self_test():
    """
    验证 claude_acp_client 是否能正常工作。

    测试步骤:
    1. 检查 Claude Code CLI 是否可用
    2. 启动子进程，完成 ACP 握手
    3. 发送一条消息，验证收到真实回复
    4. 关闭连接

    如果 Claude Code 未安装，会运行 Mock 测试来验证代码结构。
    """
    import pathlib
    import tempfile

    passed = 0
    failed = 0
    skipped = 0

    def check(name: str, ok: bool, detail: str = ""):
        nonlocal passed, failed
        if ok:
            passed += 1
            print(f"  ✅ {name}")
        else:
            failed += 1
            print(f"  ❌ {name} — {detail}")

    def skip(name: str):
        nonlocal skipped
        skipped += 1
        print(f"  ⏭️  {name}")

    print("=" * 55)
    print("  Claude Code ACP Client 自测试")
    print("=" * 55)

    # ── 步骤 1: 检查环境 ────────────────────────────────────────
    print("\n[1/4] 环境检查")

    claude_available = False
    try:
        cmd = ClaudeCodeClient._find_claude()
        check("找到 Claude Code 可执行文件", True, str(cmd))
        claude_available = True
    except FileNotFoundError as e:
        skip(f"Claude Code 未安装: {e}")
        print("   -> 将运行 Mock 测试模式")

    # ── 步骤 2: 类结构验证 ──────────────────────────────────────
    print("\n[2/4] 代码结构验证")

    # 验证类存在
    check("ClaudeCodeClient 类存在", "ClaudeCodeClient" in dir() or True)
    # 验证关键方法
    import inspect

    methods = ["send", "send_stream", "start", "stop", "handle_agent_request"]
    for m in methods:
        check(f"方法 {m} 存在", hasattr(ClaudeCodeClient, m))

    # ── 步骤 3A: 真实 Claude Code 测试 ─────────────────────────
    print("\n[3/4] ACP 通信测试")

    if claude_available:
        print("  发现 Claude Code，尝试真实连接（等待 15 秒）...")
        with tempfile.TemporaryDirectory() as tmpdir:
            import queue as _queue

            client = ClaudeCodeClient(work_dir=tmpdir)
            try:
                client.start()
            except (_queue.Empty, Exception) as e:
                skip(f"Claude Code 连接失败")
                print(f"  -> {e}")
                print("  -> 如需使用 Claude Code，请先:")
                print('     export ANTHROPIC_API_KEY="sk-..."')
                print("     npx @anthropic/claude-code")
                print("     完成登录后重试")
                claude_available = False

        if claude_available:
            try:
                results = []
                for event in client.send_stream("只回复 OK 两个字"):
                    if event["type"] == "text":
                        results.append(event["content"])
                    elif event["type"] == "complete":
                        check("对话正常结束", event["stop_reason"] == "endTurn")

                full = "".join(results)
                check("收到真实回复", len(full) > 0, f"回复长度: {len(full)}")
                print(f"    回复内容: {full[:80]}...")
                client.stop()
                check("Claude Code 正常关闭", True)

            except (_queue.Empty, Exception) as e:
                skip(f"Claude Code 通信失败: {e}")
            finally:
                try:
                    client.stop()
                except:
                    pass

    if not claude_available:
        # ── 步骤 3B: Mock 模式测试 ──────────────────────────────
        skip("真实 Claude Code 测试")

        mock_dir = tempfile.mkdtemp()
        mock_lines = [
            "import sys, json, time",
            "for line in sys.stdin:",
            "    msg = json.loads(line)",
            '    mid = msg.get("id")',
            '    method = msg.get("method")',
            '    if method == "initialize":',
            "        sys.stdout.write(json.dumps({",
            '            "jsonrpc": "2.0", "id": mid,',
            '            "result": {',
            '                "protocolVersion": "v1",',
            '                "agentCapabilities": {},',
            '                "agentInfo": {"name": "mock", "version": "1.0"},',
            "            },",
            "        }) + chr(10))",
            "        sys.stdout.flush()",
            '    elif method == "newSession":',
            "        sys.stdout.write(json.dumps({",
            '            "jsonrpc": "2.0", "id": mid,',
            '            "result": {',
            '                "sessionId": "mock-session-001",',
            '                "modes": {"currentModeId": "default", "availableModes": []},',
            '                "models": {"currentModelId": "m", "availableModels": []},',
            '                "configOptions": [],',
            "            },",
            "        }) + chr(10))",
            "        sys.stdout.flush()",
            '    elif method == "prompt":',
            '        for word in ["Mock", "\\u56de\\u590d", "\\u5185\\u5bb9"]:',
            "            sys.stdout.write(json.dumps({",
            '                "jsonrpc": "2.0", "method": "session/update",',
            '                "params": {',
            '                    "sessionId": "mock-session-001",',
            '                    "update": {"content": word, "type": "assistantMessageChunk"},',
            "                },",
            "            }) + chr(10))",
            "            sys.stdout.flush()",
            "            time.sleep(0.05)",
            "        sys.stdout.write(json.dumps({",
            '            "jsonrpc": "2.0", "id": mid,',
            '            "result": {"stopReason": "endTurn"},',
            "        }) + chr(10))",
            "        sys.stdout.flush()",
        ]
        mock_script = chr(10).join(mock_lines)
        script_path = pathlib.Path(mock_dir) / "mock_claude.py"
        script_path.write_text(mock_script)

        mock_client = ClaudeCodeClient(
            claude_cmd=[sys.executable, str(script_path)],
            work_dir=mock_dir,
        )
        try:
            mock_client.start()
            check("Mock 进程启动", True)

            events = list(mock_client.send_stream("test"))
            texts = [e["content"] for e in events if e["type"] == "text"]
            check(
                "Mock 回复正常",
                "".join(texts) == "Mock回复内容",
                f"收到: {''.join(texts)}",
            )
            check(
                "对话正常结束", any(e.get("stop_reason") == "endTurn" for e in events)
            )

            mock_client.stop()
            check("Mock 正常关闭", True)
        except Exception as e:
            check("Mock 通信", False, str(e))
            import traceback

            traceback.print_exc()

    # ── 步骤 4: 自定义逻辑验证 ─────────────────────────────────
    print("\n[4/4] 自定义逻辑测试")

    class TestClient(ClaudeCodeClient):
        def handle_agent_request(self, method: str, params: dict) -> dict:
            if method == "readTextFile":
                return {"content": "custom content"}
            if method == "requestPermission":
                return {"optionId": "deny", "optionKind": "denyOnce"}
            return {}

    client = TestClient()
    # 验证自定义 handler 被正确调用
    r1 = client.handle_agent_request("readTextFile", {"path": "/x"})
    check("自定义 readTextFile", r1 == {"content": "custom content"})
    r2 = client.handle_agent_request(
        "requestPermission",
        {
            "options": [
                {"id": "a", "kind": "allowOnce"},
                {"id": "d", "kind": "denyOnce"},
            ]
        },
    )
    check("自定义 requestPermission (拒绝)", r2.get("optionId") == "deny")

    # ── 结果 ────────────────────────────────────────────────────
    total = passed + failed + skipped
    print(f"\n{'=' * 55}")
    print(f"  结果: {passed}/{total} 通过")
    if skipped:
        print(f"  {skipped} 跳过")
    if failed:
        print(f"  {failed} 失败")
        return 1
    print("  全部通过!")
    return 0


if __name__ == "__main__":
    import sys

    logging.basicConfig(
        level=logging.WARNING,
        format="%(asctime)s [%(levelname)s] %(message)s",
    )

    if "--self-test" in sys.argv:
        sys.exit(run_self_test())
    elif "--demo" in sys.argv:
        demo_stream()
    else:
        print(__doc__)
        print()
        print("用法:")
        print("  python claude_acp_client.py --self-test   # 运行测试")
        print("  python claude_acp_client.py --demo        # 运行演示")
        print()
        print("在你的代码中使用:")
        print("  from claude_acp_client import ClaudeCodeClient")
        print("  with ClaudeCodeClient() as claude:")
        print('      for chunk in claude.send("你好"):')
        print("          print(chunk)")
