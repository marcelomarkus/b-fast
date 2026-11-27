from collections.abc import AsyncIterable, Iterable, Mapping
from typing import Any

try:
    from fastapi.responses import Response, StreamingResponse

    FASTAPI_AVAILABLE = True
except ImportError:
    try:
        from starlette.responses import Response, StreamingResponse

        FASTAPI_AVAILABLE = True
    except ImportError:

        class Response:  # type: ignore[no-redef]
            def __init__(self, *args: Any, **kwargs: Any) -> None:
                pass

        class StreamingResponse:  # type: ignore[no-redef]
            def __init__(self, *args: Any, **kwargs: Any) -> None:
                pass

        FASTAPI_AVAILABLE = False


class BFastResponse(Response):
    """FastAPI/Starlette response class that serializes payloads using B-FAST."""

    media_type = "application/x-bfast"

    def __init__(self, content: Any = None, **kwargs: Any) -> None:
        if not FASTAPI_AVAILABLE:
            raise ImportError(
                "FastAPI/Starlette is required to use BFastResponse. "
                "Install it with 'pip install bfast-py[fastapi]'."
            )

        # Delayed import to avoid circular dependency
        from ._b_fast import BFast

        self.encoder = BFast()
        super().__init__(content=content, **kwargs)

    def render(self, content: Any) -> bytes:
        if content is None:
            return b""
        return self.encoder.encode_packed(content, compress=True)


class BFastStreamingResponse(StreamingResponse):
    """FastAPI/Starlette streaming response class that streams framed B-FAST chunks.

    Supports both sync and async iterables/generators of Python objects,
    Pydantic models, dicts, or NumPy arrays. Features adaptive buffer coalescing
    and cross-platform zero-copy socket acceleration.
    """

    media_type = "application/x-bfast-stream"

    def __init__(
        self,
        content: Iterable[Any] | AsyncIterable[Any],
        compress: bool = True,
        include_handshake: bool = True,
        include_eos: bool = True,
        status_code: int = 200,
        headers: Mapping[str, str] | None = None,
        background: Any = None,
        native: bool = False,
        chunk_size: int = 65536,
        **kwargs: Any,
    ) -> None:
        if not FASTAPI_AVAILABLE:
            raise ImportError(
                "FastAPI/Starlette is required to use BFastStreamingResponse. "
                "Install it with 'pip install bfast-py[fastapi]'."
            )

        from .streaming import BFastStreamEncoder, is_async_iterable

        self.encoder = BFastStreamEncoder()
        self.compress = compress
        self.include_handshake = include_handshake
        self.include_eos = include_eos
        self.native = native
        self.chunk_size = chunk_size
        self._raw_content = content

        if is_async_iterable(content):
            stream_body = self.encoder.encode_async_stream(
                content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
                chunk_size=chunk_size,
            )
        elif isinstance(content, Iterable):
            stream_body = self.encoder.encode_stream(
                content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
                chunk_size=chunk_size,
            )
        else:
            raise ValueError(
                "Content must be an Iterable or AsyncIterable of serializable items."
            )

        super().__init__(
            content=stream_body,
            status_code=status_code,
            headers=dict(headers) if headers is not None else None,
            media_type=self.media_type,
            background=background,
            **kwargs,
        )

    async def __call__(self, scope: Any, receive: Any, send: Any) -> None:
        """ASGI handler with transparent direct socket transport optimization."""
        if self.native and scope.get("type") == "http":
            # Attempt to extract raw socket transport for zero-copy kernel bypass
            sock = None
            transport = scope.get("transport")
            if transport is None and hasattr(send, "__self__"):
                transport = getattr(send.__self__, "transport", None)

            if transport is not None and hasattr(transport, "get_extra_info"):
                sock = transport.get_extra_info("socket")

            if sock is not None and hasattr(sock, "fileno"):
                from .streaming import BFastSocketStreamer, is_async_iterable

                # Send HTTP response start header
                raw_headers = getattr(self, "raw_headers", [])
                await send(
                    {
                        "type": "http.response.start",
                        "status": self.status_code,
                        "headers": raw_headers,
                    }
                )

                streamer = BFastSocketStreamer(
                    sock,
                    compress=self.compress,
                    native=True,
                    buffer_size=self.chunk_size,
                )
                try:
                    if is_async_iterable(self._raw_content):
                        await streamer.stream_async(
                            self._raw_content,
                            include_handshake=self.include_handshake,
                            include_eos=self.include_eos,
                        )
                    else:
                        streamer.stream(
                            self._raw_content,
                            include_handshake=self.include_handshake,
                            include_eos=self.include_eos,
                        )
                finally:
                    streamer.close()

                await send(
                    {"type": "http.response.body", "body": b"", "more_body": False}
                )
                if self.background is not None:
                    await self.background()
                return

        # Fallback to standard ASGI stream loop with coalesced chunks
        await super().__call__(scope, receive, send)
