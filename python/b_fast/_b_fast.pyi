from typing import Any

class BFast:
    """Ultra-fast binary serializer with Rust backend."""

    def __init__(self) -> None:
        """Initialize B-FAST encoder with empty string table."""
        ...

    def encode(self, data: Any) -> bytes:
        """
        Encode data to B-FAST binary format without compression.

        Args:
            data: Any serializable Python object

        Returns:
            Binary data in B-FAST format
        """
        ...

    def encode_packed(self, data: Any, *, compress: bool = False) -> bytes:
        """
        Encode data to B-FAST binary format with optional LZ4 compression.

        Args:
            data: Any serializable Python object
            compress: Enable LZ4 compression for large payloads

        Returns:
            Binary data in B-FAST format (optionally compressed)
        """
        ...

    def decode_packed(self, bytes: bytes, *, decompress: bool = True) -> Any:
        """
        Decode B-FAST binary data to Python objects.

        Args:
            bytes: bytes or bytearray containing B-FAST data (optionally compressed)
            decompress: Decompress B-FAST data if compressed, otherwise parse directly

        Returns:
            Decoded Python object
        """
        ...

    def decode_lazy(self, bytes: bytes, *, decompress: bool = True) -> BFastLazyView:
        """
        Lazily parse B-FAST binary payload into a zero-allocation view without eager deserialization.

        Args:
            bytes: Raw B-FAST byte buffer (compressed or uncompressed).
            decompress: Automatically decompress if payload is compressed.

        Returns:
            BFastLazyView wrapping the buffer for on-demand traversal.
        """
        ...

    def encode_secure(self, data: Any, key: bytes, *, compress: bool = False) -> bytes:
        """
        Encode and encrypt data using ChaCha20-Poly1305.

        Args:
            data: Any serializable Python object
            key: 32-byte encryption key
            compress: Enable LZ4 compression before encryption

        Returns:
            Encrypted binary data
        """
        ...

class BFastLazyView:
    """Zero-allocation on-demand deserialization view for B-FAST binary data."""

    def is_dict(self) -> bool: ...
    def is_list(self) -> bool: ...
    def __len__(self) -> int: ...
    def __getitem__(self, key: Any) -> Any: ...
    def get(self, key: str, default: Any = None) -> Any: ...
    def keys(self) -> list[str]: ...
    def __contains__(self, key: str) -> bool: ...
    def materialize(self) -> Any: ...
    def to_dict(self) -> dict[str, Any]: ...
    def to_list(self) -> list[Any]: ...

class BFastStreamDecoder:
    """State-machine streaming decoder for framed B-FAST chunks."""

    def __init__(
        self, *, max_frame_size: int = ..., expect_handshake: bool | None = ...
    ) -> None: ...
    def feed(self, chunk: bytes) -> list[Any]: ...
    @property
    def is_eos(self) -> bool: ...
    @property
    def pending_bytes(self) -> int: ...
    @property
    def handshake_received(self) -> bool: ...
    def clear(self) -> None: ...
    def reset(self) -> None: ...

class BFastStreamEncoder:
    """Encoder for generating length-prefixed B-FAST streaming frames."""

    def __init__(self) -> None: ...
    def encode_frame(self, obj: Any, *, compress: bool = True) -> bytes: ...
    @staticmethod
    def get_handshake() -> bytes: ...
    @staticmethod
    def get_eos_frame() -> bytes: ...

class BFastNativeSender:
    """High-throughput native transport sender for B-FAST socket streams."""

    def __init__(
        self, *, max_in_flight: int = 128, buffer_size: int = 65536
    ) -> None: ...
    @property
    def is_native_active(self) -> bool: ...
    @property
    def pending_bytes(self) -> int: ...
    @property
    def in_flight_count(self) -> int: ...
    def clear_in_flight(self) -> None: ...
    def send_bytes(
        self, sock_fd: int, data: bytes, *, non_blocking: bool = False
    ) -> int: ...
    def send_frame(
        self,
        sock_fd: int,
        obj: Any,
        *,
        compress: bool = True,
        non_blocking: bool = False,
    ) -> int: ...
    def send_iterable(
        self,
        sock_fd: int,
        iterable: Any,
        *,
        compress: bool = True,
        non_blocking: bool = False,
    ) -> int: ...
    def send_handshake(self, sock_fd: int, *, non_blocking: bool = False) -> int: ...
    def send_eos(self, sock_fd: int, *, non_blocking: bool = False) -> int: ...
    def flush(self, sock_fd: int, *, non_blocking: bool = False) -> int: ...
    def close(self) -> None: ...

def is_native_available() -> bool:
    """Return True if native kernel socket transport is supported on this platform."""
    ...

def send_native_socket(sock_fd: int, data: bytes, *, non_blocking: bool = False) -> int:
    """Send bytes directly to socket descriptor using native transport."""
    ...

class BFastError(Exception):
    """Base exception for B-FAST operations."""

    pass
