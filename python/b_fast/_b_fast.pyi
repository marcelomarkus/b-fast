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

class BFastError(Exception):
    """Base exception for B-FAST operations."""

    pass
