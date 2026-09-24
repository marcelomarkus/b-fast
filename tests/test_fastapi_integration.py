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


def test_is_bfast_requested_helper():
    from b_fast.fastapi import is_bfast_requested

    # Dict/Mapping
    assert is_bfast_requested({"accept": "application/vnd.bfast"}) is True
    assert is_bfast_requested({"accept": "application/x-bfast"}) is True
    assert (
        is_bfast_requested({"accept": "application/vnd.bfast;q=0.9, text/html"}) is True
    )
    assert is_bfast_requested({"accept": "application/json"}) is False
    assert is_bfast_requested({"accept": "*/*"}) is False
    assert is_bfast_requested({}) is False

    # ASGI scope
    assert (
        is_bfast_requested(
            {"type": "http", "headers": [(b"accept", b"application/vnd.bfast")]}
        )
        is True
    )
    assert (
        is_bfast_requested(
            {"type": "http", "headers": [(b"accept", b"application/json")]}
        )
        is False
    )

    # Header list
    assert is_bfast_requested([(b"accept", b"application/x-bfast")]) is True
    assert is_bfast_requested([("accept", "application/vnd.bfast")]) is True
    assert is_bfast_requested([("content-type", "application/json")]) is False


@pytest.mark.skipif(not HAS_FASTAPI, reason="FastAPI is not installed")
def test_bfast_middleware_content_negotiation():
    from fastapi import HTTPException
    from fastapi.responses import PlainTextResponse, Response

    from b_fast.fastapi import BFastMiddleware

    app = FastAPI()
    app.add_middleware(BFastMiddleware, compress=True)

    @app.get("/items")
    def get_items():
        return [{"id": 1, "name": "Item 1"}, {"id": 2, "name": "Item 2"}]

    @app.get("/text")
    def get_text():
        return PlainTextResponse("pure text")

    @app.get("/explicit", response_class=BFastResponse)
    def get_explicit():
        return {"explicit": True}

    @app.get("/error")
    def get_error():
        raise HTTPException(status_code=404, detail="Resource not found")

    @app.get("/empty")
    def get_empty():
        return Response(status_code=204)

    client = TestClient(app)
    decoder = b_fast.BFast()

    # 1. Default without Accept -> Standard JSON
    r_default = client.get("/items")
    assert r_default.status_code == 200
    assert "application/json" in r_default.headers["content-type"]
    assert r_default.json() == [
        {"id": 1, "name": "Item 1"},
        {"id": 2, "name": "Item 2"},
    ]

    # 2. With Accept: application/x-bfast -> Auto converted to B-FAST binary
    r_bfast1 = client.get("/items", headers={"accept": "application/x-bfast"})
    assert r_bfast1.status_code == 200
    assert r_bfast1.headers["content-type"] == "application/x-bfast"
    assert "content-length" in r_bfast1.headers
    decoded1 = decoder.decode_packed(r_bfast1.content, decompress=True)
    assert decoded1 == [{"id": 1, "name": "Item 1"}, {"id": 2, "name": "Item 2"}]

    # 3. With Accept: application/vnd.bfast -> Also supported as fallback
    r_bfast2 = client.get("/items", headers={"accept": "application/vnd.bfast"})
    assert r_bfast2.status_code == 200
    assert r_bfast2.headers["content-type"] == "application/x-bfast"
    decoded2 = decoder.decode_packed(r_bfast2.content, decompress=True)
    assert decoded2 == [{"id": 1, "name": "Item 1"}, {"id": 2, "name": "Item 2"}]

    # 4. Non-JSON response passes through untouched
    r_text = client.get("/text", headers={"accept": "application/x-bfast"})
    assert r_text.status_code == 200
    assert "text/plain" in r_text.headers["content-type"]
    assert r_text.content == b"pure text"

    # 5. Explicit BFastResponse passes through untouched
    r_explicit = client.get("/explicit", headers={"accept": "application/x-bfast"})
    assert r_explicit.status_code == 200
    assert r_explicit.headers["content-type"] == "application/x-bfast"
    assert decoder.decode_packed(r_explicit.content, decompress=True) == {
        "explicit": True
    }

    # 6. Error response (404) also converts to B-FAST if requested
    r_err = client.get("/error", headers={"accept": "application/x-bfast"})
    assert r_err.status_code == 404
    assert r_err.headers["content-type"] == "application/x-bfast"
    assert decoder.decode_packed(r_err.content, decompress=True) == {
        "detail": "Resource not found"
    }

    # 7. Status 204 No Content passes through untouched
    r_empty = client.get("/empty", headers={"accept": "application/x-bfast"})
    assert r_empty.status_code == 204
