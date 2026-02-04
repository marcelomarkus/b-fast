from typing import Any

from fastapi import Response

import b_fast


class BFastResponse(Response):
    media_type = "application/x-bfast"

    def __init__(self, content: Any, **kwargs):
        super().__init__(content=content, **kwargs)
        self.encoder = b_fast.BFast()

    def render(self, content: Any) -> bytes:
        return self.encoder.encode_packed(content, compress=True)
