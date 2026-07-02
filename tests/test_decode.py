import numpy as np
import pytest
from datetime import datetime, date, time
from decimal import Decimal
from uuid import UUID, uuid4
from pydantic import BaseModel
import b_fast

class UserModel(BaseModel):
    id: int
    name: str
    active: bool

def test_decode_simple_types():
    bf = b_fast.BFast()
    data = {
        "null": None,
        "true": True,
        "false": False,
        "small_int": 5,
        "large_int": 123456789012345,
        "float": 3.14159,
        "string": "Olá, B-FAST!",
        "bytes": b"binary data"
    }
    encoded = bf.encode_packed(data, compress=False)
    decoded = bf.decode_packed(encoded, decompress=False)
    assert decoded == data

def test_decode_nested_structures():
    bf = b_fast.BFast()
    data = {
        "list": [1, "two", 3.0, {"nested": True}],
        "dict": {"a": 1, "b": [2, 3]}
    }
    encoded = bf.encode_packed(data, compress=False)
    decoded = bf.decode_packed(encoded, decompress=False)
    assert decoded == data

def test_decode_extended_types():
    bf = b_fast.BFast()
    dt = datetime(2026, 7, 2, 18, 0, 0)
    d = date(2026, 7, 2)
    t = time(18, 0, 0)
    uid = uuid4()
    dec = Decimal("123.45")

    data = {
        "datetime": dt,
        "date": d,
        "time": t,
        "uuid": uid,
        "decimal": dec
    }
    encoded = bf.encode_packed(data, compress=False)
    decoded = bf.decode_packed(encoded, decompress=False)
    assert decoded == data

def test_decode_numpy_array():
    bf = b_fast.BFast()
    array = np.array([1.5, 2.5, 3.5])
    data = {"array": array}
    encoded = bf.encode_packed(data, compress=False)
    decoded = bf.decode_packed(encoded, decompress=False)
    # Tag 0x90 decodes back to a python list of floats, matching python decoder
    assert decoded == {"array": [1.5, 2.5, 3.5]}

def test_decode_pydantic_model():
    bf = b_fast.BFast()
    model = UserModel(id=42, name="Alice", active=True)
    encoded = bf.encode_packed(model, compress=False)
    decoded = bf.decode_packed(encoded, decompress=False)
    # Decodes back to a dictionary representing __dict__
    assert decoded == {"id": 42, "name": "Alice", "active": True}

def test_decode_single_chunk_compression():
    bf = b_fast.BFast()
    data = {"text": "A" * 5000}
    encoded = bf.encode_packed(data, compress=True)
    
    # Test decoding with decompression enabled
    decoded = bf.decode_packed(encoded, decompress=True)
    assert decoded == data

def test_decode_parallel_compression():
    bf = b_fast.BFast()
    # Create payload larger than PARALLEL_COMPRESSION_THRESHOLD (1,000,000 bytes)
    # 20,000 items with long strings will exceed 1MB easily
    large_data = {
        "items": [
            {"id": i, "name": f"user_item_{i}", "payload": "X" * 100}
            for i in range(12000)
        ]
    }
    encoded = bf.encode_packed(large_data, compress=True)
    assert len(encoded) > 0
    
    # Test decoding with parallel decompression
    decoded = bf.decode_packed(encoded, decompress=True)
    assert decoded == large_data

def test_decode_errors():
    bf = b_fast.BFast()
    
    # Buffer too small
    with pytest.raises(ValueError, match="(?i)buffer too small"):
        bf.decode_packed(b"", decompress=False)
        
    with pytest.raises(ValueError, match="(?i)buffer too small"):
        bf.decode_packed(b"B", decompress=False)
        
    # Invalid magic
    with pytest.raises(ValueError, match="Invalid B-FAST magic"):
        bf.decode_packed(b"XX1234", decompress=False)
