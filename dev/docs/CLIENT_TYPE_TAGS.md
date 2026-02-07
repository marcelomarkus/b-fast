# B-FAST Client Type Tags Implementation Guide

## Special Type Tags (0xD0 - 0xDF)

These tags preserve type information for automatic reconstruction in the TypeScript client.

### Tag Definitions

| Tag  | Type     | Format                                    | Client Type |
|------|----------|-------------------------------------------|-------------|
| 0xD1 | DateTime | `[tag][len:u32][iso8601:utf8]`           | `Date`      |
| 0xD2 | Date     | `[tag][len:u32][iso8601_date:utf8]`      | `Date`      |
| 0xD3 | Time     | `[tag][len:u32][iso8601_time:utf8]`      | `string`    |
| 0xD4 | UUID     | `[tag][len:u32][hex:utf8]`               | `string`    |
| 0xD5 | Decimal  | `[tag][len:u32][decimal_string:utf8]`    | `number`    |

### Examples

**DateTime (0xD1):**
```
0xD1 0x13 0x00 0x00 0x00 "2024-01-15T10:30:45"
→ new Date("2024-01-15T10:30:45")
```

**UUID (0xD4):**
```
0xD4 0x20 0x00 0x00 0x00 "550e8400e29b41d4a716446655440000"
→ "550e8400-e29b-41d4-a716-446655440000"
```

**Decimal (0xD5):**
```
0xD5 0x07 0x00 0x00 0x00 "1234.56"
→ 1234.56
```

## TypeScript Implementation

```typescript
class BFastDecoder {
  private decodeValue(view: DataView, offset: number): [any, number] {
    const tag = view.getUint8(offset++);
    
    switch (tag) {
      case 0xD1: // DateTime
      case 0xD2: // Date
        const [dtStr, dtOffset] = this.decodeString(view, offset);
        return [new Date(dtStr), dtOffset];
        
      case 0xD3: // Time
        const [timeStr, timeOffset] = this.decodeString(view, offset);
        return [timeStr, timeOffset];
        
      case 0xD4: // UUID
        const [hexStr, uuidOffset] = this.decodeString(view, offset);
        return [this.formatUUID(hexStr), uuidOffset];
        
      case 0xD5: // Decimal
        const [decStr, decOffset] = this.decodeString(view, offset);
        return [parseFloat(decStr), decOffset];
        
      // ... other tags (0x10-0x90)
    }
  }
  
  private decodeString(view: DataView, offset: number): [string, number] {
    const length = view.getUint32(offset, true);
    offset += 4;
    const bytes = new Uint8Array(view.buffer, view.byteOffset + offset, length);
    const str = new TextDecoder().decode(bytes);
    return [str, offset + length];
  }
  
  private formatUUID(hex: string): string {
    return `${hex.slice(0,8)}-${hex.slice(8,12)}-${hex.slice(12,16)}-${hex.slice(16,20)}-${hex.slice(20)}`;
  }
}
```

## Usage Example

```typescript
interface User {
  id: number;
  name: string;
  created_at: Date;      // Reconstructed from 0xD1
  user_id: string;       // Formatted from 0xD4
  balance: number;       // Parsed from 0xD5
}

const response = await fetch('/api/users');
const buffer = await response.arrayBuffer();
const users: User[] = BFastDecoder.decode(buffer);

// Types are preserved!
console.log(users[0].created_at instanceof Date); // true
console.log(typeof users[0].balance);             // "number"
```

## Python Backend (Automatic)

No changes needed in Python code. Types are automatically detected and tagged:

```python
from datetime import datetime
from decimal import Decimal
from uuid import UUID
from pydantic import BaseModel

class User(BaseModel):
    created_at: datetime  # → 0xD1
    user_id: UUID         # → 0xD4
    balance: Decimal      # → 0xD5

# Automatically serialized with correct tags
encoder.encode_packed(user)
```
