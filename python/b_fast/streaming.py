import inspect
from collections.abc import AsyncIterable, Iterable
from typing import Any, AsyncGenerator, Generator, Optional

from ._b_fast import (
    BFastStreamDecoder as _RustBFastStreamDecoder,
)
from ._b_fast import (
    BFastStreamEncoder as _RustBFastStreamEncoder,
)


class BFastStreamEncoder:
    """High-performance length-prefixed stream encoder for B-FAST.

    Encapsulates serializing arbitrary Python objects, Pydantic models, dicts,
    and NumPy arrays into length-prefixed B-FAST frames with optional handshake
    and end-of-stream markers.
    """

    def __init__(self) -> None:
        self._encoder = _RustBFastStreamEncoder()

    def encode_frame(self, obj: Any, compress: bool = True) -> bytes:
        """Encode a single object into a framed B-FAST chunk [length(4B) + type(1B) + flags(1B) + payload]."""
        return self._encoder.encode_frame(obj, compress=compress)

    @classmethod
    def get_handshake(cls) -> bytes:
        """Get the 4-byte stream handshake ('BS\\x01\\x00')."""
        return _RustBFastStreamEncoder.get_handshake()

    @classmethod
    def get_eos_frame(cls) -> bytes:
        """Get the 6-byte End-of-Stream frame."""
        return _RustBFastStreamEncoder.get_eos_frame()

    def encode_stream(
        self,
        iterable: Iterable[Any],
        compress: bool = True,
        include_handshake: bool = True,
        include_eos: bool = True,
    ) -> Generator[bytes, None, None]:
        """Synchronously stream-encode an iterable of objects."""
        if include_handshake:
            yield self.get_handshake()

        for item in iterable:
            yield self.encode_frame(item, compress=compress)

        if include_eos:
            yield self.get_eos_frame()

    async def encode_async_stream(
        self,
        async_iterable: AsyncIterable[Any],
        compress: bool = True,
        include_handshake: bool = True,
        include_eos: bool = True,
    ) -> AsyncGenerator[bytes, None]:
        """Asynchronously stream-encode an async iterable of objects."""
        if include_handshake:
            yield self.get_handshake()

        async for item in async_iterable:
            yield self.encode_frame(item, compress=compress)

        if include_eos:
            yield self.get_eos_frame()


class BFastStreamDecoder:
    """Stateful, zero-copy sliding buffer decoder for B-FAST streams.

    Resilient to arbitrary TCP fragmentation, partial packet boundaries,
    and multi-frame network segments.
    """

    def __init__(
        self,
        max_frame_size: Optional[int] = None,
        expect_handshake: Optional[bool] = None,
    ) -> None:
        kwargs: dict[str, Any] = {}
        if max_frame_size is not None:
            kwargs["max_frame_size"] = max_frame_size
        if expect_handshake is not None:
            kwargs["expect_handshake"] = expect_handshake
        self._decoder = _RustBFastStreamDecoder(**kwargs)

    def feed(self, chunk: bytes) -> list[Any]:
        """Feed arbitrary network bytes into the buffer and return decoded frames."""
        return self._decoder.feed(chunk)

    @property
    def is_eos(self) -> bool:
        """True if the End-of-Stream frame has been reached."""
        return self._decoder.is_eos

    @property
    def pending_bytes(self) -> int:
        """Number of unconsumed bytes currently waiting in the buffer."""
        return self._decoder.pending_bytes

    @property
    def handshake_received(self) -> bool:
        """True if the initial stream handshake has been processed."""
        return self._decoder.handshake_received

    def clear(self) -> None:
        """Clear all internal buffer state."""
        self._decoder.clear()

    def reset(self) -> None:
        """Reset internal decoder state for a new stream."""
        self._decoder.reset()

    def decode_stream(self, byte_stream: Iterable[bytes]) -> Generator[Any, None, None]:
        """Synchronously decode an iterable of raw byte chunks into items."""
        for chunk in byte_stream:
            items = self.feed(chunk)
            yield from items
            if self.is_eos:
                break

    async def decode_async_stream(
        self, async_byte_stream: AsyncIterable[bytes]
    ) -> AsyncGenerator[Any, None]:
        """Asynchronously decode an async iterable of raw byte chunks into items."""
        async for chunk in async_byte_stream:
            items = self.feed(chunk)
            for item in items:
                yield item
            if self.is_eos:
                break


def is_async_iterable(obj: Any) -> bool:
    """Helper to detect whether an object is an async iterable."""
    return isinstance(obj, AsyncIterable) or inspect.isasyncgen(obj)
