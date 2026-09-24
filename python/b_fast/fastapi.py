"""
FastAPI and Starlette integrations for B-FAST.

Provides:
- BFastMiddleware: Transparent Content Negotiation ASGI middleware that automatically
  serializes JSON responses into B-FAST binary when requested by the client.
- BFastResponse: FastAPI/Starlette response class for explicit B-FAST binary endpoints.
- BFastStreamingResponse: FastAPI/Starlette response class for framed binary streaming.
- is_bfast_requested: Helper function to inspect the Accept header.
"""

import json
from collections.abc import Sequence
from typing import Any, Callable, Optional

from .integration import BFastResponse, BFastStreamingResponse

DEFAULT_ACCEPTED_MEDIA_TYPES: tuple[str, ...] = (
    "application/x-bfast",
    "application/vnd.bfast",
)

__all__ = [
    "BFastMiddleware",
    "BFastResponse",
    "BFastStreamingResponse",
    "is_bfast_requested",
    "DEFAULT_ACCEPTED_MEDIA_TYPES",
]


def is_bfast_requested(
    headers_or_scope: Any,
    accepted_types: Sequence[str] = DEFAULT_ACCEPTED_MEDIA_TYPES,
) -> bool:
    """
    Check if the client requested B-FAST in the HTTP Accept header.

    Supports:
    - Starlette/FastAPI Request or Headers object
    - Mapping/dict of headers
    - Raw ASGI scope dictionary
    - Sequence of (bytes/str, bytes/str) header tuples
    """
    accept_val = ""
    if isinstance(headers_or_scope, dict) and "headers" in headers_or_scope:
        # ASGI scope
        for k, v in headers_or_scope.get("headers", []):
            if k.lower() == b"accept":
                accept_val = v.decode("latin1", errors="ignore").lower()
                break
    elif hasattr(headers_or_scope, "headers"):
        # Starlette/FastAPI Request object
        accept_val = headers_or_scope.headers.get("accept", "").lower()
    elif hasattr(headers_or_scope, "get"):
        # Dict or Header-like mapping
        accept_val = headers_or_scope.get("accept", "").lower()
    elif isinstance(headers_or_scope, (list, tuple)):
        # List of header tuples
        for k, v in headers_or_scope:
            if (isinstance(k, bytes) and k.lower() == b"accept") or (
                isinstance(k, str) and k.lower() == "accept"
            ):
                val_str = (
                    v.decode("latin1", errors="ignore")
                    if isinstance(v, bytes)
                    else str(v)
                )
                accept_val = val_str.lower()
                break

    for target in accepted_types:
        if target.lower() in accept_val:
            return True
    return False


class BFastMiddleware:
    """
    Transparent Content Negotiation ASGI middleware for FastAPI and Starlette.

    When an incoming HTTP request includes `Accept: application/x-bfast` (or
    `Accept: application/vnd.bfast`), this middleware intercepts standard JSON responses,
    parses the JSON payload, serializes it with B-FAST's sub-microsecond Rust engine,
    and returns `Content-Type: application/x-bfast`.

    If the client does NOT request B-FAST (e.g. standard browser or `Accept: application/json`),
    the response passes through untouched as standard JSON.

    Usage:
        ```python
        from fastapi import FastAPI
        from b_fast.fastapi import BFastMiddleware

        app = FastAPI()
        app.add_middleware(BFastMiddleware, compress=True)


        @app.get("/items")
        def get_items():
            # Returns JSON by default; automatically returns B-FAST when client requests it!
            return [{"id": 1, "name": "Item"}]
        ```
    """

    def __init__(
        self,
        app: Any,
        compress: bool = True,
        accepted_media_types: Optional[Sequence[str]] = None,
        media_type: str = "application/x-bfast",
    ) -> None:
        self.app = app
        self.compress = compress
        self.accepted_media_types = (
            tuple(accepted_media_types)
            if accepted_media_types is not None
            else DEFAULT_ACCEPTED_MEDIA_TYPES
        )
        self.media_type = media_type
        self._encoder: Optional[Any] = None

    def _get_encoder(self) -> Any:
        if self._encoder is None:
            from ._b_fast import BFast

            self._encoder = BFast()
        return self._encoder

    async def __call__(
        self,
        scope: dict[str, Any],
        receive: Callable[..., Any],
        send: Callable[..., Any],
    ) -> None:
        if scope.get("type") != "http":
            await self.app(scope, receive, send)
            return

        if not is_bfast_requested(scope, self.accepted_media_types):
            await self.app(scope, receive, send)
            return

        # Client requested B-FAST. Intercept response to check if it's JSON.
        response_start: Optional[dict[str, Any]] = None
        is_json_response = False
        body_chunks: list[bytes] = []

        async def send_wrapper(message: dict[str, Any]) -> None:
            nonlocal response_start, is_json_response

            msg_type = message.get("type")

            if msg_type == "http.response.start":
                response_start = message
                status = message.get("status", 200)
                # Skip 204 No Content, 304 Not Modified
                if status in (204, 304):
                    await send(message)
                    return

                headers = message.get("headers", [])
                content_type = ""
                for k, v in headers:
                    if k.lower() == b"content-type":
                        content_type = v.decode("latin1", errors="ignore").lower()
                        break

                if "application/json" in content_type:
                    is_json_response = True
                else:
                    # Non-JSON response (HTML, text, already bfast, image, etc.)
                    await send(message)

            elif msg_type == "http.response.body":
                if not is_json_response or response_start is None:
                    # Pass through
                    await send(message)
                    return

                body = message.get("body", b"")
                more_body = message.get("more_body", False)
                if body:
                    body_chunks.append(body)

                if not more_body:
                    # End of response body: convert to B-FAST
                    full_body = b"".join(body_chunks)
                    if full_body:
                        try:
                            parsed_data = json.loads(full_body)
                            encoder = self._get_encoder()
                            bfast_bytes = encoder.encode_packed(
                                parsed_data, compress=self.compress
                            )
                        except Exception:
                            # Fallback if body could not be parsed as JSON
                            bfast_bytes = full_body
                    else:
                        bfast_bytes = b""

                    # Reconstruct headers: update content-type and content-length
                    new_headers: list[tuple[bytes, bytes]] = []
                    content_type_bytes = self.media_type.encode("latin1")
                    content_length_bytes = str(len(bfast_bytes)).encode("latin1")

                    for k, v in response_start.get("headers", []):
                        lower_k = k.lower()
                        if lower_k == b"content-type":
                            new_headers.append((b"content-type", content_type_bytes))
                        elif lower_k == b"content-length":
                            new_headers.append(
                                (b"content-length", content_length_bytes)
                            )
                        else:
                            new_headers.append((k, v))

                    # Ensure content-length is present
                    if not any(k.lower() == b"content-length" for k, _ in new_headers):
                        new_headers.append((b"content-length", content_length_bytes))

                    new_response_start = dict(response_start)
                    new_response_start["headers"] = new_headers

                    await send(new_response_start)
                    await send(
                        {
                            "type": "http.response.body",
                            "body": bfast_bytes,
                            "more_body": False,
                        }
                    )
            else:
                await send(message)

        await self.app(scope, receive, send_wrapper)
