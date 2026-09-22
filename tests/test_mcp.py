import pytest

from b_fast.mcp import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    stream_mcp_tool_results,
    wrap_mcp_tool_output,
)
from b_fast.streaming import BFastStreamDecoder


def test_is_bfast_stream_requested():
    assert is_bfast_stream_requested({"accept": "application/x-bfast-stream"}) is True
    assert is_bfast_stream_requested({"Accept": "application/x-bfast"}) is True
    assert (
        is_bfast_stream_requested({"accept": "application/json, text/plain"}) is False
    )
    assert is_bfast_stream_requested({}) is False
    assert is_bfast_stream_requested({"other": "header"}) is False


def test_wrap_mcp_tool_output():
    tool_result = {
        "columns": ["id", "score"],
        "rows": [[1, 0.95], [2, 0.88], [3, 0.72]],
    }
    framed_bytes = wrap_mcp_tool_output(tool_result, compress=True)
    assert isinstance(framed_bytes, bytes)
    assert len(framed_bytes) > 6

    decoder = BFastStreamDecoder()
    decoded = decoder.feed(framed_bytes)
    assert len(decoded) == 1
    assert decoded[0] == tool_result


def test_stream_mcp_tool_results():
    results = [{"tool_step": i, "result": i**2} for i in range(5)]
    stream = list(stream_mcp_tool_results(results, compress=True))
    assert len(stream) > 0

    full_payload = b"".join(stream)
    decoder = BFastStreamDecoder()
    decoded = list(decoder.decode_stream([full_payload]))
    assert decoded == results


@pytest.mark.anyio
async def test_stream_mcp_async_tool_results():
    results = [{"batch_id": i, "data": [i, i + 1]} for i in range(4)]

    async def async_results_gen():
        for r in results:
            yield r

    chunks = []
    async for chunk in stream_mcp_async_tool_results(
        async_results_gen(), compress=True
    ):
        chunks.append(chunk)

    full_payload = b"".join(chunks)
    decoder = BFastStreamDecoder()
    decoded = list(decoder.decode_stream([full_payload]))
    assert decoded == results
