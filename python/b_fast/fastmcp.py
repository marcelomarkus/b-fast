"""B-FAST FastMCP (Model Context Protocol) Integration.

Provides ultra-fast binary serialization, compressed blob resources, and streaming
for FastMCP and MCPServer tools and resources.
"""

from __future__ import annotations

import base64
import functools
import inspect
from typing import Any, Callable, Union

from ._b_fast import BFast, BFastError
from .mcp import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    stream_mcp_tool_results,
    wrap_mcp_tool_output,
)

BFAST_MIME_TYPE = "application/x-bfast"
BFAST_STREAM_MIME_TYPE = "application/x-bfast-stream"

try:
    import mcp.types as mcp_types
    from mcp.types import BlobResourceContents, EmbeddedResource, TextContent

    _HAS_MCP = True
except ImportError:
    _HAS_MCP = False
    mcp_types = None

    class BlobResourceContents:  # type: ignore[no-redef]
        def __init__(
            self,
            uri: str,
            mime_type: str,
            blob: str,
            meta: dict[str, Any] | None = None,
            **kwargs: Any,
        ):
            self.uri = uri
            self.mime_type = mime_type
            self.blob = blob
            self.meta = meta

    class EmbeddedResource:  # type: ignore[no-redef]
        def __init__(
            self,
            type: str,
            resource: Any,
            annotations: dict[str, Any] | None = None,
            meta: dict[str, Any] | None = None,
            **kwargs: Any,
        ):
            self.type = type
            self.resource = resource
            self.annotations = annotations
            self.meta = meta

    class TextContent:  # type: ignore[no-redef]
        def __init__(
            self,
            type: str,
            text: str,
            annotations: dict[str, Any] | None = None,
            meta: dict[str, Any] | None = None,
            **kwargs: Any,
        ):
            self.type = type
            self.text = text
            self.annotations = annotations
            self.meta = meta


def encode_mcp_resource(
    data: Any,
    uri: str = "bfast://data/output",
    compress: bool = True,
    include_summary: bool = True,
    summary_text: str | None = None,
) -> list[TextContent | EmbeddedResource]:
    """Encode Python data into a B-FAST binary MCP EmbeddedResource.

    Args:
        data: Any Python object, list, dict, Pydantic model, or NumPy array.
        uri: URI identifier for the MCP resource (defaults to "bfast://data/output").
        compress: Whether to apply LZ4 compression (default True).
        include_summary: Whether to include a TextContent item summarizing the payload.
        summary_text: Optional custom summary text. If not provided, an informative summary
                      with byte size and record count is generated.

    Returns:
        A list containing [TextContent, EmbeddedResource] (or [EmbeddedResource] if include_summary is False).
    """
    encoder = BFast()
    raw_bytes = encoder.encode_packed(data, compress=compress)
    if isinstance(raw_bytes, list):
        raw_bytes = bytes(raw_bytes)

    b64_str = base64.b64encode(raw_bytes).decode("ascii")
    blob = BlobResourceContents(
        uri=uri,
        mime_type=BFAST_MIME_TYPE,
        blob=b64_str,
    )
    resource_item = EmbeddedResource(
        type="resource",
        resource=blob,
    )

    if not include_summary:
        return [resource_item]

    if summary_text is None:
        count_desc = ""
        if hasattr(data, "__len__"):
            try:
                count_desc = f" ({len(data)} items)"
            except Exception:
                pass
        compression_desc = "compressed with LZ4" if compress else "uncompressed"
        summary_text = (
            f"[B-FAST binary payload{count_desc}: {len(raw_bytes)} bytes ({compression_desc}) "
            f"available in embedded resource '{uri}']"
        )

    text_item = TextContent(
        type="text",
        text=summary_text,
    )
    return [text_item, resource_item]


def decode_mcp_resource(resource_or_result: Any) -> Any:
    """Decode a B-FAST payload from an MCP tool result, EmbeddedResource, or BlobResourceContents.

    Args:
        resource_or_result: Any of:
            - CallToolResult / ToolResult containing .content
            - list of MCP Content items
            - EmbeddedResource
            - BlobResourceContents
            - ResourceContent (from read_resource)
            - base64 encoded string
            - raw bytes

    Returns:
        The decoded Python object (dict, list, NumPy array, etc.).
    """
    encoder = BFast()

    # 1. Direct bytes
    if isinstance(resource_or_result, (bytes, bytearray)):
        return encoder.decode_packed(bytes(resource_or_result))

    # 2. String (base64)
    if isinstance(resource_or_result, str):
        raw = base64.b64decode(resource_or_result)
        return encoder.decode_packed(raw)

    # 3. ToolResult / CallToolResult with .content attribute
    content_list = getattr(resource_or_result, "content", None)
    if content_list is None and hasattr(resource_or_result, "contents"):
        content_list = resource_or_result.contents

    if content_list is not None:
        return decode_mcp_resource(content_list)

    # 4. List of content items
    if isinstance(resource_or_result, (list, tuple)):
        for item in resource_or_result:
            if hasattr(item, "resource"):
                res = item.resource
                mime = getattr(res, "mime_type", None) or getattr(res, "mimeType", "")
                uri = getattr(res, "uri", "")
                if (
                    BFAST_MIME_TYPE in mime
                    or "bfast" in uri
                    or uri.startswith("bfast://")
                ):
                    blob_data = getattr(res, "blob", None)
                    if blob_data:
                        raw = base64.b64decode(blob_data)
                        return encoder.decode_packed(raw)
            elif hasattr(item, "content"):
                raw = item.content
                if isinstance(raw, (bytes, bytearray)):
                    return encoder.decode_packed(bytes(raw))
                elif isinstance(raw, str):
                    try:
                        raw_bytes = base64.b64decode(raw)
                        return encoder.decode_packed(raw_bytes)
                    except Exception:
                        pass

        # Fallback: check first item with resource.blob
        for item in resource_or_result:
            if hasattr(item, "resource") and hasattr(item.resource, "blob"):
                raw = base64.b64decode(item.resource.blob)
                return encoder.decode_packed(raw)

        raise BFastError("No B-FAST resource found in MCP content list")

    # 5. EmbeddedResource
    if hasattr(resource_or_result, "resource"):
        blob_data = getattr(resource_or_result.resource, "blob", None)
        if blob_data:
            raw = base64.b64decode(blob_data)
            return encoder.decode_packed(raw)

    # 6. BlobResourceContents
    if hasattr(resource_or_result, "blob"):
        raw = base64.b64decode(resource_or_result.blob)
        return encoder.decode_packed(raw)

    raise BFastError(
        f"Unsupported MCP resource type for B-FAST decoding: {type(resource_or_result)}"
    )


def bfast_tool(
    mcp_or_func: Any = None,
    *,
    compress: bool = True,
    include_summary: bool = True,
    summary_text: str | None = None,
    uri: str | None = None,
    name: str | None = None,
    description: str | None = None,
) -> Any:
    """Decorator to transform tool output into high-performance B-FAST binary MCP EmbeddedResource.

    Can be used in two ways:
    1. Direct decorator on function:
        @mcp.tool()
        @bfast_tool(compress=True)
        def my_tool(x: int):
            return {"result": x}

    2. Pass MCP server instance directly:
        @bfast_tool(mcp, compress=True)
        def my_tool(x: int):
            return {"result": x}
    """

    def make_wrapper(func: Callable) -> Callable:
        resource_uri = uri or f"bfast://tool/{func.__name__}/output"

        wrapper: Any
        if inspect.iscoroutinefunction(func):

            @functools.wraps(func)
            async def async_wrapper(
                *args: Any, **kwargs: Any
            ) -> list[TextContent | EmbeddedResource]:
                result = await func(*args, **kwargs)
                return encode_mcp_resource(
                    result,
                    uri=resource_uri,
                    compress=compress,
                    include_summary=include_summary,
                    summary_text=summary_text,
                )

            wrapper = async_wrapper
        else:

            @functools.wraps(func)
            def sync_wrapper(
                *args: Any, **kwargs: Any
            ) -> list[TextContent | EmbeddedResource]:
                result = func(*args, **kwargs)
                return encode_mcp_resource(
                    result,
                    uri=resource_uri,
                    compress=compress,
                    include_summary=include_summary,
                    summary_text=summary_text,
                )

            wrapper = sync_wrapper

        if hasattr(wrapper, "__annotations__"):
            wrapper.__annotations__["return"] = list[
                Union[TextContent, EmbeddedResource]
            ]

        return wrapper

    # If called as @bfast_tool (no parens)
    if callable(mcp_or_func) and not hasattr(mcp_or_func, "tool"):
        return make_wrapper(mcp_or_func)

    # If called with MCP server instance: @bfast_tool(mcp, ...)
    if mcp_or_func is not None and hasattr(mcp_or_func, "tool"):
        server = mcp_or_func

        def server_decorator(func: Callable) -> Callable:
            wrapped = make_wrapper(func)
            tool_decorator = (
                server.tool(name=name, description=description)
                if (name or description)
                else server.tool()
            )
            return tool_decorator(wrapped)

        return server_decorator

    # If called as @bfast_tool(compress=True, ...)
    def decorator(func: Callable) -> Callable:
        return make_wrapper(func)

    return decorator


def bfast_resource(
    mcp_or_uri: Any,
    uri: str | None = None,
    *,
    compress: bool = True,
    name: str | None = None,
    description: str | None = None,
) -> Any:
    """Decorator to expose Python data as a B-FAST binary MCP resource.

    Can be used as:
        @bfast_resource(mcp, "bfast://users/all", compress=True)
        def get_all_users():
            return db.users()
    """
    if hasattr(mcp_or_uri, "resource") and uri is not None:
        server = mcp_or_uri
        resource_uri = uri

        def server_resource_decorator(func: Callable) -> Callable:
            encoder = BFast()
            wrapped: Any
            if inspect.iscoroutinefunction(func):

                @functools.wraps(func)
                async def async_res_wrapper(*args: Any, **kwargs: Any) -> bytes:
                    val = await func(*args, **kwargs)
                    raw = encoder.encode_packed(val, compress=compress)
                    return bytes(raw) if isinstance(raw, list) else raw

                wrapped = async_res_wrapper
            else:

                @functools.wraps(func)
                def sync_res_wrapper(*args: Any, **kwargs: Any) -> bytes:
                    val = func(*args, **kwargs)
                    raw = encoder.encode_packed(val, compress=compress)
                    return bytes(raw) if isinstance(raw, list) else raw

                wrapped = sync_res_wrapper

            if hasattr(wrapped, "__annotations__"):
                wrapped.__annotations__["return"] = bytes

            resource_opts = {"mime_type": BFAST_MIME_TYPE}
            if name:
                resource_opts["name"] = name
            if description:
                resource_opts["description"] = description

            return server.resource(resource_uri, **resource_opts)(wrapped)

        return server_resource_decorator

    # If used without server instance: @bfast_resource("bfast://...")
    resource_uri = mcp_or_uri

    def decorator(func: Callable) -> Callable:
        encoder = BFast()
        wrapped: Any
        if inspect.iscoroutinefunction(func):

            @functools.wraps(func)
            async def async_res_wrapper(*args: Any, **kwargs: Any) -> bytes:
                val = await func(*args, **kwargs)
                raw = encoder.encode_packed(val, compress=compress)
                return bytes(raw) if isinstance(raw, list) else raw

            wrapped = async_res_wrapper
        else:

            @functools.wraps(func)
            def sync_res_wrapper(*args: Any, **kwargs: Any) -> bytes:
                val = func(*args, **kwargs)
                raw = encoder.encode_packed(val, compress=compress)
                return bytes(raw) if isinstance(raw, list) else raw

            wrapped = sync_res_wrapper

        if hasattr(wrapped, "__annotations__"):
            wrapped.__annotations__["return"] = bytes

        return wrapped

    return decorator


class FastMCPBFast:
    """Wrapper / Adapter for FastMCP or MCPServer with first-class B-FAST support."""

    def __init__(self, name_or_server: Any = "BFastMCPServer", **kwargs: Any):
        if hasattr(name_or_server, "tool") and hasattr(name_or_server, "resource"):
            self.server = name_or_server
        else:
            try:
                from fastmcp import FastMCP

                self.server = FastMCP(name_or_server, **kwargs)
            except ImportError:
                try:
                    from mcp.server.mcpserver import MCPServer

                    self.server = MCPServer(name_or_server, **kwargs)
                except ImportError:
                    raise ImportError(
                        "FastMCP or MCPServer is required. Install with: pip install 'bfast-py[fastmcp]'"
                    ) from None

    def __getattr__(self, name: str) -> Any:
        return getattr(self.server, name)

    def bfast_tool(
        self,
        compress: bool = True,
        include_summary: bool = True,
        summary_text: str | None = None,
        uri: str | None = None,
        name: str | None = None,
        description: str | None = None,
    ) -> Callable:
        """Register a tool whose return value is automatically encoded as a B-FAST binary EmbeddedResource."""
        return bfast_tool(
            self.server,
            compress=compress,
            include_summary=include_summary,
            summary_text=summary_text,
            uri=uri,
            name=name,
            description=description,
        )

    def bfast_resource(
        self,
        uri: str,
        compress: bool = True,
        name: str | None = None,
        description: str | None = None,
    ) -> Callable:
        """Register a resource whose return value is automatically encoded as B-FAST binary bytes."""
        return bfast_resource(
            self.server,
            uri=uri,
            compress=compress,
            name=name,
            description=description,
        )


__all__ = [
    "BFAST_MIME_TYPE",
    "BFAST_STREAM_MIME_TYPE",
    "FastMCPBFast",
    "bfast_resource",
    "bfast_tool",
    "decode_mcp_resource",
    "encode_mcp_resource",
    "is_bfast_stream_requested",
    "stream_mcp_async_tool_results",
    "stream_mcp_tool_results",
    "wrap_mcp_tool_output",
]
