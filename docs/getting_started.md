# Quick Start Guide - Python Backend

## Installation
```bash
uv add bfast-py
# or
pip install bfast-py
```

## Basic Usage

### Simple Serialization
```python
import b_fast

# Create encoder
encoder = b_fast.BFast()

# Your data
data = [{"id": i, "name": f"User {i}"} for i in range(1000)]

# Serialize
encoded = encoder.encode_packed(data, compress=True)
print(f"Size: {len(encoded)} bytes")

# Deserialize
decoded = encoder.decode_packed(encoded)
```

### Compression
```python
# With compression (recommended for > 1KB)
compressed_data = encoder.encode_packed(data, compress=True)
```

### FastAPI Integration ⭐ Recommended

#### Custom Response
```python
from fastapi import Response
import b_fast

class BFastResponse(Response):
    media_type = "application/x-bfast"
    
    def __init__(self, content=None, *args, **kwargs):
        super().__init__(content, *args, **kwargs)
        self.encoder = b_fast.BFast()

    def render(self, content) -> bytes:
        return self.encoder.encode_packed(content, compress=True)
```

#### Route Application
```python
from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class User(BaseModel):
    id: int
    name: str
    email: str

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]
```

## Next Steps

- [Frontend Integration](frontend.md) - TypeScript client setup
- [Performance](performance.md) - Detailed benchmarks
- [Troubleshooting](troubleshooting.md) - Common issues
