# Model Context Protocol (MCP) & FastMCP Integration

⚡ **B-FAST** provides first-class native integration for **FastMCP** and **MCPServer** (Anthropic Model Context Protocol SDK).

Standard MCP transmits tool results as JSON-RPC over STDIO, SSE, or HTTP. When tools return large datasets (dataframes, database queries, sensor logs, embeddings, or NumPy arrays), JSON-RPC causes severe latency, CPU overhead, and token inflation.

With B-FAST, tool outputs are serialized **up to 15x faster**, **up to 80% smaller** with LZ4, and wrapped cleanly into native MCP `EmbeddedResource` (`BlobResourceContents`).

---

## Installation

Install B-FAST with the optional FastMCP extra:

```bash
pip install "bfast-py[fastmcp]"
```

Or standalone:

```bash
pip install bfast-py fastmcp
```

---

## FastMCP Integration

### 1. High-Performance Tool Outputs (`@bfast_tool`)

Use `@bfast_tool` to automatically serialize tool outputs into a B-FAST binary resource. It works with both synchronous and asynchronous tools:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import bfast_tool

mcp = FastMCP("AnalyticsServer")


# Method 1: Combine with @mcp.tool()
@mcp.tool()
@bfast_tool(compress=True)
def query_sensor_metrics(sensor_id: str, count: int = 1000) -> dict:
    """Fetch high-frequency sensor telemetry."""
    return {
        "sensor": sensor_id,
        "timestamps": [1700000000 + i for i in range(count)],
        "readings": [20.5 + (i % 5) for i in range(count)],
    }


# Method 2: Pass server directly to @bfast_tool
@bfast_tool(mcp, compress=True, description="Query user database")
async def get_users(role: str) -> list:
    return [
        {"id": 1, "name": "Alice", "role": role},
        {"id": 2, "name": "Bob", "role": role},
    ]
```

#### What the MCP Agent Receives:
The tool returns:
1. `TextContent`: An informational summary for the LLM (e.g. `[B-FAST binary payload (1000 items): 4200 bytes (compressed with LZ4) available in embedded resource 'bfast://...']`).
2. `EmbeddedResource`: A `BlobResourceContents` object with `mime_type="application/x-bfast"` containing the base64-encoded compressed binary bytes.

---

### 2. MCP Binary Resources (`@bfast_resource`)

Expose structured dataset snapshots as MCP resources with native `application/x-bfast` MIME type:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import bfast_resource

mcp = FastMCP("DataServer")


@bfast_resource(mcp, "bfast://models/weights", compress=True)
def get_weights() -> list:
    return [0.125, 0.456, 0.789, 1.024]
```

---

### 3. All-in-One Adapter (`FastMCPBFast`)

Wrap any existing `FastMCP` or `MCPServer` instance or create one with first-class B-FAST methods:

```python
from b_fast.fastmcp import FastMCPBFast

mcp = FastMCPBFast("EnterpriseService")


@mcp.bfast_tool(compress=True)
def process_data(batch_id: int):
    return {"status": "success", "batch": batch_id}


@mcp.bfast_resource("bfast://cluster/status")
def cluster_status():
    return {"nodes": 12, "healthy": True}
```

---

## Decoding in TypeScript Client (`bfast-client`)

When your AI agent, frontend, or web UI receives the tool result from the MCP server, decode it in one single line:

```typescript
import { decodeMcpResource } from 'bfast-client';

// Pass the CallToolResult directly from your MCP client
const result = await mcpClient.callTool({ name: 'query_sensor_metrics', arguments: { sensor_id: 'S-1' } });

// Decodes the embedded B-FAST resource automatically (with WebAssembly LZ4 acceleration)
const data = decodeMcpResource(result);
console.log(data.sensor);    // 'S-1'
console.log(data.readings);  // [20.5, 21.5, ...]
```

---

## Decoding in Python (`decode_mcp_resource`)

Python clients can decode MCP results just as easily:

```python
from b_fast.fastmcp import decode_mcp_resource

# Accepts CallToolResult, EmbeddedResource, BlobResourceContents, or base64 string
data = decode_mcp_resource(tool_result)
print(data["sensor"])
```

---

## Streaming MCP Tool Results

For tools processing continuous data streams, yield B-FAST chunks over Streamable HTTP or SSE:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import stream_mcp_async_tool_results

mcp = FastMCP("StreamServer")


@mcp.tool()
async def stream_large_dataset():
    async def data_generator():
        for batch_id in range(50):
            yield {"batch": batch_id, "data": [1, 2, 3]}

    return stream_mcp_async_tool_results(data_generator(), compress=True)
```
