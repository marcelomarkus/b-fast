"""
Django and Django Ninja integrations for B-FAST.

Provides:
- BFastRenderer: Native Django Ninja BaseRenderer for high-performance binary APIs.
- BFastHttpResponse: Django HttpResponse serializing data to B-FAST binary format.
- BFastStreamingHttpResponse: Django StreamingHttpResponse streaming framed B-FAST chunks.
"""

from collections.abc import AsyncIterable, Iterable
from typing import Any, Optional, Union

try:
    from django.http import HttpResponse, StreamingHttpResponse

    DJANGO_AVAILABLE = True
except Exception:
    DJANGO_AVAILABLE = False

    class HttpResponse:  # type: ignore[no-redef]
        def __init__(self, *args: Any, **kwargs: Any) -> None:
            pass

    class StreamingHttpResponse:  # type: ignore[no-redef]
        def __init__(self, *args: Any, **kwargs: Any) -> None:
            pass


NINJA_AVAILABLE = False


class BaseRenderer:  # type: ignore[no-redef]
    media_type: Optional[str] = None
    charset: str = "utf-8"

    def render(self, request: Any, data: Any, *, response_status: int = 200) -> Any:
        raise NotImplementedError


try:
    import sys

    if "django.conf" in sys.modules:
        from django.conf import settings

        if getattr(settings, "configured", False):
            from ninja.renderers import BaseRenderer as _NinjaBaseRenderer

            BaseRenderer = _NinjaBaseRenderer  # type: ignore[misc]
            NINJA_AVAILABLE = True
except Exception:
    pass


class BFastRenderer(BaseRenderer):
    """
    Django Ninja renderer that serializes API responses using B-FAST.

    Usage:
        ```python
        from ninja import NinjaAPI
        from b_fast.django import BFastRenderer

        api = NinjaAPI(renderer=BFastRenderer())


        @api.get("/users")
        def list_users(request):
            return [{"id": 1, "name": "Alice"}]
        ```
    """

    media_type = "application/x-bfast"
    charset = "utf-8"

    def __init__(self, compress: bool = True) -> None:
        global NINJA_AVAILABLE
        if not NINJA_AVAILABLE:
            try:
                from ninja.renderers import BaseRenderer  # noqa: F401

                NINJA_AVAILABLE = True
            except Exception:
                raise ImportError(
                    "django-ninja is required to use BFastRenderer. "
                    "Install it with 'pip install bfast-py[django]' or 'pip install django-ninja'."
                ) from None
        self.compress = compress
        self._encoder: Optional[Any] = None

    def render(self, request: Any, data: Any, *, response_status: int = 200) -> bytes:
        if data is None:
            return b""
        if self._encoder is None:
            from ._b_fast import BFast

            self._encoder = BFast()
        return self._encoder.encode_packed(data, compress=self.compress)


class BFastHttpResponse(HttpResponse):
    """
    Django HttpResponse that serializes data into B-FAST binary format.

    Usage:
        ```python
        from b_fast.django import BFastHttpResponse


        def my_view(request):
            data = {"message": "Hello from Django!", "items": [1, 2, 3]}
            return BFastHttpResponse(data)
        ```
    """

    def __init__(
        self,
        data: Any = None,
        compress: bool = True,
        content_type: str = "application/x-bfast",
        status: int = 200,
        **kwargs: Any,
    ) -> None:
        global DJANGO_AVAILABLE
        if not DJANGO_AVAILABLE:
            try:
                from django.http import HttpResponse  # noqa: F401

                DJANGO_AVAILABLE = True
            except Exception:
                raise ImportError(
                    "Django is required to use BFastHttpResponse. "
                    "Install it with 'pip install bfast-py[django]' or 'pip install django'."
                ) from None

        from ._b_fast import BFast

        encoder = BFast()
        content = (
            encoder.encode_packed(data, compress=compress) if data is not None else b""
        )

        super().__init__(
            content=content,
            content_type=content_type,
            status=status,
            **kwargs,
        )


class BFastStreamingHttpResponse(StreamingHttpResponse):
    """
    Django StreamingHttpResponse that streams framed B-FAST binary chunks.

    Supports both sync and async iterables/generators of Python objects,
    dicts, Pydantic models, Polars/Pandas DataFrames, or NumPy arrays.

    Usage:
        ```python
        from b_fast.django import BFastStreamingHttpResponse


        def stream_view(request):
            def generate_data():
                for i in range(100):
                    yield {"step": i, "val": i * 2}

            return BFastStreamingHttpResponse(generate_data())
        ```
    """

    def __init__(
        self,
        streaming_content: Union[Iterable[Any], AsyncIterable[Any]],
        compress: bool = True,
        include_handshake: bool = True,
        include_eos: bool = True,
        content_type: str = "application/x-bfast-stream",
        status: int = 200,
        **kwargs: Any,
    ) -> None:
        global DJANGO_AVAILABLE
        if not DJANGO_AVAILABLE:
            try:
                from django.http import StreamingHttpResponse  # noqa: F401

                DJANGO_AVAILABLE = True
            except Exception:
                raise ImportError(
                    "Django is required to use BFastStreamingHttpResponse. "
                    "Install it with 'pip install bfast-py[django]' or 'pip install django'."
                ) from None

        from .streaming import BFastStreamEncoder, is_async_iterable

        encoder = BFastStreamEncoder()

        if is_async_iterable(streaming_content):
            stream_body = encoder.encode_async_stream(
                streaming_content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
            )
        elif isinstance(streaming_content, Iterable):
            stream_body = encoder.encode_stream(
                streaming_content,
                compress=compress,
                include_handshake=include_handshake,
                include_eos=include_eos,
            )
        else:
            raise ValueError(
                "streaming_content must be an Iterable or AsyncIterable of serializable items."
            )

        super().__init__(
            streaming_content=stream_body,
            content_type=content_type,
            status=status,
            **kwargs,
        )
