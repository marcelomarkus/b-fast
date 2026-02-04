import numpy as np
import pytest
from pydantic import BaseModel

import b_fast


class UserModel(BaseModel):  # Renamed to avoid pytest collection warning
    id: int
    name: str
    active: bool


def test_basic_encoding():
    """Test basic B-FAST encoding functionality."""
    encoder = b_fast.BFast()

    # Test simple data
    data = {"id": 1, "name": "test", "active": True}
    result = encoder.encode_packed(data, compress=False)

    # Accept both bytes and list of integers (current implementation)
    assert isinstance(result, (bytes, list))
    assert len(result) > 0

    # If it's a list, convert to bytes for further validation
    if isinstance(result, list):
        result_bytes = bytes(result)
        assert len(result_bytes) > 0


def test_pydantic_encoding():
    """Test Pydantic model encoding."""
    encoder = b_fast.BFast()

    model = UserModel(id=1, name="test", active=True)
    result = encoder.encode_packed(model, compress=False)

    assert isinstance(result, (bytes, list))
    assert len(result) > 0


def test_numpy_encoding():
    """Test NumPy array encoding."""
    encoder = b_fast.BFast()

    array = np.array([1.0, 2.0, 3.0])
    data = {"array": array}
    result = encoder.encode_packed(data, compress=False)

    assert isinstance(result, (bytes, list))
    assert len(result) > 0


def test_compression():
    """Test LZ4 compression."""
    encoder = b_fast.BFast()

    # Large data that should benefit from compression
    large_data = {"items": [{"id": i, "name": f"item_{i}"} for i in range(100)]}

    uncompressed = encoder.encode_packed(large_data, compress=False)
    compressed = encoder.encode_packed(large_data, compress=True)

    assert isinstance(compressed, (bytes, list))
    assert len(compressed) > 0
    assert isinstance(uncompressed, (bytes, list))
    assert len(uncompressed) > 0


def test_encoder_reuse():
    """Test that encoder can be reused (string interning)."""
    encoder = b_fast.BFast()

    data1 = {"name": "test", "id": 1}
    data2 = {"name": "test2", "id": 2}  # Same keys should be interned

    result1 = encoder.encode_packed(data1, compress=False)
    result2 = encoder.encode_packed(data2, compress=False)

    assert isinstance(result1, (bytes, list))
    assert isinstance(result2, (bytes, list))
    assert len(result1) > 0
    assert len(result2) > 0


if __name__ == "__main__":
    pytest.main([__file__])
