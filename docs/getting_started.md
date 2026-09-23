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

### FastMCP 2.0 Integration 🤖
Transmit large masses of AI tool output using B-FAST decorators to save up to 85% context tokens:

```python
from b_fast.fastmcp import FastMCPBFast, bfast_tool

mcp = FastMCPBFast("Analytics Server")

@mcp.tool()
@bfast_tool(compress=True)
def query_large_dataset(limit: int = 1000) -> list[dict]:
    return [{"id": i, "value": i * 1.5} for i in range(limit)]
```

---

## Next Steps

- [Integrations Guide](integrations.md) - Django Ninja, Django, Polars, and Pandas
- [Frontend Integration](frontend.md) - TypeScript client, TanStack Query, and Zod
- [AI & LLM Guide (`llms.txt`)](ai.md) - OpenCode, Cursor, and Claude Code instructions
- [Performance & Benchmarks](performance.md) - Technical benchmarks vs orjson and JSON
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
