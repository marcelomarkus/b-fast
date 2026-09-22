from collections.abc import AsyncIterable, Iterable, Mapping
from typing import Any, Optional, Union

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
    Pydantic models, dicts, or NumPy arrays.
    """

    media_type = "application/x-bfast-stream"

    def __init__(
        self,
        content: Union[Iterable[Any], AsyncIterable[Any]],
        compress: bool = True,
        include_handshake: bool = True,
        include_eos: bool = True,
        status_code: int = 200,
        headers: Optional[Mapping[str, str]] = None,
        background: Any = None,
        **kwargs: Any,
    ) -> None:
        if not FASTAPI_AVAILABLE:
            raise ImportError(
                "FastAPI/Starlette is required to use BFastStreamingResponse. "
                "Install it with 'pip install bfast-py[fastapi]'."
            )

        from .streaming import BFastStreamEncoder, is_async_iterable

        encoder = BFastStreamEncoder()

        if is_async_iterable(content):
            stream_body = encoder.encode_async_stream(
                content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
            )
        elif isinstance(content, Iterable):
            stream_body = encoder.encode_stream(
                content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
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
