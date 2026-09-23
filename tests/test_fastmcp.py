import pytest

mcp = pytest.importorskip("mcp")
types = pytest.importorskip("mcp.types")
fastmcp_mod = pytest.importorskip("fastmcp")
FastMCP = fastmcp_mod.FastMCP

from b_fast._b_fast import BFastError
from b_fast.fastmcp import (
    BFAST_MIME_TYPE,
    FastMCPBFast,
    bfast_resource,
    bfast_tool,
    decode_mcp_resource,
    encode_mcp_resource,
)


def test_encode_and_decode_mcp_resource():
    """Test encoding and decoding directly with MCP content items."""
    data = {
        "id": 123,
        "name": "Dataset A",
        "scores": [1.5, 2.5, 3.5],
        "active": True,
    }

    # Encode with summary
    items = encode_mcp_resource(data, uri="bfast://test/1", compress=True)
    assert len(items) == 2
    assert isinstance(items[0], types.TextContent)
    assert "[B-FAST binary payload" in items[0].text
    assert isinstance(items[1], types.EmbeddedResource)
    assert items[1].resource.mime_type == BFAST_MIME_TYPE
    assert items[1].resource.uri == "bfast://test/1"

    # Decode directly from the list of content items
    decoded = decode_mcp_resource(items)
    assert decoded["id"] == 123
    assert decoded["name"] == "Dataset A"
    assert decoded["scores"] == [1.5, 2.5, 3.5]
    assert decoded["active"] is True

    # Decode directly from the EmbeddedResource
    decoded_single = decode_mcp_resource(items[1])
    assert decoded_single == decoded

    # Decode directly from BlobResourceContents
    decoded_blob = decode_mcp_resource(items[1].resource)
    assert decoded_blob == decoded

    # Decode directly from base64 string
    decoded_b64 = decode_mcp_resource(items[1].resource.blob)
    assert decoded_b64 == decoded


def test_encode_mcp_resource_without_summary():
    """Test encoding without text summary."""
    data = [1, 2, 3]
    items = encode_mcp_resource(data, compress=False, include_summary=False)
    assert len(items) == 1
    assert isinstance(items[0], types.EmbeddedResource)
    assert decode_mcp_resource(items) == data


@pytest.mark.anyio
async def test_fastmcp_sync_tool_decorator():
    """Test registering a sync tool on FastMCP via @mcp.tool() and @bfast_tool."""
    mcp = FastMCP("SyncServer")

    @mcp.tool()
    @bfast_tool(compress=True, uri="bfast://tools/sync_analytics")
    def sync_analytics(metric: str, count: int) -> dict:
        """Calculate test analytics metrics."""
        return {
            "metric": metric,
            "values": [i * 10 for i in range(count)],
            "mean": (count - 1) * 5.0,
        }

    # Call the tool through FastMCP
    res = await mcp.call_tool("sync_analytics", {"metric": "latency", "count": 4})
    assert len(res.content) == 2
    assert any(c.type == "text" for c in res.content)
    assert any(c.type == "resource" for c in res.content)

    # Decode the full tool result
    data = decode_mcp_resource(res)
    assert data["metric"] == "latency"
    assert data["values"] == [0, 10, 20, 30]
    assert data["mean"] == 15.0


@pytest.mark.anyio
async def test_fastmcp_async_tool_decorator():
    """Test registering an async tool on FastMCP."""
    mcp = FastMCP("AsyncServer")

    @mcp.tool()
    @bfast_tool(compress=True)
    async def async_fetch_users(role: str) -> list:
        return [
            {"id": 1, "role": role, "name": "Alice"},
            {"id": 2, "role": role, "name": "Bob"},
        ]

    res = await mcp.call_tool("async_fetch_users", {"role": "admin"})
    data = decode_mcp_resource(res)
    assert len(data) == 2
    assert data[0]["name"] == "Alice"
    assert data[1]["name"] == "Bob"


@pytest.mark.anyio
async def test_bfast_tool_direct_server_registration():
    """Test registering a tool directly using @bfast_tool(mcp, ...)."""
    mcp = FastMCP("DirectServer")

    @bfast_tool(mcp, compress=True, description="Fetch sensor batches")
    def get_sensors() -> dict:
        return {"sensors": ["A1", "B2"], "status": "online"}

    res = await mcp.call_tool("get_sensors", {})
    data = decode_mcp_resource(res)
    assert data == {"sensors": ["A1", "B2"], "status": "online"}


@pytest.mark.anyio
async def test_fastmcp_resource():
    """Test registering and reading an MCP resource with @bfast_resource."""
    mcp = FastMCP("ResourceServer")

    @bfast_resource(mcp, "bfast://system/config", compress=True)
    def system_config() -> dict:
        return {"version": "2.0.0", "rate_limit": 1000}

    read_res = await mcp.read_resource("bfast://system/config")
    assert len(read_res.contents) == 1
    content_item = read_res.contents[0]
    assert content_item.mime_type == BFAST_MIME_TYPE
    assert isinstance(content_item.content, bytes)

    decoded = decode_mcp_resource(read_res)
    assert decoded == {"version": "2.0.0", "rate_limit": 1000}


@pytest.mark.anyio
async def test_fastmcp_async_resource():
    """Test registering and reading an async MCP resource with @bfast_resource."""
    mcp = FastMCP("AsyncResourceServer")

    @bfast_resource(mcp, "bfast://metrics/latest", compress=False)
    async def latest_metrics() -> list:
        return [10.5, 20.5, 30.5]

    read_res = await mcp.read_resource("bfast://metrics/latest")
    decoded = decode_mcp_resource(read_res)
    assert decoded == [10.5, 20.5, 30.5]


@pytest.mark.anyio
async def test_fastmcp_bfast_adapter():
    """Test the FastMCPBFast all-in-one class adapter."""
    mcp = FastMCPBFast("AdapterServer")

    @mcp.bfast_tool(compress=True)
    def calculate_matrix(rows: int, cols: int) -> dict:
        return {
            "rows": rows,
            "cols": cols,
            "matrix": [[1.0] * cols for _ in range(rows)],
        }

    @mcp.bfast_resource("bfast://info/summary")
    def get_info() -> dict:
        return {"server": "AdapterServer", "status": "ready"}

    tool_res = await mcp.call_tool("calculate_matrix", {"rows": 2, "cols": 3})
    tool_data = decode_mcp_resource(tool_res)
    assert tool_data["rows"] == 2
    assert tool_data["cols"] == 3
    assert tool_data["matrix"] == [[1.0, 1.0, 1.0], [1.0, 1.0, 1.0]]

    resource_res = await mcp.read_resource("bfast://info/summary")
    resource_data = decode_mcp_resource(resource_res)
    assert resource_data == {"server": "AdapterServer", "status": "ready"}


@pytest.mark.anyio
async def test_mcp2_mcpserver_compatibility():
    """Test compatibility with mcp.server.mcpserver.MCPServer from mcp 2.x."""
    try:
        from mcp.server.mcpserver import MCPServer
    except ImportError:
        pytest.skip("mcp.server.mcpserver not available")

    server = MCPServer("Mcp2Server")

    @bfast_tool(server, compress=True)
    def query_stats(server_id: str) -> dict:
        return {"server_id": server_id, "cpu": 12.5, "mem": 45.0}

    res = await server.call_tool("query_stats", {"server_id": "srv-01"})
    decoded = decode_mcp_resource(res)
    assert decoded["server_id"] == "srv-01"
    assert decoded["cpu"] == 12.5
    assert decoded["mem"] == 45.0


def test_decode_mcp_resource_errors():
    """Test error handling in decode_mcp_resource."""
    # Empty content list
    with pytest.raises(BFastError, match="No B-FAST resource found"):
        decode_mcp_resource([])

    # Content list with no B-FAST resource
    other_content = [types.TextContent(type="text", text="plain text")]
    with pytest.raises(BFastError, match="No B-FAST resource found"):
        decode_mcp_resource(other_content)

    # Unsupported type
    with pytest.raises(BFastError, match="Unsupported MCP resource type"):
        decode_mcp_resource(12345)
