"""
ACP (Agent Communication Protocol) — Python 实现

这是 Zed 使用的 Agent Communication Protocol 的 Python 实现。
ACP 是基于 JSON-RPC 2.0 的协议，通过 stdin/stdout 行分隔 JSON 消息通信。

协议流程：
  1. 初始化握手 (initialize/initialized)
  2. 会话创建 (newSession)
  3. 提示词交换 (prompt + session/update 通知)
  4. 工具调用 (agent -> client 双向请求)
  5. 会话关闭 (closeSession)

参考：Zed 编辑器中的 agent-client-protocol crate (v0.11.1)
"""

from __future__ import annotations

import asyncio
import json
import logging
import sys
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Optional

logger = logging.getLogger("acp")

# ─── 协议常量 ────────────────────────────────────────────────────────────

PROTOCOL_VERSION = "v1"
MINIMUM_SUPPORTED_VERSION = "v1"
INBOX_MAX_SIZE = 4096


# ─── 枚举 & 数据结构 ──────────────────────────────────────────────────────


class StopReason(str, Enum):
    """提示词停止原因"""

    END_TURN = "endTurn"
    CANCELLED = "cancelled"
    MAX_TOKENS = "maxTokens"
    STOP_SEQUENCE = "stopSequence"
    ERROR = "error"


class ToolKind(str, Enum):
    """工具类型"""

    EXECUTE = "execute"
    EDIT = "edit"
    READ = "read"
    WRITE = "write"
    SEARCH = "search"
    THINK = "think"
    COMPUTER = "computer"


class ToolCallStatus(str, Enum):
    """工具调用状态"""

    PENDING = "pending"
    IN_PROGRESS = "inProgress"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    WAITING_FOR_CONFIRMATION = "waitingForConfirmation"


class PermissionOptionKind(str, Enum):
    """权限选项类型"""

    ALLOW_ONCE = "allowOnce"
    ALLOW_FOREVER = "allowForever"
    DENY_ONCE = "denyOnce"
    DENY_FOREVER = "denyForever"


class SessionUpdateType(str, Enum):
    """会话更新类型"""

    USER_MESSAGE_CHUNK = "userMessageChunk"
    USER_MESSAGE_COMPLETE = "userMessageComplete"
    ASSISTANT_MESSAGE_CHUNK = "assistantMessageChunk"
    ASSISTANT_MESSAGE_COMPLETE = "assistantMessageComplete"
    TOOL_CALL = "toolCall"
    TOOL_CALL_UPDATE = "toolCallUpdate"
    TOOL_CALL_COMPLETE = "toolCallComplete"
    CURRENT_MODE_UPDATE = "currentModeUpdate"
    CONFIG_OPTION_UPDATE = "configOptionUpdate"
    SESSION_INFO_UPDATE = "sessionInfoUpdate"
    TOOL_CALL_STREAM = "toolCallStream"


class ErrorCode:
    """ACP 错误码（基于 JSON-RPC 标准扩展）"""

    PARSE_ERROR = -32700
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603
    AUTH_REQUIRED = -32001
    SESSION_NOT_FOUND = -32002
    TOOL_EXECUTION_FAILED = -32003


# ─── 数据类 ──────────────────────────────────────────────────────────────────


@dataclass
class ContentChunk:
    """内容块"""

    content: str
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        d = {"content": self.content}
        if self.metadata:
            d["metadata"] = self.metadata
        return d

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ContentChunk":
        return cls(
            content=data.get("content", ""),
            metadata=data.get("metadata", {}),
        )


@dataclass
class ContentBlock:
    """内容块（文本或资源链接等）"""

    type: str = "text"
    text: str = ""
    resource: Optional[dict[str, Any]] = None

    def to_dict(self) -> dict[str, Any]:
        if self.type == "text":
            return {"type": "text", "text": self.text}
        return {
            "type": self.type,
            "text": self.text,
            **({"resource": self.resource} if self.resource else {}),
        }

    @classmethod
    def text_block(cls, text: str) -> "ContentBlock":
        return cls(type="text", text=text)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ContentBlock":
        return cls(
            type=data.get("type", "text"),
            text=data.get("text", ""),
            resource=data.get("resource"),
        )


@dataclass
class ToolCallLocation:
    """工具调用位置"""

    uri: str
    range: Optional[dict[str, Any]] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"uri": self.uri}
        if self.range:
            d["range"] = self.range
        return d

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ToolCallLocation":
        return cls(uri=data["uri"], range=data.get("range"))


@dataclass
class ToolCall:
    """工具调用"""

    id: str
    title: str
    kind: ToolKind = ToolKind.EXECUTE
    content: list[dict[str, Any]] = field(default_factory=list)
    status: ToolCallStatus = ToolCallStatus.PENDING
    locations: list[ToolCallLocation] = field(default_factory=list)
    meta: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "title": self.title,
            "kind": self.kind.value,
            "content": self.content,
            "status": self.status.value,
            "locations": [l.to_dict() for l in self.locations],
            "meta": self.meta,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ToolCall":
        return cls(
            id=data["id"],
            title=data.get("title", ""),
            kind=ToolKind(data.get("kind", "execute")),
            content=data.get("content", []),
            status=ToolCallStatus(data.get("status", "pending")),
            locations=[
                ToolCallLocation.from_dict(l) for l in data.get("locations", [])
            ],
            meta=data.get("meta", {}),
        )


@dataclass
class ToolCallUpdateFields:
    """工具调用更新字段"""

    kind: Optional[ToolKind] = None
    status: Optional[ToolCallStatus] = None
    title: Optional[str] = None
    content: Optional[list[dict[str, Any]]] = None
    locations: Optional[list[ToolCallLocation]] = None
    raw_input: Optional[Any] = None
    raw_output: Optional[Any] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {}
        if self.kind:
            d["kind"] = self.kind.value
        if self.status:
            d["status"] = self.status.value
        if self.title:
            d["title"] = self.title
        if self.content is not None:
            d["content"] = self.content
        if self.locations:
            d["locations"] = [l.to_dict() for l in self.locations]
        if self.raw_input is not None:
            d["rawInput"] = self.raw_input
        if self.raw_output is not None:
            d["rawOutput"] = self.raw_output
        return d


@dataclass
class SessionUpdate:
    """会话更新"""

    type: SessionUpdateType
    data: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def assistant_message_chunk(cls, content: str) -> "SessionUpdate":
        return cls(
            type=SessionUpdateType.ASSISTANT_MESSAGE_CHUNK, data={"content": content}
        )

    @classmethod
    def assistant_message_complete(cls) -> "SessionUpdate":
        return cls(type=SessionUpdateType.ASSISTANT_MESSAGE_COMPLETE)

    @classmethod
    def tool_call(cls, tool_call: ToolCall) -> "SessionUpdate":
        return cls(type=SessionUpdateType.TOOL_CALL, data=tool_call.to_dict())

    @classmethod
    def tool_call_update(
        cls, fields: ToolCallUpdateFields, meta: Optional[dict[str, Any]] = None
    ) -> "SessionUpdate":
        d = fields.to_dict()
        if meta:
            d["meta"] = meta
        return cls(type=SessionUpdateType.TOOL_CALL_UPDATE, data=d)

    @classmethod
    def tool_call_complete(cls, tool_call_id: str) -> "SessionUpdate":
        return cls(type=SessionUpdateType.TOOL_CALL_COMPLETE, data={"id": tool_call_id})

    @classmethod
    def user_message_chunk(cls, content: str) -> "SessionUpdate":
        return cls(type=SessionUpdateType.USER_MESSAGE_CHUNK, data={"content": content})

    @classmethod
    def user_message_complete(cls) -> "SessionUpdate":
        return cls(type=SessionUpdateType.USER_MESSAGE_COMPLETE)

    @classmethod
    def session_info_update(cls, **kwargs) -> "SessionUpdate":
        return cls(type=SessionUpdateType.SESSION_INFO_UPDATE, data=kwargs)

    def to_dict(self) -> dict[str, Any]:
        return {"type": self.type.value, **self.data}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "SessionUpdate":
        update_type = SessionUpdateType(data.pop("type", ""))
        return cls(type=update_type, data=data)


@dataclass
class PermissionOption:
    """权限选项"""

    id: str
    label: str
    kind: PermissionOptionKind = PermissionOptionKind.ALLOW_ONCE
    description: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "id": self.id,
            "label": self.label,
            "kind": self.kind.value,
        }
        if self.description:
            d["description"] = self.description
        return d


@dataclass
class ACPError(Exception):
    """ACP 协议错误"""

    code: int
    message: str
    data: Any = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"code": self.code, "message": self.message}
        if self.data is not None:
            d["data"] = self.data
        return d

    @classmethod
    def internal_error(cls, msg: str = "Internal error") -> "ACPError":
        return cls(code=ErrorCode.INTERNAL_ERROR, message=msg)

    @classmethod
    def method_not_found(cls, method: str) -> "ACPError":
        return cls(
            code=ErrorCode.METHOD_NOT_FOUND, message=f"Method not found: {method}"
        )

    @classmethod
    def auth_required(cls) -> "ACPError":
        return cls(code=ErrorCode.AUTH_REQUIRED, message="Authentication required")

    @classmethod
    def invalid_params(cls, msg: str) -> "ACPError":
        return cls(code=ErrorCode.INVALID_PARAMS, message=msg)

    @classmethod
    def session_not_found(cls, session_id: str) -> "ACPError":
        return cls(
            code=ErrorCode.SESSION_NOT_FOUND, message=f"Session not found: {session_id}"
        )


# ─── JSON-RPC 消息 ────────────────────────────────────────────────────────


@dataclass
class JsonRpcRequest:
    """JSON-RPC 请求"""

    method: str
    params: Any = None
    id: Optional[RequestId] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"jsonrpc": "2.0", "method": self.method}
        if self.params is not None:
            d["params"] = self.params
        if self.id is not None:
            d["id"] = self.id
        return d


@dataclass
class JsonRpcResponse:
    """JSON-RPC 响应"""

    id: RequestId
    result: Any = None
    error: Optional[ACPError] = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"jsonrpc": "2.0", "id": self.id}
        if self.error:
            d["error"] = self.error.to_dict()
        else:
            d["result"] = self.result
        return d


@dataclass
class JsonRpcNotification:
    """JSON-RPC 通知（无 id）"""

    method: str
    params: Any = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"jsonrpc": "2.0", "method": self.method}
        if self.params is not None:
            d["params"] = self.params
        return d


def parse_json_rpc(
    line: str,
) -> JsonRpcRequest | JsonRpcResponse | JsonRpcNotification | None:
    """从一行 JSON 解析 JSON-RPC 消息"""
    try:
        obj = json.loads(line)
    except json.JSONDecodeError:
        logger.warning(f"无法解析 JSON: {line[:100]}")
        return None
    if not isinstance(obj, dict):
        return None
    msg_id = obj.get("id")
    method = obj.get("method")

    if msg_id is not None and method:
        return JsonRpcRequest(method=method, params=obj.get("params"), id=msg_id)
    elif msg_id is not None and not method:
        if "error" in obj:
            err_data = obj["error"]
            error = ACPError(
                code=err_data.get("code", ErrorCode.INTERNAL_ERROR),
                message=err_data.get("message", "Unknown error"),
                data=err_data.get("data"),
            )
            return JsonRpcResponse(id=msg_id, error=error)
        else:
            return JsonRpcResponse(id=msg_id, result=obj.get("result"))
    elif method:
        return JsonRpcNotification(method=method, params=obj.get("params"))
    logger.warning(f"无法识别的 JSON-RPC 消息: {line[:100]}")
    return None


# ─── ACP Agent 基类 ─────────────────────────────────────────────────────


class AcpAgent:
    """
    ACP Agent 服务器基类

    Agent 通过 stdin/stdout 接收和发送 JSON-RPC 消息。
    使用 inbox 队列异步处理消息，使得 prompt 处理期间
    也能接收响应消息（支持工具调用的双向通信）。

    子类需要实现 on_prompt() 来处理用户消息。
    """

    def __init__(
        self,
        *,
        name: str = "python-acp-agent",
        version: str = "0.1.0",
        reader: asyncio.StreamReader | None = None,
        writer: asyncio.StreamWriter | None = None,
    ):
        self.name = name
        self.version = version
        self._reader = reader or asyncio.StreamReader()
        self._writer = writer
        self._running = False
        self._pending_requests: dict[RequestId, asyncio.Future] = {}
        self._request_id_counter = 0

        # inbox: 后台任务从 stdin 读取消息放入此队列
        # 前台任务（prompt handler）可从中消费响应
        self._inbox: asyncio.Queue[str] = asyncio.Queue(maxsize=INBOX_MAX_SIZE)
        self._background_reader_task: asyncio.Task | None = None

        self.sessions: dict[str, "AcpSession"] = {}

        self.agent_capabilities: dict[str, Any] = {
            "promptCapabilities": {"modes": True, "modelSelection": True},
            "sessionCapabilities": {
                "close": {"supports": True},
                "resume": {"supports": False},
                "list": {"supports": False},
            },
            "loadSession": True,
        }
        self.default_modes: list[dict[str, Any]] = [
            {"id": "default", "name": "Default", "description": "标准模式"},
        ]
        self.default_models: list[dict[str, Any]] = [
            {
                "modelId": "default-model",
                "name": "Default Model",
                "description": "默认模型",
            },
        ]

        self._request_handlers: dict[str, Callable] = {}
        self._register_default_handlers()

    def _register_default_handlers(self):
        self._request_handlers["initialize"] = self._handle_initialize
        self._request_handlers["newSession"] = self._handle_new_session
        self._request_handlers["loadSession"] = self._handle_load_session
        self._request_handlers["closeSession"] = self._handle_close_session
        self._request_handlers["prompt"] = self._handle_prompt

    @property
    def _next_id(self) -> int:
        self._request_id_counter += 1
        return self._request_id_counter

    # ─── IO 层 ──────────────────────────────────────────────────────────

    async def _read_line_impl(self) -> str:
        line = await self._reader.readline()
        if not line:
            raise EOFError("stdin 已关闭")
        return line.decode("utf-8").rstrip("\n\r")

    async def _send_impl(self, data: str):
        line = data + "\n"
        raw = line.encode("utf-8")
        if self._writer:
            self._writer.write(raw)
            await self._writer.drain()
        else:
            sys.stdout.write(line)
            sys.stdout.flush()

    # 子类可重写这些进行测试
    async def _read_line(self) -> str:
        return await self._read_line_impl()

    async def _send(self, data: str):
        await self._send_impl(data)

    async def send_jsonrpc(
        self, msg: JsonRpcRequest | JsonRpcResponse | JsonRpcNotification
    ):
        text = json.dumps(msg.to_dict(), ensure_ascii=False)
        logger.debug(f"  >> {text}")
        await self._send(text)

    async def send_request(self, method: str, params: Any = None) -> Any:
        """
        发送请求到客户端（Zed）并等待响应。

        用于 agent → client 方向的请求。
        主循环会从 inbox 读取响应并 resolve future。
        """
        msg_id = self._next_id
        future: asyncio.Future = asyncio.get_event_loop().create_future()
        self._pending_requests[msg_id] = future
        await self.send_jsonrpc(JsonRpcRequest(method=method, params=params, id=msg_id))
        try:
            return await asyncio.wait_for(future, timeout=300.0)
        except asyncio.TimeoutError:
            self._pending_requests.pop(msg_id, None)
            raise
        except Exception:
            self._pending_requests.pop(msg_id, None)
            raise

    async def send_notification(self, method: str, params: Any = None):
        await self.send_jsonrpc(JsonRpcNotification(method=method, params=params))

    async def send_session_update(self, session_id: str, update: SessionUpdate):
        await self.send_notification(
            "session/update",
            {
                "sessionId": session_id,
                "update": update.to_dict(),
            },
        )

    # ─── 后台读取 ───────────────────────────────────────────────────────

    async def _background_reader(self):
        """后台任务：持续从 stdin 读取消息并放入 inbox"""
        try:
            while self._running:
                line = await self._read_line()
                await self._inbox.put(line)
        except (EOFError, asyncio.CancelledError):
            pass

    # ─── 任务分发 ───────────────────────────────────────────────────────
    #
    # 请求（Request）在独立 Task 中执行，使主循环可以继续
    # 处理后续 inbox 消息（如 cancel 通知）。
    #
    # 通知（Notification）和响应（Response）由主循环
    # 直接同步处理（它们不会长时间阻塞）。

    async def _run_handler_in_task(self, req: JsonRpcRequest):
        """在独立 Task 中运行请求处理程序"""
        try:
            handler = self._request_handlers.get(req.method)
            if handler is None:
                logger.warning(f"未知方法: {req.method}")
                await self.send_jsonrpc(
                    JsonRpcResponse(
                        id=req.id, error=ACPError.method_not_found(req.method)
                    )
                )
                return
            try:
                result = await handler(req.params)
                await self.send_jsonrpc(JsonRpcResponse(id=req.id, result=result))
            except ACPError as e:
                await self.send_jsonrpc(JsonRpcResponse(id=req.id, error=e))
            except Exception as e:
                logger.exception(f"处理请求 {req.method} 出错")
                await self.send_jsonrpc(
                    JsonRpcResponse(id=req.id, error=ACPError.internal_error(str(e)))
                )
        except (asyncio.CancelledError, RuntimeError):
            pass  # agent 正在关闭

    async def _process_one_message(self, line: str):
        msg = parse_json_rpc(line)
        if msg is None:
            return
        if isinstance(msg, JsonRpcRequest):
            # 在独立 Task 中运行请求处理，避免阻塞主循环
            asyncio.create_task(self._run_handler_in_task(msg))
        elif isinstance(msg, JsonRpcResponse):
            fut = self._pending_requests.get(msg.id)
            if fut:
                if msg.error:
                    fut.set_exception(
                        ACPError(msg.error.code, msg.error.message, msg.error.data)
                    )
                else:
                    fut.set_result(msg.result)
                del self._pending_requests[msg.id]
            else:
                logger.warning(f"收到未知请求 ID 的响应: {msg.id}")
        elif isinstance(msg, JsonRpcNotification):
            await self._handle_incoming_notification(msg)

    async def _handle_incoming_response(self, resp: JsonRpcResponse):
        """由 inbox 处理器调用"""
        future = self._pending_requests.get(resp.id)
        if future is None:
            logger.warning(f"收到未知请求 ID 的响应: {resp.id}")
            return
        if resp.error:
            future.set_exception(
                ACPError(resp.error.code, resp.error.message, resp.error.data)
            )
        else:
            future.set_result(resp.result)
        del self._pending_requests[resp.id]

    async def _handle_incoming_notification(self, notif: JsonRpcNotification):
        if notif.method == "cancel":
            session_id = notif.params.get("sessionId") if notif.params else None
            if session_id and session_id in self.sessions:
                self.sessions[session_id].cancel()
            logger.info(f"收到取消通知: session={session_id}")
        elif notif.method == "initialized":
            logger.info("客户端已初始化")
        else:
            logger.debug(f"收到未处理的通知: {notif.method}")

    # ─── 请求处理程序 ───────────────────────────────────────────────────

    async def _handle_initialize(self, params: dict[str, Any]) -> dict[str, Any]:
        protocol_version = params.get("protocolVersion", "v1")
        logger.info(f"初始化请求: 协议版本={protocol_version}")
        if protocol_version < MINIMUM_SUPPORTED_VERSION:
            raise ACPError(
                code=ErrorCode.INVALID_PARAMS,
                message=f"不支持的协议版本: {protocol_version}，最低要求: {MINIMUM_SUPPORTED_VERSION}",
            )
        return {
            "protocolVersion": PROTOCOL_VERSION,
            "agentCapabilities": self.agent_capabilities,
            "authMethods": [],
            "agentInfo": {"name": self.name, "version": self.version},
        }

    async def _handle_new_session(self, params: dict[str, Any]) -> dict[str, Any]:
        cwd = params.get("cwd", "/")
        session_id = str(uuid.uuid4())
        logger.info(f"新建会话: id={session_id}, cwd={cwd}")
        session = AcpSession(session_id=session_id, cwd=cwd, agent=self)
        self.sessions[session_id] = session
        return {
            "sessionId": session_id,
            "modes": {"currentModeId": "default", "availableModes": self.default_modes},
            "models": {
                "currentModelId": "default-model",
                "availableModels": self.default_models,
            },
            "configOptions": [],
        }

    async def _handle_load_session(self, params: dict[str, Any]) -> dict[str, Any]:
        session_id = params.get("sessionId")
        cwd = params.get("cwd", "/")
        if session_id not in self.sessions:
            logger.info(f"加载会话（新建）: id={session_id}, cwd={cwd}")
            session = AcpSession(session_id=session_id, cwd=cwd, agent=self)
            self.sessions[session_id] = session
        else:
            logger.info(f"加载会话（已有）: id={session_id}")
        return {
            "modes": {"currentModeId": "default", "availableModes": self.default_modes},
            "models": {
                "currentModelId": "default-model",
                "availableModels": self.default_models,
            },
            "configOptions": [],
        }

    async def _handle_close_session(self, params: dict[str, Any]) -> dict[str, Any]:
        session_id = params.get("sessionId")
        if session_id in self.sessions:
            del self.sessions[session_id]
            logger.info(f"关闭会话: {session_id}")
        return {}

    async def _handle_prompt(self, params: dict[str, Any]) -> dict[str, Any]:
        session_id = params.get("sessionId")
        messages = params.get("messages", [])
        session = self.sessions.get(session_id)
        if session is None:
            raise ACPError.session_not_found(session_id)
        try:
            await self.on_prompt(session, messages)
        except asyncio.CancelledError:
            return {"stopReason": StopReason.CANCELLED.value}
        return {"stopReason": StopReason.END_TURN.value}

    # ─── 子类可重写的方法 ──────────────────────────────────────────────

    async def on_prompt(self, session: "AcpSession", messages: list[Any]):
        """处理提示词。子类应重写此方法。"""
        await session.send_assistant_chunk(f"你好！我收到了 {len(messages)} 条消息。")
        await session.send_assistant_chunk("（这是一个 ACP Python 演示 Agent）")

    # ─── 运行 ───────────────────────────────────────────────────────────

    async def run(self):
        self._running = True
        logger.info(f"ACP Agent '{self.name}' v{self.version} 已启动")

        # 启动后台 reader 任务
        self._background_reader_task = asyncio.create_task(self._background_reader())

        try:
            while self._running:
                line = await self._inbox.get()
                await self._process_one_message(line)
        except (EOFError, asyncio.CancelledError):
            logger.info("Agent 退出")
        finally:
            self._running = False
            if self._background_reader_task and not self._background_reader_task.done():
                self._background_reader_task.cancel()
            for future in self._pending_requests.values():
                if not future.done():
                    future.cancel()
            self._pending_requests.clear()

    def stop(self):
        self._running = False


# ─── 会话 ──────────────────────────────────────────────────────────────


class AcpSession:
    """ACP 会话"""

    def __init__(self, session_id: str, cwd: str, agent: AcpAgent):
        self.session_id = session_id
        self.cwd = cwd
        self.agent = agent
        self._cancel_event = asyncio.Event()
        self._cancelled = False
        self.messages: list[dict[str, Any]] = []

    def cancel(self):
        self._cancelled = True
        self._cancel_event.set()

    @property
    def is_cancelled(self) -> bool:
        return self._cancel_event.is_set()

    async def wait_for_cancel(self, timeout: float = 0.1):
        """等待取消信号或超时，用于中断较长的 sleep"""
        try:
            await asyncio.wait_for(
                asyncio.shield(self._cancel_event.wait()),
                timeout=timeout,
            )
            return True
        except asyncio.TimeoutError:
            return False

    async def send_assistant_chunk(self, text: str):
        if self._cancelled:
            return
        await self.agent.send_session_update(
            self.session_id, SessionUpdate.assistant_message_chunk(text)
        )

    async def send_assistant_complete(self):
        await self.agent.send_session_update(
            self.session_id, SessionUpdate.assistant_message_complete()
        )

    async def send_tool_call(self, tool_call: ToolCall):
        await self.agent.send_session_update(
            self.session_id, SessionUpdate.tool_call(tool_call)
        )

    async def send_tool_call_update(self, tool_call_id: str, **fields):
        await self.agent.send_session_update(
            self.session_id,
            SessionUpdate.tool_call_update(ToolCallUpdateFields(**fields)),
        )

    async def send_tool_call_complete(self, tool_call_id: str):
        await self.agent.send_session_update(
            self.session_id, SessionUpdate.tool_call_complete(tool_call_id)
        )

    async def request_permission(
        self, tool_call: ToolCall, options: list[PermissionOption]
    ) -> dict[str, Any]:
        return await self.agent.send_request(
            "requestPermission",
            {
                "sessionId": self.session_id,
                "toolCall": tool_call.to_dict(),
                "options": [o.to_dict() for o in options],
            },
        )

    async def write_text_file(self, path: str, content: str) -> None:
        await self.agent.send_request(
            "writeTextFile",
            {
                "sessionId": self.session_id,
                "path": path,
                "content": content,
            },
        )

    async def read_text_file(self, path: str, line: int = 1, limit: int = 100) -> str:
        result = await self.agent.send_request(
            "readTextFile",
            {
                "sessionId": self.session_id,
                "path": path,
                "line": line,
                "limit": limit,
            },
        )
        return result.get("content", "")

    async def create_terminal(
        self,
        command: str,
        args: list[str] | None = None,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"sessionId": self.session_id, "command": command}
        if args:
            params["args"] = args
        if cwd:
            params["cwd"] = cwd
        if env:
            params["env"] = [{"name": k, "value": v} for k, v in env.items()]
        return await self.agent.send_request("createTerminal", params)


# ─── 命令行运行 ─────────────────────────────────────────────────────────


def run_agent(agent_class: type[AcpAgent] = AcpAgent, **kwargs):
    """
    在命令行中运行 ACP Agent。

    通过 stdin/stdout 与父进程（如 Zed）通信。
    使用 asyncio 的事件循环连接 stdin/stdout 管道。
    """
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        stream=sys.stderr,
    )

    agent = agent_class(**kwargs)

    async def _run():
        loop = asyncio.get_event_loop()
        reader = asyncio.StreamReader()
        protocol = asyncio.StreamReaderProtocol(reader)
        await loop.connect_read_pipe(lambda: protocol, sys.stdin)
        writer_transport, writer_protocol = await loop.connect_write_pipe(
            asyncio.Protocol, sys.stdout
        )
        writer = asyncio.StreamWriter(writer_transport, writer_protocol, None, loop)
        agent._reader = reader
        agent._writer = writer
        await agent.run()

    try:
        asyncio.run(_run())
    except KeyboardInterrupt:
        logger.info("用户中断")
