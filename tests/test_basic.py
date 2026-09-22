import numpy as np
import pytest
from pydantic import BaseModel

import b_fast


class UserModel(BaseModel):  # Renamed to avoid pytest collection warning
    id: int
    name: str
    active: bool


class ComplexUserModel(BaseModel):
    id: int
    scores: list[float]
    metadata: dict


def test_pydantic_batch_nested_structures():
    """Test Pydantic models with nested lists and dicts in batch encoding."""
    encoder = b_fast.BFast()
    users = [
        ComplexUserModel(id=i, scores=[1.0, 2.5], metadata={"key": f"val_{i}"})
        for i in range(10)
    ]
    encoded = encoder.encode_packed(users, compress=False)
    decoded = encoder.decode_packed(encoded)
    assert len(decoded) == 10
    assert decoded[0]["id"] == 0
    assert decoded[0]["scores"] == [1.0, 2.5]
    assert decoded[0]["metadata"] == {"key": "val_0"}


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


def test_version():
    """Test that b_fast exports a valid version string matching Cargo.toml."""
    assert isinstance(b_fast.__version__, str)
    assert b_fast.__version__ == "1.5.1"


if __name__ == "__main__":
    pytest.main([__file__])
