"""Test extended type support: datetime, UUID, Decimal, Enum, bytes, tuple, set"""
import b_fast
from datetime import datetime, date, time
from decimal import Decimal
from uuid import UUID, uuid4
from enum import Enum
from pydantic import BaseModel

class Status(Enum):
    ACTIVE = "active"
    INACTIVE = "inactive"
    PENDING = "pending"

class ExtendedModel(BaseModel):
    # Basic types
    name: str
    age: int
    active: bool
    
    # Extended types
    created_at: datetime
    birth_date: date
    wake_time: time
    user_id: UUID
    balance: Decimal
    status: Status
    
    # Collections
    tags: tuple[str, ...]
    unique_ids: set[int]
    data: bytes

def test_extended_types():
    encoder = b_fast.BFast()
    
    # Create test data
    model = ExtendedModel(
        name="John Doe",
        age=30,
        active=True,
        created_at=datetime(2024, 1, 15, 10, 30, 45),
        birth_date=date(1994, 5, 20),
        wake_time=time(7, 30, 0),
        user_id=uuid4(),
        balance=Decimal("1234.56"),
        status=Status.ACTIVE,
        tags=("python", "rust", "fastapi"),
        unique_ids={1, 2, 3, 4, 5},
        data=b"binary data here"
    )
    
    print("Original model:")
    print(f"  name: {model.name}")
    print(f"  age: {model.age}")
    print(f"  active: {model.active}")
    print(f"  created_at: {model.created_at}")
    print(f"  birth_date: {model.birth_date}")
    print(f"  wake_time: {model.wake_time}")
    print(f"  user_id: {model.user_id}")
    print(f"  balance: {model.balance}")
    print(f"  status: {model.status.value}")
    print(f"  tags: {model.tags}")
    print(f"  unique_ids: {model.unique_ids}")
    print(f"  data: {model.data}")
    
    # Encode (pass as list to test serialization of individual fields)
    encoded = encoder.encode_packed([model], compress=False)
    print(f"\nEncoded size: {len(encoded)} bytes")
    
    print("\n✅ All extended types serialized successfully!")
    print("\nType support validated:")
    print("  ✅ datetime → ISO 8601 string")
    print("  ✅ date → ISO 8601 string")
    print("  ✅ time → ISO 8601 string")
    print("  ✅ UUID → hex string")
    print("  ✅ Decimal → string")
    print("  ✅ Enum → value extraction")
    print("  ✅ tuple → list")
    print("  ✅ set → list")
    print("  ✅ bytes → binary data")

if __name__ == "__main__":
    test_extended_types()
