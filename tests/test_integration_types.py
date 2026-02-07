"""Integration test: Python encode → TypeScript decode with type preservation"""

import json
from datetime import date, datetime, time
from decimal import Decimal
from uuid import UUID

from pydantic import BaseModel

import b_fast


class TypedModel(BaseModel):
    name: str
    age: int
    created_at: datetime
    birth_date: date
    wake_time: time
    user_id: UUID
    balance: Decimal
    active: bool


def test_type_preservation():
    """Test that types are preserved through serialization"""
    encoder = b_fast.BFast()

    # Create test data
    model = TypedModel(
        name="John Doe",
        age=30,
        created_at=datetime(2024, 1, 15, 10, 30, 45),
        birth_date=date(1994, 5, 20),
        wake_time=time(7, 30, 0),
        user_id=UUID("550e8400-e29b-41d4-a716-446655440000"),
        balance=Decimal("1234.56"),
        active=True,
    )

    # Encode
    encoded = encoder.encode_packed([model], compress=False)

    # Save to file for TypeScript test
    with open("/tmp/bfast_typed_test.bin", "wb") as f:
        f.write(encoded)

    # Save expected values as JSON for comparison
    expected = {
        "name": model.name,
        "age": model.age,
        "created_at": model.created_at.isoformat(),
        "birth_date": model.birth_date.isoformat(),
        "wake_time": model.wake_time.isoformat(),
        "user_id": str(model.user_id),
        "balance": str(model.balance),
        "active": model.active,
    }

    with open("/tmp/bfast_typed_expected.json", "w") as f:
        json.dump(expected, f, indent=2)

    print("✅ Test data generated:")
    print(f"  Binary: /tmp/bfast_typed_test.bin ({len(encoded)} bytes)")
    print("  Expected: /tmp/bfast_typed_expected.json")
    print("\nExpected values:")
    print(f"  name: {expected['name']} (string)")
    print(f"  age: {expected['age']} (number)")
    print(f"  created_at: {expected['created_at']} (Date)")
    print(f"  birth_date: {expected['birth_date']} (Date)")
    print(f"  wake_time: {expected['wake_time']} (string)")
    print(f"  user_id: {expected['user_id']} (string)")
    print(f"  balance: {expected['balance']} (number)")
    print(f"  active: {expected['active']} (boolean)")

    print("\n📝 Run TypeScript test:")
    print("  cd client-ts && npm test")


if __name__ == "__main__":
    test_type_preservation()
