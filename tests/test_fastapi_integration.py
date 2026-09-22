import asyncio
import importlib

import pytest

try:
    from fastapi import FastAPI
    from fastapi.testclient import TestClient

    HAS_FASTAPI = True
except ImportError:
    HAS_FASTAPI = False
from pydantic import BaseModel

import b_fast
from b_fast.integration import BFastResponse, BFastStreamingResponse
from b_fast.streaming import BFastStreamDecoder


class UserPayload(BaseModel):
    user_id: int
    username: str


def test_bfast_response_render():
    resp = BFastResponse(content={"ok": True, "count": 10})
    rendered = resp.render({"ok": True, "count": 10})
    assert isinstance(rendered, bytes)
    assert len(rendered) > 0

    # Test render None
    assert resp.render(None) == b""


@pytest.mark.skipif(not HAS_FASTAPI, reason="FastAPI is not installed")
def test_bfast_response_in_fastapi():
    app = FastAPI()

    @app.get("/user", response_class=BFastResponse)
    def get_user():
        return {"id": 1, "name": "Alice"}

    client = TestClient(app)
    response = client.get("/user")
    assert response.status_code == 200
    assert response.headers["content-type"] == "application/x-bfast"

    decoder = b_fast.BFast()
    data = decoder.decode_packed(response.content, decompress=True)
    assert data == {"id": 1, "name": "Alice"}


@pytest.mark.skipif(not HAS_FASTAPI, reason="FastAPI is not installed")
def test_bfast_streaming_response_sync_fastapi():
    app = FastAPI()

    @app.get("/stream-sync")
    def stream_sync():
        def generate_data():
            for i in range(5):
                yield {"index": i, "val": f"sync_{i}"}

        return BFastStreamingResponse(generate_data())

    client = TestClient(app)
    response = client.get("/stream-sync")
    assert response.status_code == 200
    assert response.headers["content-type"] == "application/x-bfast-stream"

    decoder = BFastStreamDecoder()
    items = list(decoder.decode_stream([response.content]))
    assert len(items) == 5
    assert items[0] == {"index": 0, "val": "sync_0"}
    assert items[4] == {"index": 4, "val": "sync_4"}


@pytest.mark.skipif(not HAS_FASTAPI, reason="FastAPI is not installed")
def test_bfast_streaming_response_async_fastapi():
    app = FastAPI()

    @app.get("/stream-async")
    async def stream_async():
        async def generate_async():
            for i in range(5):
                await asyncio.sleep(0.001)
                yield UserPayload(user_id=i, username=f"user_{i}")

        return BFastStreamingResponse(
            generate_async(),
            status_code=201,
            headers={"X-Custom-Stream": "Active"},
        )

    client = TestClient(app)
    response = client.get("/stream-async")
    assert response.status_code == 201
    assert response.headers["content-type"] == "application/x-bfast-stream"
    assert response.headers["x-custom-stream"] == "Active"

    decoder = BFastStreamDecoder()
    items = list(decoder.decode_stream([response.content]))
    assert len(items) == 5
    assert items[0] == {"user_id": 0, "username": "user_0"}
    assert items[4] == {"user_id": 4, "username": "user_4"}


def test_bfast_streaming_response_invalid_content():
    with pytest.raises(
        ValueError, match="Content must be an Iterable or AsyncIterable"
    ):
        BFastStreamingResponse(12345)  # Not an iterable


def test_fastapi_unavailable_fallbacks(monkeypatch):
    """Test behavior when FastAPI/Starlette is unavailable."""
    import b_fast.integration as integration_module

    # Mock FASTAPI_AVAILABLE = False
    monkeypatch.setattr(integration_module, "FASTAPI_AVAILABLE", False)

    with pytest.raises(
        ImportError, match="FastAPI/Starlette is required to use BFastResponse"
    ):
        integration_module.BFastResponse({"data": 1})

    with pytest.raises(
        ImportError, match="FastAPI/Starlette is required to use BFastStreamingResponse"
    ):
        integration_module.BFastStreamingResponse([1, 2, 3])


def test_fallback_dummy_classes():
    """Verify that when FastAPI and Starlette are uninstalled, import executes fallback classes."""
    from unittest.mock import patch

    import b_fast.integration as integration_module

    with patch.dict(
        "sys.modules",
        {
            "fastapi": None,
            "fastapi.responses": None,
            "starlette": None,
            "starlette.responses": None,
        },
    ):
        reloaded = importlib.reload(integration_module)
        assert reloaded.FASTAPI_AVAILABLE is False
        dummy_resp = reloaded.Response()
        dummy_stream = reloaded.StreamingResponse()
        assert dummy_resp is not None
        assert dummy_stream is not None

    # Test fastapi missing but starlette present
    with patch.dict(
        "sys.modules",
        {
            "fastapi": None,
            "fastapi.responses": None,
        },
    ):
        reloaded = importlib.reload(integration_module)
        assert reloaded.FASTAPI_AVAILABLE is True

    # Restore normal state
    importlib.reload(integration_module)
    assert integration_module.FASTAPI_AVAILABLE is True
