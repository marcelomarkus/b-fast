from collections.abc import AsyncIterable, Iterable, Mapping
from typing import Any, AsyncGenerator, Generator

from .streaming import BFastStreamEncoder


def is_bfast_stream_requested(headers: Mapping[str, str]) -> bool:
    """Check if the incoming request's Accept header requests B-FAST streaming."""
    accept = headers.get("accept") or headers.get("Accept") or ""
    return "application/x-bfast-stream" in accept or "application/x-bfast" in accept


def wrap_mcp_tool_output(data: Any, compress: bool = True) -> bytes:
    """Wrap tool output into a framed B-FAST packet for MCP Streamable HTTP transmission."""
    encoder = BFastStreamEncoder()
    return encoder.encode_frame(data, compress=compress)


def stream_mcp_tool_results(
    results: Iterable[Any],
    compress: bool = True,
    include_handshake: bool = True,
    include_eos: bool = True,
) -> Generator[bytes, None, None]:
    """Yield B-FAST framed chunks for progressive MCP tool results."""
    encoder = BFastStreamEncoder()
    return encoder.encode_stream(
        results,
        compress=compress,
        include_handshake=include_handshake,
        include_eos=include_eos,
    )


async def stream_mcp_async_tool_results(
    results: AsyncIterable[Any],
    compress: bool = True,
    include_handshake: bool = True,
    include_eos: bool = True,
) -> AsyncGenerator[bytes, None]:
    """Asynchronously yield B-FAST framed chunks for progressive MCP tool results."""
    encoder = BFastStreamEncoder()
    async for chunk in encoder.encode_async_stream(
        results,
        compress=compress,
        include_handshake=include_handshake,
        include_eos=include_eos,
    ):
        yield chunk
