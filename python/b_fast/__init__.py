"""
B-FAST: Binary Fast Adaptive Serialization Transfer

Ultra-fast binary serialization library with Rust backend.
"""

from ._b_fast import BFast, BFastError
from .data import decode_dataframe, encode_dataframe
from .django import BFastHttpResponse, BFastRenderer, BFastStreamingHttpResponse
from .fastmcp import (
    FastMCPBFast,
    bfast_resource,
    bfast_tool,
    decode_mcp_resource,
    encode_mcp_resource,
)
from .integration import BFastResponse, BFastStreamingResponse
from .mcp import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    stream_mcp_tool_results,
    wrap_mcp_tool_output,
)
from .streaming import BFastStreamDecoder, BFastStreamEncoder

try:
    from ._b_fast import __version__
except (ImportError, AttributeError):
    try:
        from importlib.metadata import version as _version

        __version__ = _version("bfast-py")
    except Exception:
        __version__ = "unknown"

__all__ = [
    "__version__",
    "BFast",
    "BFastError",
    "BFastResponse",
    "BFastStreamingResponse",
    "BFastRenderer",
    "BFastHttpResponse",
    "BFastStreamingHttpResponse",
    "encode_dataframe",
    "decode_dataframe",
    "BFastStreamEncoder",
    "BFastStreamDecoder",
    "wrap_mcp_tool_output",
    "stream_mcp_tool_results",
    "stream_mcp_async_tool_results",
    "is_bfast_stream_requested",
    "bfast_tool",
    "bfast_resource",
    "encode_mcp_resource",
    "decode_mcp_resource",
    "FastMCPBFast",
]
