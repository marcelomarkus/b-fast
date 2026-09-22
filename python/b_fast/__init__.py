"""
B-FAST: Binary Fast Adaptive Serialization Transfer

Ultra-fast binary serialization library with Rust backend.
"""

from ._b_fast import BFast, BFastError
from .integration import BFastResponse, BFastStreamingResponse
from .mcp import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    stream_mcp_tool_results,
    wrap_mcp_tool_output,
)
from .streaming import BFastStreamDecoder, BFastStreamEncoder

__version__ = "1.4.0"
__all__ = [
    "BFast",
    "BFastError",
    "BFastResponse",
    "BFastStreamingResponse",
    "BFastStreamEncoder",
    "BFastStreamDecoder",
    "wrap_mcp_tool_output",
    "stream_mcp_tool_results",
    "stream_mcp_async_tool_results",
    "is_bfast_stream_requested",
]
