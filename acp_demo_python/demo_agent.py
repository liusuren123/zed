"""
ACP 演示 Agent

一个功能丰富的 ACP Agent 示例，展示：
- 流式助手消息
- 工具调用
- 文件操作
- 终端操作
- 权限请求

运行方式：
  python demo_agent.py                     # 直接运行（等待 stdin 消息）
  python demo_agent.py --self-test          # 运行自测试（模拟 Zed 客户端）

配置为 Zed 的外部 Agent：
  在 settings.json 中添加：
  {
    "agent_servers": {
      "python-demo": {
        "command": ["python", "path/to/demo_agent.py"],
        "args": []
      }
    }
  }
"""

import asyncio
import json
import logging
import sys
import time
import uuid
from typing import Any

from acp_protocol import (
    AcpAgent,
    ACPError,
    AcpSession,
    ErrorCode,
    JsonRpcNotification,
    JsonRpcRequest,
    JsonRpcResponse,
    PermissionOption,
    PermissionOptionKind,
    ToolCall,
    ToolCallStatus,
    ToolKind,
    parse_json_rpc,
    run_agent,
)

logger = logging.getLogger("demo_agent")


class DemoAgent(AcpAgent):
    """
    演示 ACP Agent
    """

    def __init__(self, **kwargs):
        super().__init__(
            name=kwargs.pop("name", "python-demo-agent"),
            version=kwargs.pop("version", "0.2.0"),
            **kwargs,
        )
        self.agent_capabilities = {
            "promptCapabilities": {"modes": False, "modelSelection": False},
            "sessionCapabilities": {
                "close": {"supports": True},
                "resume": {"supports": False},
                "list": {"supports": False},
            },
            "loadSession": True,
        }

    async def on_prompt(self, session: AcpSession, messages: list[Any]):
        user_text = self._extract_user_text(messages)
        logger.info(f"收到提示词: {user_text[:100]}...")
        session.messages.append({"role": "user", "content": user_text})
        await asyncio.sleep(0.2)

        lower = user_text.lower().strip()

        if lower in ("hello", "你好"):
            await self._handle_greeting(session)
        elif lower.startswith("echo "):
            await self._handle_echo_tool(session, lower[5:])
        elif lower.startswith("write "):
            await self._handle_write_file(session, lower[6:])
        elif "read" in lower and "file" in lower:
            await self._handle_read_file(session)
        elif "terminal" in lower or "run" in lower:
            await self._handle_terminal(session, user_text)
        elif "think" in lower or "思考" in lower:
            await self._handle_thought(session)
        elif "error" in lower:
            raise ACPError(code=ErrorCode.TOOL_EXECUTION_FAILED, message="模拟错误")
        elif "delay" in lower or "取消" in lower:
            await self._handle_delayed_response(session)
        elif "tool" in lower or "工具" in lower:
            await self._handle_complex_tool(session)
        else:
            await session.send_assistant_chunk(
                "你好！我是 Python ACP 演示 Agent。\n\n"
                f"你说了: **{user_text}**\n\n"
                "试试这些命令:\n"
                "- `hello` / `你好` — 打招呼\n"
                "- `echo <text>` — 工具调用演示\n"
                "- `write <content>` — 文件写入\n"
                "- `read file` — 文件读取\n"
                "- `run <command>` — 终端命令\n"
                "- `think` — 思维过程\n"
                "- `delay` — 取消处理\n"
                "- `error` — 错误处理\n"
                "- `tools` — 工具类型概览\n"
            )

        await session.send_assistant_complete()

    async def _handle_greeting(self, session: AcpSession):
        await session.send_assistant_chunk("你好！👋\n\n")
        await asyncio.sleep(0.3)
        await session.send_assistant_chunk(
            "我是 **ACP Demo Agent**，基于 Python 实现。\n\n"
            "我通过 **Agent Communication Protocol** 与 Zed 通信。\n\n"
            "支持功能:\n"
            "1. 流式文本响应\n"
            "2. 工具调用（执行、读写文件、终端）\n"
            "3. 权限请求\n"
            "4. 取消处理\n"
        )

    async def _handle_echo_tool(self, session: AcpSession, text: str):
        tid = str(uuid.uuid4())
        tc = ToolCall(
            id=tid,
            title=f"echo {text[:30]}",
            kind=ToolKind.EXECUTE,
            content=[
                {
                    "type": "text",
                    "text": json.dumps(
                        {"command": "echo", "args": [text]}, ensure_ascii=False
                    ),
                }
            ],
            status=ToolCallStatus.WAITING_FOR_CONFIRMATION,
        )
        await session.send_tool_call(tc)
        try:
            await session.request_permission(
                tc,
                [
                    PermissionOption(
                        id="allow", label="允许", kind=PermissionOptionKind.ALLOW_ONCE
                    ),
                    PermissionOption(
                        id="deny", label="拒绝", kind=PermissionOptionKind.DENY_ONCE
                    ),
                ],
            )
        except ACPError:
            await session.send_assistant_chunk(f"❌ 用户拒绝执行\n")
            return
        await session.send_tool_call_update(tid, status=ToolCallStatus.IN_PROGRESS)
        await asyncio.sleep(0.5)
        result = f"执行结果:\n```\n{text}\n```\n"
        await session.send_tool_call_update(
            tid,
            status=ToolCallStatus.COMPLETED,
            content=[{"type": "text", "text": result}],
            raw_output=text,
        )
        await session.send_assistant_chunk(f"✅ 命令完成！\n\n{result}")

    async def _handle_write_file(self, session: AcpSession, content: str):
        tid = str(uuid.uuid4())
        path = f"/tmp/demo_{int(time.time())}.txt"
        tc = ToolCall(
            id=tid,
            title=f"写入 {path}",
            kind=ToolKind.WRITE,
            status=ToolCallStatus.WAITING_FOR_CONFIRMATION,
        )
        await session.send_tool_call(tc)
        try:
            await session.request_permission(
                tc,
                [
                    PermissionOption(
                        id="allow", label="允许", kind=PermissionOptionKind.ALLOW_ONCE
                    ),
                    PermissionOption(
                        id="deny", label="拒绝", kind=PermissionOptionKind.DENY_ONCE
                    ),
                ],
            )
        except ACPError:
            await session.send_assistant_chunk("❌ 写入被拒绝\n")
            return
        await session.send_tool_call_update(tid, status=ToolCallStatus.IN_PROGRESS)
        await session.write_text_file(path, content)
        await session.send_tool_call_update(
            tid, status=ToolCallStatus.COMPLETED, raw_output=f"写入 {len(content)} 字节"
        )
        await session.send_assistant_chunk(
            f"✅ 文件写入: `{path}`\n内容: {content[:100]}"
        )
        await session.send_tool_call_complete(tid)

    async def _handle_read_file(self, session: AcpSession):
        tid = str(uuid.uuid4())
        await session.send_tool_call(
            ToolCall(
                id=tid,
                title="读取文件",
                kind=ToolKind.READ,
                status=ToolCallStatus.IN_PROGRESS,
            )
        )
        await asyncio.sleep(0.5)
        await session.send_tool_call_update(tid, status=ToolCallStatus.COMPLETED)
        await session.send_assistant_chunk(
            "📖 文件读取演示\n\n"
            "实际场景中，ACP Agent 通过 `readTextFile` 请求\n"
            "让 Zed 读取文件内容并返回。"
        )

    async def _handle_terminal(self, session: AcpSession, user_text: str):
        cmd = user_text
        for prefix in ["terminal ", "run "]:
            if cmd.lower().startswith(prefix):
                cmd = cmd[len(prefix) :]
        tid = str(uuid.uuid4())
        tc = ToolCall(
            id=tid,
            title=cmd[:50],
            kind=ToolKind.EXECUTE,
            content=[
                {
                    "type": "text",
                    "text": json.dumps({"command": cmd}, ensure_ascii=False),
                }
            ],
            status=ToolCallStatus.WAITING_FOR_CONFIRMATION,
        )
        await session.send_tool_call(tc)
        try:
            await session.request_permission(
                tc,
                [
                    PermissionOption(
                        id="allow", label="允许", kind=PermissionOptionKind.ALLOW_ONCE
                    ),
                    PermissionOption(
                        id="deny", label="拒绝", kind=PermissionOptionKind.DENY_ONCE
                    ),
                ],
            )
        except ACPError:
            await session.send_assistant_chunk("❌ 终端命令被拒绝\n")
            return
        await session.send_tool_call_update(tid, status=ToolCallStatus.IN_PROGRESS)
        try:
            info = await session.create_terminal(
                cmd.split()[0] if cmd.split() else "sh",
                cmd.split()[1:] if len(cmd.split()) > 1 else None,
                session.cwd,
            )
            await session.send_tool_call_update(tid, status=ToolCallStatus.COMPLETED)
            await session.send_assistant_chunk(
                f"✅ 终端已创建: {info.get('terminalId', '?')}"
            )
        except Exception:
            await session.send_assistant_chunk("⚠️ 终端创建模拟（演示模式）")

    async def _handle_thought(self, session: AcpSession):
        await session.send_assistant_chunk("让我思考一下...\n\n")
        for t in ["🤔 理解需求...", "💭 分析方案...", "📊 评估...", "✅ 准备回答..."]:
            await session.send_assistant_chunk(f"{t}\n")
            await asyncio.sleep(0.5)
        await session.send_assistant_chunk("\n**结论:** 这是一个思维过程演示。")

    async def _handle_delayed_response(self, session: AcpSession):
        await session.send_assistant_chunk("开始处理 (共3步)...\n\n")
        for i in range(3):
            for tick in range(10):
                if session.is_cancelled:
                    await session.send_assistant_chunk(
                        "\n\n已被取消，可发送新消息继续。"
                    )
                    logger.info(f"延迟任务 step={i + 1} 取消")
                    return
                await asyncio.sleep(0.1)
            await session.send_assistant_chunk(f"步骤 {i + 1}/3...\n")
        await session.send_assistant_chunk("\n全部完成！")

    async def _handle_complex_tool(self, session: AcpSession):
        await session.send_assistant_chunk("ACP 工具类型:\n\n")
        for name, kind, desc in [
            ("🔧 执行", ToolKind.EXECUTE, "运行命令"),
            ("📝 编辑", ToolKind.EDIT, "修改文件"),
            ("📖 读取", ToolKind.READ, "读取文件"),
            ("✏️ 写入", ToolKind.WRITE, "创建文件"),
            ("🔍 搜索", ToolKind.SEARCH, "搜索代码"),
            ("💭 思维", ToolKind.THINK, "中间思考"),
        ]:
            await session.send_assistant_chunk(f"{name} ({kind.value}): {desc}\n")
            await asyncio.sleep(0.2)
        await session.send_assistant_chunk(
            "\n生命周期: pending → inProgress → completed/failed/cancelled"
        )

    def _extract_user_text(self, messages: list[Any]) -> str:
        texts: list[str] = []
        for msg in messages:
            if isinstance(msg, dict):
                t = msg.get("text", "")
                if isinstance(t, str) and t.strip():
                    texts.append(t)
            elif isinstance(msg, str):
                texts.append(msg)
        return " ".join(texts).strip() or "(空消息)"


# ─── 自测试 ───────────────────────────────────────────────────────────


class MockZedClient:
    """模拟 Zed ACP 客户端"""

    def __init__(self, to_agent_q: asyncio.Queue, from_agent_q: asyncio.Queue):
        self._to_agent = to_agent_q
        self._from_agent = from_agent_q
        self._pending: dict[int, asyncio.Future] = {}
        self._id_counter = 0

    async def _send_to_agent(self, msg):
        await self._to_agent.put(json.dumps(msg.to_dict(), ensure_ascii=False) + "\n")

    async def send_request(self, method: str, params=None) -> Any:
        self._id_counter += 1
        future = asyncio.get_event_loop().create_future()
        self._pending[self._id_counter] = future
        await self._send_to_agent(
            JsonRpcRequest(method=method, params=params, id=self._id_counter)
        )
        return await asyncio.wait_for(future, timeout=30)

    async def send_notification(self, method: str, params=None):
        await self._send_to_agent(JsonRpcNotification(method=method, params=params))

    async def _handle_agent_message(self, line: str):
        msg = parse_json_rpc(line)
        if not msg:
            return
        if isinstance(msg, JsonRpcResponse):
            fut = self._pending.get(msg.id)
            if fut:
                if msg.error:
                    fut.set_exception(
                        ACPError(msg.error.code, msg.error.message, msg.error.data)
                    )
                else:
                    fut.set_result(msg.result)
                del self._pending[msg.id]
        elif isinstance(msg, JsonRpcRequest):
            await self._handle_agent_request(msg)
        elif isinstance(msg, JsonRpcNotification):
            if msg.method == "session/update":
                u = (msg.params or {}).get("update", {})
                logger.info(f"  [会话更新] type={u.get('type')}")

    async def _handle_agent_request(self, req: JsonRpcRequest):
        p = req.params or {}
        logger.info(f"  [Agent请求] {req.method}: {str(p)[:100]}")
        if req.method == "requestPermission":
            allow_id = next(
                (
                    o["id"]
                    for o in p.get("options", [])
                    if o.get("kind") in ("allowOnce", "allowForever")
                ),
                None,
            )
            resp = (
                JsonRpcResponse(
                    id=req.id, result={"optionId": allow_id, "optionKind": "allowOnce"}
                )
                if allow_id
                else JsonRpcResponse(
                    id=req.id, error=ACPError(ErrorCode.INTERNAL_ERROR, "无允许选项")
                )
            )
        elif req.method == "writeTextFile":
            logger.info(f"    -> 写入 {p.get('path')}")
            resp = JsonRpcResponse(id=req.id, result={})
        elif req.method == "readTextFile":
            resp = JsonRpcResponse(id=req.id, result={"content": "模拟文件内容\n"})
        elif req.method == "createTerminal":
            resp = JsonRpcResponse(
                id=req.id, result={"terminalId": f"term-{uuid.uuid4().hex[:8]}"}
            )
        else:
            resp = JsonRpcResponse(id=req.id, result={})
        await self._send_to_agent(resp)

    async def run(self):
        print("=" * 60)
        print("  ACP 协议自测试")
        print("=" * 60)

        # 1. 初始化
        print("\n[1/5] 初始化...")
        r = await self.send_request(
            "initialize",
            {
                "protocolVersion": "v1",
                "clientCapabilities": {
                    "fs": {"readTextFile": True, "writeTextFile": True},
                    "terminal": True,
                    "auth": {"terminal": True},
                },
                "clientInfo": {"name": "mock-zed", "version": "1.0.0"},
            },
        )
        ai = r.get("agentInfo", {})
        print(f"    ✓ Agent: {ai.get('name')} v{ai.get('version')}")
        await self.send_notification("initialized")

        # 2. 创建会话
        print("\n[2/5] 创建会话...")
        r = await self.send_request("newSession", {"cwd": "/home/user/project"})
        sid = r.get("sessionId")
        print(f"    ✓ 会话: {sid}")

        # 3. 基本提示词
        print("\n[3/5] 基本提示词...")
        r = await self.send_request(
            "prompt",
            {"sessionId": sid, "messages": [{"type": "text", "text": "hello"}]},
        )
        print(f"    ✓ 停止原因: {r.get('stopReason')}")

        # 4. 工具调用
        print("\n[4/5] 工具调用...")
        r = await self.send_request(
            "prompt",
            {
                "sessionId": sid,
                "messages": [{"type": "text", "text": "echo Hello ACP!"}],
            },
        )
        print(f"    ✓ 停止原因: {r.get('stopReason')}")

        # 5. 取消
        print("\n[5/5] 取消测试...")
        import time as _time

        start_ts = _time.time()
        task = asyncio.create_task(
            self.send_request(
                "prompt",
                {"sessionId": sid, "messages": [{"type": "text", "text": "delay"}]},
            )
        )
        await asyncio.sleep(2)
        print(f"    -> 发送取消 (t+{_time.time() - start_ts:.1f}s)")
        await self.send_notification("cancel", {"sessionId": sid})
        try:
            r = await asyncio.wait_for(task, 15)
            print(
                f"    ✓ 停止原因: {r.get('stopReason')} (t+{_time.time() - start_ts:.1f}s)"
            )
        except asyncio.TimeoutError:
            print(f"    ✗ 超时 (t+{_time.time() - start_ts:.1f}s)")

        # 关闭
        print(f"关闭会话... (t+{_time.time() - start_ts:.1f}s)")
        await self.send_request("closeSession", {"sessionId": sid})
        print("\n" + "=" * 60)
        print("  测试完成!")
        print("=" * 60)


async def self_test():
    to_agent: asyncio.Queue[str] = asyncio.Queue()
    from_agent: asyncio.Queue[str] = asyncio.Queue()

    agent = DemoAgent()
    agent._read_line = lambda: to_agent.get()
    agent._send = lambda data: from_agent.put(data)

    mock = MockZedClient(to_agent, from_agent)

    async def agent_recv_loop():
        """后台任务：持续把 agent 发出的消息传给 mock"""
        while True:
            line = await from_agent.get()
            await mock._handle_agent_message(line)

    agent_task = asyncio.create_task(agent.run())
    recv_task = asyncio.create_task(agent_recv_loop())

    try:
        await mock.run()
    finally:
        agent.stop()
        agent_task.cancel()
        recv_task.cancel()
        for t in (agent_task, recv_task):
            try:
                await t
            except (asyncio.CancelledError, EOFError):
                pass


def main():
    if "--self-test" in sys.argv:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s [%(levelname)s] %(message)s",
            stream=sys.stderr,
        )
        asyncio.run(self_test())
    else:
        run_agent(DemoAgent)


if __name__ == "__main__":
    main()
