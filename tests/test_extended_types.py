"""Unit tests for extended type support in B-FAST"""

from datetime import date, datetime, time
from decimal import Decimal
from enum import Enum
from uuid import UUID, uuid4

from pydantic import BaseModel

import b_fast


class Color(Enum):
    RED = "red"
    GREEN = "green"
    BLUE = "blue"


class Priority(Enum):
    LOW = 1
    MEDIUM = 2
    HIGH = 3


def test_datetime_serialization():
    """Test datetime serialization to ISO 8601"""
    encoder = b_fast.BFast()
    dt = datetime(2024, 1, 15, 10, 30, 45)

    class Model(BaseModel):
        timestamp: datetime

    obj = Model(timestamp=dt)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_date_serialization():
    """Test date serialization to ISO 8601"""
    encoder = b_fast.BFast()
    d = date(2024, 1, 15)

    class Model(BaseModel):
        birth_date: date

    obj = Model(birth_date=d)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_time_serialization():
    """Test time serialization to ISO 8601"""
    encoder = b_fast.BFast()
    t = time(10, 30, 45)

    class Model(BaseModel):
        wake_time: time

    obj = Model(wake_time=t)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_uuid_serialization():
    """Test UUID serialization to hex string"""
    encoder = b_fast.BFast()
    uid = uuid4()

    class Model(BaseModel):
        user_id: UUID

    obj = Model(user_id=uid)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_decimal_serialization():
    """Test Decimal serialization to string"""
    encoder = b_fast.BFast()
    dec = Decimal("1234.56")

    class Model(BaseModel):
        balance: Decimal

    obj = Model(balance=dec)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_enum_string_serialization():
    """Test Enum with string values"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        color: Color

    obj = Model(color=Color.RED)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_enum_int_serialization():
    """Test Enum with int values"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        priority: Priority

    obj = Model(priority=Priority.HIGH)
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_tuple_serialization():
    """Test tuple serialization as list"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        tags: tuple[str, ...]

    obj = Model(tags=("python", "rust", "fastapi"))
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_set_serialization():
    """Test set serialization as list"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        unique_ids: set[int]

    obj = Model(unique_ids={1, 2, 3, 4, 5})
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_frozenset_serialization():
    """Test frozenset serialization as list"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        immutable_ids: frozenset[int]

    obj = Model(immutable_ids=frozenset({10, 20, 30}))
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_bytes_serialization():
    """Test bytes serialization"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        data: bytes

    obj = Model(data=b"binary data here")
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_bytearray_serialization():
    """Test bytearray serialization"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        buffer: bytes

    obj = Model(buffer=bytearray(b"mutable buffer"))
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_mixed_types():
    """Test model with multiple extended types"""
    encoder = b_fast.BFast()

    class ComplexModel(BaseModel):
        name: str
        age: int
        created_at: datetime
        user_id: UUID
        balance: Decimal
        color: Color
        tags: tuple[str, ...]
        active_ids: set[int]
        data: bytes

    obj = ComplexModel(
        name="Test User",
        age=30,
        created_at=datetime(2024, 1, 15, 10, 30, 45),
        user_id=uuid4(),
        balance=Decimal("999.99"),
        color=Color.BLUE,
        tags=("tag1", "tag2"),
        active_ids={1, 2, 3},
        data=b"test",
    )

    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_nested_collections():
    """Test nested collections with extended types"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        timestamps: list[datetime]
        id_sets: list[set[int]]

    obj = Model(
        timestamps=[
            datetime(2024, 1, 1, 0, 0, 0),
            datetime(2024, 1, 2, 0, 0, 0),
        ],
        id_sets=[{1, 2}, {3, 4}],
    )

    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_optional_extended_types():
    """Test optional extended types"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        optional_date: date | None = None
        optional_uuid: UUID | None = None

    # Test with None
    obj1 = Model()
    encoded1 = encoder.encode_packed([obj1], compress=False)
    assert len(encoded1) > 0

    # Test with values
    obj2 = Model(optional_date=date(2024, 1, 1), optional_uuid=uuid4())
    encoded2 = encoder.encode_packed([obj2], compress=False)
    assert len(encoded2) > len(encoded1)


def test_empty_collections():
    """Test empty collections"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        empty_tuple: tuple[str, ...]
        empty_set: set[int]

    obj = Model(empty_tuple=(), empty_set=set())
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_large_decimal():
    """Test large Decimal numbers"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        big_number: Decimal

    obj = Model(big_number=Decimal("123456789.123456789"))
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_microseconds_datetime():
    """Test datetime with microseconds"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        precise_time: datetime

    obj = Model(precise_time=datetime(2024, 1, 15, 10, 30, 45, 123456))
    encoded = encoder.encode_packed([obj], compress=False)

    assert len(encoded) > 0
    assert isinstance(encoded, bytes)


def test_compression_with_extended_types():
    """Test compression works with extended types"""
    encoder = b_fast.BFast()

    class Model(BaseModel):
        timestamp: datetime
        user_id: UUID
        balance: Decimal

    # Create multiple objects
    objects = [
        Model(
            timestamp=datetime(2024, 1, 1, 10, 30, i % 60),
            user_id=uuid4(),
            balance=Decimal(f"{i * 100}.50"),
        )
        for i in range(100)
    ]

    encoded_normal = encoder.encode_packed(objects, compress=False)
    encoded_compressed = encoder.encode_packed(objects, compress=True)

    assert len(encoded_compressed) < len(encoded_normal)
    assert isinstance(encoded_compressed, bytes)


if __name__ == "__main__":
    import sys

    print("🧪 Running B-FAST Extended Type Unit Tests\n")
    print("=" * 60)

    tests = [
        ("datetime serialization", test_datetime_serialization),
        ("date serialization", test_date_serialization),
        ("time serialization", test_time_serialization),
        ("UUID serialization", test_uuid_serialization),
        ("Decimal serialization", test_decimal_serialization),
        ("Enum (string) serialization", test_enum_string_serialization),
        ("Enum (int) serialization", test_enum_int_serialization),
        ("tuple serialization", test_tuple_serialization),
        ("set serialization", test_set_serialization),
        ("frozenset serialization", test_frozenset_serialization),
        ("bytes serialization", test_bytes_serialization),
        ("bytearray serialization", test_bytearray_serialization),
        ("mixed types", test_mixed_types),
        ("nested collections", test_nested_collections),
        ("optional extended types", test_optional_extended_types),
        ("empty collections", test_empty_collections),
        ("large Decimal", test_large_decimal),
        ("microseconds datetime", test_microseconds_datetime),
        ("compression with extended types", test_compression_with_extended_types),
    ]

    passed = 0
    failed = 0

    for name, test_func in tests:
        try:
            test_func()
            print(f"✅ {name}")
            passed += 1
        except Exception as e:
            print(f"❌ {name}: {e}")
            failed += 1

    print("=" * 60)
    print(f"\n📊 Results: {passed} passed, {failed} failed out of {len(tests)} tests")

    if failed > 0:
        sys.exit(1)
    else:
        print("✅ All tests passed!")
