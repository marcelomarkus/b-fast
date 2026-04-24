from typing import Any

try:
    from fastapi import Response

    FASTAPI_AVAILABLE = True
except ImportError:
    # Fallback class to prevent crash on import
    class Response:
        def __init__(self, *args, **kwargs):
            pass

    FASTAPI_AVAILABLE = False


class BFastResponse(Response):
    media_type = "application/x-bfast"

    def __init__(self, content: Any, **kwargs):
        if not FASTAPI_AVAILABLE:
            raise ImportError(
                "FastAPI is required to use BFastResponse. Install it with 'pip install bfast-py[fastapi]'."
            )

        # Delayed import to avoid circular dependency
        from ._b_fast import BFast

        self.encoder = BFast()
        super().__init__(content=content, **kwargs)

    def render(self, content: Any) -> bytes:
        return self.encoder.encode_packed(content, compress=True)
