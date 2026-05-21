#!/usr/bin/env python3
"""
ACP 测试客户端

模拟 Zed，向 ACP Agent 发送消息并查看响应。
可以测试任何 ACP Agent，不仅限于 Python 版本。

用法：
  # 测试 Python Agent
  python acp_test_client.py --agent "python internal_agent.py"

  # 测试真实 Claude Code
  python acp_test_client.py --agent "npx @anthropic/claude-code --acp"

  # 交互模式（手动输入消息）
  python acp_test_client.py --agent "python internal_agent.py" --interactive
"""

import json
import queue
import shlex
import subprocess
import sys
import threading


class AcpTestClient:
    """ACP 协议测试客户端"""

    def __init__(self, agent_cmd: str):
        self.agent_cmd = agent_cmd
        self.proc: subprocess.Popen | None = None
        self._id = 0
        self._pending: dict[int, dict] = {}
        self._recv_queue: queue.Queue = queue.Queue()
        self._reader_thread: threading.Thread | None = None

    def start(self):
        """启动 Agent 子进程"""
        print(f"\n启动 Agent: {self.agent_cmd}")
        self.proc = subprocess.Popen(
            shlex.split(self.agent_cmd),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        print(f"   PID: {self.proc.pid}\n")

        # 后台读取 stdout 线程
        def reader():
            for line in self.proc.stdout:
                line = line.strip()
                if line:
                    self._recv_queue.put(line)

        self._reader_thread = threading.Thread(target=reader, daemon=True)
        self._reader_thread.start()

        # 后台读取 stderr 线程
        def stderr_reader():
            for line in self.proc.stderr:
                print(f"  [stderr] {line.rstrip()}")

        st = threading.Thread(target=stderr_reader, daemon=True)
        st.start()

    def stop(self):
        if self.proc:
            self.proc.terminate()
            self.proc.wait(timeout=5)
            print(f"\nAgent 已停止 (exit={self.proc.returncode})")

    def send(self, obj: dict):
        line = json.dumps(obj, ensure_ascii=False) + "\n"
        self.proc.stdin.write(line)
        self.proc.stdin.flush()

    def recv(self, timeout: float = 5.0) -> dict:
        """从 Agent 读取一行 JSON（线程安全）"""
        line = self._recv_queue.get(timeout=timeout)
        return json.loads(line)

    def request(self, method: str, params: dict = None) -> dict:
        """发送请求并等待响应"""
        self._id += 1
        self.send(
            {
                "jsonrpc": "2.0",
                "id": self._id,
                "method": method,
                "params": params or {},
            }
        )
        while True:
            resp = self.recv()
            if resp.get("id") == self._id:
                return resp
            self._pending[resp.get("id")] = resp

    def notify(self, method: str, params: dict = None):
        self.send({"jsonrpc": "2.0", "method": method, "params": params or {}})


def interactive_mode(client: AcpTestClient):
    """交互模式：手动输入消息"""
    print("=" * 50)
    print("  ACP 交互测试")
    print("  输入消息直接发送给 Agent")
    print("  输入 q 退出")
    print("=" * 50)

    # 握手
    r = client.request(
        "initialize",
        {
            "protocolVersion": "v1",
            "clientCapabilities": {
                "fs": {"readTextFile": True, "writeTextFile": True},
                "terminal": True,
                "auth": {"terminal": True},
            },
            "clientInfo": {"name": "acp-test-client", "version": "1.0"},
        },
    )
    print(f"\n✅ 初始化成功: {r.get('result', {}).get('agentInfo', {})}")
    client.notify("initialized")

    # 创建会话
    r = client.request("newSession", {"cwd": "."})
    sid = r.get("result", {}).get("sessionId")
    print(f"✅ 会话创建: {sid}\n")

    # 交互循环
    print("--- 输入消息，直接回车发送 ---")
    while True:
        try:
            text = input("> ").strip()
        except (EOFError, KeyboardInterrupt):
            break
        if not text or text == "q":
            break

        r = client.request(
            "prompt", {"sessionId": sid, "messages": [{"type": "text", "text": text}]}
        )
        print(f"   停止原因: {r.get('result', {}).get('stopReason')}")

    # 清理
    client.request("closeSession", {"sessionId": sid})


def batch_test(client: AcpTestClient):
    """批量测试：自动运行预设的测试用例"""
    passed = 0
    failed = 0

    def test(name: str, ok: bool, detail: str = ""):
        nonlocal passed, failed
        if ok:
            passed += 1
            print(f"  ✅ {name}")
        else:
            failed += 1
            print(f"  ❌ {name} — {detail}")

    # 握手
    print("\n[测试 1] 初始化")
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
    test("协议版本", r.get("result", {}).get("protocolVersion") == "v1")
    test("有 agentInfo", bool(r.get("result", {}).get("agentInfo")))
    client.notify("initialized")

    # 会话
    print("\n[测试 2] 创建会话")
    r = client.request("newSession", {"cwd": "/tmp"})
    sid = r.get("result", {}).get("sessionId")
    test("有 sessionId", bool(sid))

    # 提示词
    print("\n[测试 3] 发送消息")
    r = client.request(
        "prompt", {"sessionId": sid, "messages": [{"type": "text", "text": "hello"}]}
    )
    test("有响应", bool(r.get("result")))
    test("正常结束", r.get("result", {}).get("stopReason") in ("endTurn", "endTurn"))

    # 关闭
    print("\n[测试 4] 关闭")
    r = client.request("closeSession", {"sessionId": sid})
    test("关闭成功", r.get("result") == {})

    # 结果
    total = passed + failed
    print(f"\n结果: {passed}/{total} 通过")
    return 0 if failed == 0 else 1


def main():
    import argparse

    parser = argparse.ArgumentParser(description="ACP 协议测试客户端")
    parser.add_argument(
        "--agent",
        default="python internal_agent.py",
        help="Agent 启动命令 (默认: python internal_agent.py)",
    )
    parser.add_argument("--interactive", action="store_true", help="交互模式")
    args = parser.parse_args()

    client = AcpTestClient(args.agent)
    try:
        client.start()
        if args.interactive:
            interactive_mode(client)
        else:
            sys.exit(batch_test(client))
    except Exception as e:
        print(f"  ❌ 错误: {e}")
        sys.exit(1)
    finally:
        client.stop()


if __name__ == "__main__":
    main()
