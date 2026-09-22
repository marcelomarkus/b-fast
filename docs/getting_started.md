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

B-FAST provides built-in `Response` and `StreamingResponse` classes for FastAPI and Starlette.

#### Standard Response
```python
from fastapi import FastAPI
from pydantic import BaseModel
from b_fast import BFastResponse

app = FastAPI()

class User(BaseModel):
    id: int
    name: str
    email: str

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]
```

#### Streamable HTTP Response (Continuous Streaming) 🌊
Stream binary frames in real-time over HTTP/1.1 (Chunked), HTTP/2, or HTTP/3:

```python
import asyncio
from fastapi import FastAPI
from b_fast import BFastStreamingResponse

app = FastAPI()

@app.get("/stream-users", response_class=BFastStreamingResponse)
async def stream_users():
    async def user_generator():
        for i in range(100):
            yield {"id": i, "name": f"User {i}", "status": "active"}
            await asyncio.sleep(0.05)
    
    return user_generator()
```

---

### Model Context Protocol (MCP) Integration 🤖
Transmit large masses of AI tool output using B-FAST Streamable HTTP:

```python
from b_fast import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    wrap_mcp_tool_output,
)

# Check content negotiation
if is_bfast_stream_requested(request.headers):
    # Stream generator output in binary
    return stream_mcp_async_tool_results(tool_data_generator())
```

## Next Steps

- [Frontend Integration](frontend.md) - TypeScript client setup & streaming
- [Performance](performance.md) - Detailed benchmarks
- [Troubleshooting](troubleshooting.md) - Common issues
