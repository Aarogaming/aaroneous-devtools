import asyncio
import json
import os
import aiohttp
from typing import Optional, Dict, Any, List

class AASBridge:
    """
    Aaroneous Autonomous SDK (AAS) Bridge.
    Connects Python "Cognitive Shards" to the Rust Aaroneous core via MCP (HTTP+SSE).
    """
    def __init__(self, mcp_url: str = None):
        self.mcp_url = mcp_url or os.getenv("AARONEOUS_MCP_URL", "http://localhost:8766/mcp")
        self.session_id = f"aas-{os.getpid()}"
        self._http_session: Optional[aiohttp.ClientSession] = None

    async def __aenter__(self):
        self._http_session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self._http_session:
            await self._http_session.close()

    async def _post(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Send a JSON-RPC 2.0 request to the Aaroneous core."""
        if not self._http_session:
            raise RuntimeError("Bridge not connected. Use 'async with AASBridge() as bridge:'")
        
        payload = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": {
                **params,
                "_meta": {"session_id": self.session_id}
            }
        }
        
        async with self._http_session.post(self.mcp_url, json=payload) as resp:
            return await resp.json()

    async def list_tools(self) -> List[Dict[str, Any]]:
        """Query available tools from the Aaroneous Hive."""
        result = await self._post("tools/list", {})
        return result.get("result", {}).get("tools", [])

    async def call_tool(self, name: str, arguments: Dict[str, Any]) -> str:
        """Invoke a specific Hive tool (Sovereign or Core tool)."""
        payload = {"name": name, "arguments": arguments}
        result = await self._post("tools/call", payload)
        
        if "error" in result:
            raise Exception(f"MCP Error: {result['error']}")
        
        # Standard MCP tool response structure: { "content": [{ "type": "text", "text": "..." }] }
        content = result.get("result", {}).get("content", [])
        return "\n".join([c["text"] for c in content if c["type"] == "text"])

    async def register_as_specialist(self, name: str, capabilities: List[str]):
        """
        Internalizes this Python Shard as a Dynamic Specialist within the Rust Core.
        This allows the Rust Sentinel loop to delegate tasks to this Python Shard.
        """
        self.shard_name = name
        result = await self.call_tool("register_shard", {
            "name": name,
            "capabilities": capabilities,
            "endpoint": f"http://localhost:5000/execute" # Shard's own callback
        })
        # Start heartbeat task
        asyncio.create_task(self._heartbeat_loop())
        return result

    async def _heartbeat_loop(self):
        """Metabolic heartbeat to sync with Rust biology governor."""
        while True:
            try:
                # Include local metrics to allow the core to make better scaling decisions
                import psutil
                process = psutil.Process(os.getpid())
                mem_info = process.memory_info()
                
                await self.call_tool("metabolic_heartbeat", {
                    "name": self.shard_name,
                    "vram_mb": 0, # Placeholder for GPU shards
                    "cpu_pct": process.cpu_percent(),
                    "mem_mb": mem_info.rss / 1024 / 1024,
                    "token_request": 1.0
                })
            except ImportError:
                # Fallback if psutil not installed
                await self.call_tool("metabolic_heartbeat", {
                    "name": self.shard_name,
                    "token_request": 1.0
                })
            except Exception as e:
                print(f"AAS Heartbeat failed: {e}")
            await asyncio.sleep(5)

    async def listen_for_tasks(self, callback):
        """
        Persistent SSE listener to receive task assignments from the Rust Core.
        When Rust identifies a task for this shard, it pushes via SSE.
        """
        if not self._http_session:
            raise RuntimeError("Bridge not connected.")

        sse_url = f"{self.mcp_url.replace('/mcp', '/sse')}?session={self.session_id}"
        print(f"AAS: Listening for tasks via SSE at {sse_url}...")

        while True:
            try:
                async with self._http_session.get(sse_url) as resp:
                    async for line in resp.content:
                        line = line.decode('utf-8').strip()
                        if line.startswith("data:"):
                            data = json.loads(line[5:])
                            # Filter for tool calls targeted at this shard
                            if data.get("method") == "tools/call":
                                result = await callback(data["params"])
                                # Send result back via POST
                                await self._post("tools/call_result", {
                                    "call_id": data.get("id"),
                                    "result": result
                                })
            except Exception as e:
                print(f"AAS SSE Connection lost: {e}. Retrying in 5s...")
                await asyncio.sleep(5)

    async def submit_intent(self, intent: str, priority: str = "Normal") -> str:
        """High-level helper to submit an intent to the full hive."""
        return await self.call_tool("submit_intent", {"content": intent, "priority": priority})

    async def emit_signal(self, signal_type: str, payload: Dict[str, Any]) -> str:
        """Broadcast a signal to all active WASM agents via the Rust E-Bus bridge."""
        return await self.call_tool("signal_wasms", {"signal_type": signal_type, "payload": payload})

    async def sync_memory(self, action: str, entries: List[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Synchronize memory with the Rust Core (Library sync)."""
        args = {"shard_name": self.shard_name, "action": action}
        if entries:
            args["entries"] = entries
        result_text = await self.call_tool("memory_sync", args)
        try:
            return json.loads(result_text)
        except json.JSONDecodeError:
            return {"raw_response": result_text}

async def main():
    """Example AAS Shard handshake."""
    async with AASBridge() as bridge:
        print("--- AAS Handshake ---")
        tools = await bridge.list_tools()
        print(f"Connected to Hive. Available tools: {len(tools)}")
        
        # Test basic retrieval
        status = await bridge.call_tool("get_specialists", {})
        print(f"Hive Status: {status[:200]}...")

if __name__ == "__main__":
    asyncio.run(main())
