# Framework & Data Science Integrations

B-FAST provides first-class, drop-in integrations for modern Python API frameworks and data science libraries.

---

## ⚡ FastAPI & Starlette

B-FAST supports two seamless ways to integrate with [FastAPI](https://fastapi.tiangolo.com/) and [Starlette](https://www.starlette.io/):

---

### Approach 1: With Middleware (`BFastMiddleware`) — Automatic Content Negotiation

**When to use:** You have an existing FastAPI codebase and want to support B-FAST without modifying any route signatures.

`BFastMiddleware` inspects the HTTP `Accept` header. When a client requests `Accept: application/x-bfast`, it automatically serializes the route's response using B-FAST's sub-microsecond Rust engine. Regular web browsers and clients requesting standard JSON continue receiving JSON as usual:

```python
from fastapi import FastAPI
from b_fast.fastapi import BFastMiddleware

app = FastAPI()

# Add BFastMiddleware to enable automatic content negotiation across all routes
app.add_middleware(BFastMiddleware, compress=True)


@app.get("/users")
def get_users():
    # Regular browsers / curl get standard JSON
    # B-FAST enabled clients (e.g. bfastFetch) automatically get compressed B-FAST binary!
    return [{"id": i, "name": f"User {i}"} for i in range(1000)]
```

---

### Approach 2: Without Middleware (`BFastResponse` & `BFastStreamingResponse`) — Explicit Endpoints

**When to use:** You want explicit control over specific high-performance endpoints, microservices, or real-time event streaming with zero middleware layer overhead.

#### Standard Dictionaries & Streaming

```python
from fastapi import FastAPI
from b_fast import BFastResponse, BFastStreamingResponse

app = FastAPI()


# 1. Direct binary response for dictionaries and lists
@app.get("/items", response_class=BFastResponse)
def get_items():
    return [{"id": 1, "value": "A"}, {"id": 2, "value": "B"}]


# 2. Direct streaming route with response_class
@app.get("/stream", response_class=BFastStreamingResponse)
async def stream_items():
    async def event_generator():
        for i in range(10):
            yield {"item": i}

    return event_generator()
```

#### Native Pydantic Models & High-Frequency Feeds

```python
from fastapi import FastAPI
from pydantic import BaseModel
from b_fast.fastapi import BFastResponse, BFastStreamingResponse

app = FastAPI()


class SensorData(BaseModel):
    sensor_id: int
    temperature: float


# Directly serializes Pydantic models in Rust skipping slow .model_dump()
@app.get("/telemetry", response_class=BFastResponse)
def get_telemetry():
    return [SensorData(sensor_id=i, temperature=20.5 + i * 0.1) for i in range(1000)]


# Ultra-high-throughput streaming feed (3.18M frames/sec)
@app.get("/feed")
def stream_feed():
    def event_generator():
        for i in range(100):
            yield {"step": i, "temperature": 24.5 + i * 0.1}

    # Streams binary framed chunks with Content-Type: application/x-bfast-stream
    return BFastStreamingResponse(event_generator())
```

---

## 🥷 Django Ninja & Django

### Django Ninja (`BFastRenderer`)

[Django Ninja](https://django-ninja.dev/) is the fastest-growing API framework in the Django ecosystem. With `BFastRenderer`, any Django Ninja API or router can serve binary B-FAST responses with zero boilerplate:

```python
from b_fast.django import BFastRenderer
from ninja import NinjaAPI

# Apply BFastRenderer globally to the API
api = NinjaAPI(renderer=BFastRenderer())


@api.get("/users")
def get_users(request):
    # Automatically serialized to compressed B-FAST binary
    return [
        {"id": 1, "name": "Alice", "role": "admin"},
        {"id": 2, "name": "Bob", "role": "member"},
    ]
```

You can also apply `BFastRenderer` to specific operations or routers:

```python
@api.get("/telemetry", renderer=BFastRenderer())
def get_telemetry(request):
    return {"sensors": [10.5, 20.3, 15.8]}
```

### Classic Django (`BFastHttpResponse` & `BFastStreamingHttpResponse`)

For standard Django views (function-based views, class-based views, or Django REST framework views):

```python
from b_fast.django import BFastHttpResponse, BFastStreamingHttpResponse
from django.http import HttpRequest


def user_view(request: HttpRequest):
    data = {"status": "ok", "users": ["Alice", "Bob"]}
    return BFastHttpResponse(data)


def telemetry_stream_view(request: HttpRequest):
    def event_generator():
        for i in range(100):
            yield {"step": i, "temperature": 20.0 + i * 0.1}

    return BFastStreamingHttpResponse(event_generator())
```

---

## 📊 Data Science: Polars, Pandas & PyArrow

B-FAST provides native, zero-friction serialization for DataFrames, Series, and PyArrow tables.

### Native Serialization in `BFast.encode_packed`

You can pass a Polars DataFrame, Pandas DataFrame, or PyArrow Table directly to `BFast.encode_packed()`:

```python
from b_fast import BFast
import polars as pl
import pandas as pd

bf = BFast()

# 1. Polars DataFrame
df_pl = pl.DataFrame({"id": [1, 2, 3], "name": ["Alice", "Bob", "Charlie"]})
bytes_pl = bf.encode_packed(df_pl, compress=True)

# 2. Pandas DataFrame
df_pd = pd.DataFrame({"id": [1, 2], "score": [98.5, 91.0]})
bytes_pd = bf.encode_packed(df_pd, compress=True)

# 3. Nested inside dicts/responses
payload = {
    "status": "success",
    "total": len(df_pl),
    "records": df_pl,  # Automatically serialized as list of row dicts!
}
bytes_nested = bf.encode_packed(payload, compress=True)
```

In TypeScript, `BFastDecoder.decode(bytes)` immediately receives an array of row objects `[{ id: 1, name: "Alice" }, ...]`, ready for TanStack Table, AG Grid, or charts without any server-side manual `.to_dict(orient="records")` conversion!

### Dedicated Data Science Helpers: `encode_dataframe` & `decode_dataframe`

When building data pipelines or high-performance Python-to-Python microservices, `encode_dataframe` and `decode_dataframe` give you explicit control over tabular orientation:

```python
from b_fast import encode_dataframe, decode_dataframe
import polars as pl

df = pl.DataFrame({"id": [1, 2, 3], "city": ["SP", "RJ", "BH"]})

# 1. 'records' orient: list of row dicts (ideal for REST APIs & frontends)
data_records = encode_dataframe(df, orient="records")

# 2. 'columns' orient: columnar dictionary {col: [vals]} (blazing fast, minimal memory)
data_columns = encode_dataframe(df, orient="columns")

# 3. 'split' orient: {'columns': [...], 'data': [[...], ...]}
data_split = encode_dataframe(df, orient="split")

# Reconstructing DataFrames
df_reconstructed = decode_dataframe(data_columns, engine="polars")  # or "pandas", "arrow", "auto"
```

### Supported Layouts & Engines

| Orientation | Python Structure | Ideal Use Case |
| :--- | :--- | :--- |
| `records` (default) | `[ {col1: val1, col2: val2}, ... ]` | REST APIs, TanStack Table, React, Browser UI |
| `columns` | `{ col1: [val1, ...], col2: [val2, ...] }` | High-volume analytics, microservices, chart buffers |
| `split` | `{ "columns": [...], "data": [[...], ...] }` | Database exports, pandas compatibility |
