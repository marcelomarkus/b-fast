"""
Tests for B-FAST Django and Django Ninja integrations.
"""

import polars as pl
from ninja import NinjaAPI
from ninja.testing import TestClient

from b_fast import BFast, BFastStreamDecoder
from b_fast.django import BFastHttpResponse, BFastRenderer, BFastStreamingHttpResponse


def test_django_ninja_bfast_renderer():
    api = NinjaAPI(renderer=BFastRenderer())

    @api.get("/users")
    def get_users(request):
        return [
            {"id": 1, "username": "alice", "email": "alice@example.com"},
            {"id": 2, "username": "bob", "email": "bob@example.com"},
        ]

    @api.get("/stats")
    def get_stats(request):
        return pl.DataFrame({"metric": ["cpu", "memory"], "usage": [45.2, 78.1]})

    client = TestClient(api)

    # Test users endpoint
    resp1 = client.get("/users")
    assert resp1.status_code == 200
    assert "application/x-bfast" in resp1.headers["content-type"]
    users = BFast().decode_packed(resp1.content)
    assert len(users) == 2
    assert users[0]["username"] == "alice"
    assert users[1]["username"] == "bob"

    # Test dataframe endpoint
    resp2 = client.get("/stats")
    assert resp2.status_code == 200
    stats = BFast().decode_packed(resp2.content)
    assert stats == [
        {"metric": "cpu", "usage": 45.2},
        {"metric": "memory", "usage": 78.1},
    ]


def test_django_bfast_http_response():
    data = {"status": "success", "items": [1, 2, 3]}
    response = BFastHttpResponse(data)

    assert response.status_code == 200
    assert response["Content-Type"] == "application/x-bfast"
    decoded = BFast().decode_packed(response.content)
    assert decoded == data


def test_django_bfast_streaming_http_response():
    def event_stream():
        for i in range(5):
            yield {"seq": i, "val": i * 10}

    response = BFastStreamingHttpResponse(event_stream())

    assert response.status_code == 200
    assert response["Content-Type"] == "application/x-bfast-stream"

    chunks = list(response.streaming_content)
    all_bytes = b"".join(chunks)

    decoder = BFastStreamDecoder()
    items = decoder.feed(all_bytes)
    assert len(items) == 5
    assert items[0] == {"seq": 0, "val": 0}
    assert items[4] == {"seq": 4, "val": 40}
