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
    assert b_fast.__version__ == "1.10.0"


def test_pydantic_batch_homogeneous_encoding():
    """Test batch encoding of homogeneous Pydantic models."""
    encoder = b_fast.BFast()
    users = [UserModel(id=i, name=f"User {i}", active=(i % 2 == 0)) for i in range(50)]
    encoded = encoder.encode_packed(users, compress=False)
    decoded = encoder.decode_packed(encoded)
    assert len(decoded) == 50
    assert decoded[0] == {"id": 0, "name": "User 0", "active": True}
    assert decoded[1] == {"id": 1, "name": "User 1", "active": False}
    assert decoded[49] == {"id": 49, "name": "User 49", "active": False}


def test_dict_records_batch_encoding():
    """Test batch encoding of plain Python dictionaries."""
    encoder = b_fast.BFast()
    records = [
        {"id": i, "name": f"Item {i}", "price": i * 2.5, "in_stock": True}
        for i in range(50)
    ]
    encoded = encoder.encode_packed(records, compress=False)
    decoded = encoder.decode_packed(encoded)
    assert len(decoded) == 50
    assert decoded[0] == {"id": 0, "name": "Item 0", "price": 0.0, "in_stock": True}
    assert decoded[10] == {"id": 10, "name": "Item 10", "price": 25.0, "in_stock": True}


def test_dict_records_disordered_keys_fallback():
    """Test fallback when dictionary keys have varying order across items."""
    encoder = b_fast.BFast()
    records = [
        {"a": 1, "b": 2, "c": 3},
        {"b": 20, "a": 10, "c": 30},
        {"c": 300, "b": 200, "a": 100},
    ] * 10
    encoded = encoder.encode_packed(records, compress=False)
    decoded = encoder.decode_packed(encoded)
    assert len(decoded) == 30
    assert decoded[0] == {"a": 1, "b": 2, "c": 3}
    assert decoded[1] == {"a": 10, "b": 20, "c": 30}
    assert decoded[2] == {"a": 100, "b": 200, "c": 300}


if __name__ == "__main__":
    pytest.main([__file__])
