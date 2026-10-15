import inspect
import os
import socket
from collections.abc import AsyncGenerator, AsyncIterable, Generator, Iterable
from typing import Any

from ._b_fast import (
    BFastNativeSender as _RustBFastNativeSender,
)
from ._b_fast import (
    BFastStreamDecoder as _RustBFastStreamDecoder,
)
from ._b_fast import (
    BFastStreamEncoder as _RustBFastStreamEncoder,
)
from ._b_fast import (
    is_native_available,
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
        max_frame_size: int | None = None,
        expect_handshake: bool | None = None,
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


class BFastSocketStreamer:
    """High-throughput socket streamer leveraging native OS transport acceleration when available.

    Streams framed B-FAST objects directly into raw sockets, asyncio StreamWriters,
    or file descriptors with native kernel transport on Linux.
    """

    def __init__(
        self,
        sock: Any,
        compress: bool = True,
        native: bool = True,
        max_in_flight: int = 128,
        buffer_size: int = 65536,
    ) -> None:
        self._sock = sock
        self._compress = compress
        self._fileno: int | None = None
        self._raw_sock: socket.socket | None = None

        if hasattr(sock, "fileno") and callable(sock.fileno):
            self._fileno = sock.fileno()
            if isinstance(sock, socket.socket):
                self._raw_sock = sock
        elif hasattr(sock, "get_extra_info"):
            s = sock.get_extra_info("socket")
            if s is not None and hasattr(s, "fileno"):
                self._fileno = s.fileno()
                if isinstance(s, socket.socket):
                    self._raw_sock = s
        elif isinstance(sock, int):
            self._fileno = sock
        else:
            raise TypeError(
                f"Expected socket, asyncio StreamWriter, or int file descriptor, got {type(sock).__name__}"
            )

        if self._fileno is None or self._fileno < 0:
            raise ValueError(f"Invalid socket file descriptor: {self._fileno}")

        self._native_requested = native
        self._sender: _RustBFastNativeSender | None = None
        self._fallback_encoder: BFastStreamEncoder | None = None

        if native and is_native_available():
            try:
                self._sender = _RustBFastNativeSender(
                    max_in_flight=max_in_flight, buffer_size=buffer_size
                )
            except Exception:
                self._sender = None

        if self._sender is None:
            self._fallback_encoder = BFastStreamEncoder()

        if self._raw_sock is not None:
            try:
                if (
                    hasattr(socket, "IPPROTO_TCP")
                    and hasattr(socket, "TCP_NODELAY")
                    and self._raw_sock.family in (socket.AF_INET, socket.AF_INET6)
                    and self._raw_sock.type == socket.SOCK_STREAM
                ):
                    self._raw_sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            except (AttributeError, OSError):
                pass

    @property
    def fileno(self) -> int:
        """Socket file descriptor."""
        assert self._fileno is not None
        return self._fileno

    @property
    def is_native_active(self) -> bool:
        """True if the native transport fast-path is active."""
        return self._sender is not None and self._sender.is_native_active

    @property
    def pending_bytes(self) -> int:
        """Number of bytes currently buffered in the native pipe waiting to be sent."""
        if self._sender is not None:
            return self._sender.pending_bytes
        return 0

    def send_handshake(self) -> int:
        """Send the 4-byte stream handshake ('BS\\x01\\x00') to the socket."""
        assert self._fileno is not None
        if self._sender is not None:
            return self._sender.send_handshake(self._fileno)
        handshake = BFastStreamEncoder.get_handshake()
        return self._send_raw_bytes(handshake)

    def send_frame(self, obj: Any) -> int:
        """Serialize an object into a framed B-FAST chunk and stream directly into the socket."""
        assert self._fileno is not None
        if self._sender is not None:
            return self._sender.send_frame(self._fileno, obj, compress=self._compress)
        assert self._fallback_encoder is not None
        frame = self._fallback_encoder.encode_frame(obj, compress=self._compress)
        return self._send_raw_bytes(frame)

    def send_eos(self) -> int:
        """Send the 6-byte End-of-Stream frame to the socket."""
        assert self._fileno is not None
        if self._sender is not None:
            return self._sender.send_eos(self._fileno)
        eos = BFastStreamEncoder.get_eos_frame()
        return self._send_raw_bytes(eos)

    def flush(self) -> int:
        """Flush any pending bytes in the native pipe into the socket."""
        assert self._fileno is not None
        if self._sender is not None:
            return self._sender.flush(self._fileno)
        return 0

    def close(self) -> None:
        """Release native transport pipe resources."""
        if self._sender is not None:
            self._sender.close()

    def _send_raw_bytes(self, data: bytes) -> int:
        if self._raw_sock is not None:
            self._raw_sock.sendall(data)
            return len(data)
        elif hasattr(self._sock, "write") and callable(self._sock.write):
            self._sock.write(data)
            return len(data)
        else:
            assert self._fileno is not None
            total = 0
            while total < len(data):
                n = os.write(self._fileno, data[total:])
                total += n
            return total

    def stream(
        self,
        iterable: Iterable[Any],
        include_handshake: bool = True,
        include_eos: bool = True,
    ) -> int:
        """Synchronously stream an iterable of objects directly into the socket."""
        assert self._fileno is not None
        total_bytes = 0
        if include_handshake:
            total_bytes += self.send_handshake()

        if self._sender is not None:
            total_bytes += self._sender.send_iterable(
                self._fileno, iterable, compress=self._compress
            )
        else:
            for item in iterable:
                total_bytes += self.send_frame(item)

        if include_eos:
            total_bytes += self.send_eos()

        self.flush()
        return total_bytes

    async def stream_async(
        self,
        async_iterable: AsyncIterable[Any],
        include_handshake: bool = True,
        include_eos: bool = True,
    ) -> int:
        """Asynchronously stream an async iterable of objects directly into the socket."""
        total_bytes = 0
        if include_handshake:
            total_bytes += self.send_handshake()

        async for item in async_iterable:
            total_bytes += self.send_frame(item)

        if include_eos:
            total_bytes += self.send_eos()

        self.flush()
        return total_bytes

    def __enter__(self) -> "BFastSocketStreamer":
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        self.flush()

    async def __aenter__(self) -> "BFastSocketStreamer":
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        self.flush()
