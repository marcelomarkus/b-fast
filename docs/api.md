# 📑 API Reference

Complete reference for all classes, methods, and functions in the B-FAST ecosystem across Python and TypeScript.

---

## 🧭 Overview

<div class="grid cards" markdown>

-   __⚡ Core Module (`b_fast`)__

    ---

    [`BFast`](#bfast-class) and [`BFastError`](#bfasterror-exception). High-throughput packed binary encoding and decoding with Rust core.

-   __🚀 Web Frameworks (`b_fast.integration` & `b_fast.django`)__

    ---

    [`BFastResponse`](#bfastresponse), [`BFastStreamingResponse`](#bfaststreamingresponse) for FastAPI/Starlette, and [`BFastRenderer`](#bfastrenderer) for Django Ninja.

-   __📊 Data Science (`b_fast.data`)__

    ---

    [`encode_dataframe`](#encode_dataframe) and [`decode_dataframe`](#decode_dataframe) for native Polars, Pandas, and PyArrow serialization.

-   __🤖 FastMCP 2.0 (`b_fast.fastmcp`)__

    ---

    [`FastMCPBFast`](#fastmcpbfast), [`bfast_tool`](#bfast_tool), and [`bfast_resource`](#bfast_resource) for high-performance AI Agent communication.

-   __💻 TypeScript Client (`bfast-client`)__

    ---

    [`bfastFetch`](#bfastfetch), [`bfastQueryOptions`](#bfastqueryoptions), [`BFastDecoder`](#bfastdecoder), and [`Streaming Utilities`](#streaming-utilities).

</div>

---

## ⚡ Core Module (`b_fast`)

### `BFast` (Class)

The primary encoder and decoder engine implemented in Rust with PyO3.

```python
from b_fast import BFast

bf = BFast()
```

#### Methods

##### `encode_packed(obj: Any, compress: bool = True) -> bytes`
Serializes any Python object (primitives, dicts, lists, Pydantic models, NumPy arrays, Polars/Pandas DataFrames) into the B-FAST packed binary format.

- **Parameters:**
    - `obj`: Object to serialize. Supports Pydantic v2 models, NumPy ndarrays, Polars/Pandas DataFrames and Series, datetime, UUID, Decimal, dicts, lists, and primitives.
    - `compress` *(bool, default=True)*: Enables LZ4 compression. Payload sizes > 1MB automatically use parallel chunk compression.
- **Returns:** `bytes` containing the packed binary payload.
- **Raises:** `BFastError` if an unsupported type cannot be serialized.

##### `decode_packed(data: Union[bytes, bytearray, memoryview]) -> Any`
Deserializes a B-FAST binary payload back into Python native structures.

- **Parameters:**
    - `data`: Binary buffer to decode.
- **Returns:** Native Python object (dict, list, primitive, NumPy array, datetime, etc.).
- **Raises:** `BFastError` if the payload is malformed or invalid.

---

### `BFastError` (Exception)

Raised whenever serialization or deserialization encounters invalid data or unsupported structures. Subclasses Python's `ValueError`.

---

## 🚀 Web Framework Integrations

### FastAPI / Starlette (`b_fast.integration`)

#### `BFastResponse`
FastAPI/Starlette response that encodes payloads directly into B-FAST binary.

```python
from b_fast import BFastResponse

@app.get("/data")
def get_data():
    return BFastResponse(my_data, compress=True)
```

- **Content-Type:** `application/x-bfast`
- **Parameters:**
    - `content`: Data to serialize.
    - `status_code` *(int, default=200)*: HTTP status code.

#### `BFastStreamingResponse`
FastAPI/Starlette streaming response for continuous chunked data feeds.

```python
from b_fast import BFastStreamingResponse

@app.get("/stream")
async def stream():
    async def event_generator():
        for i in range(10):
            yield {"count": i}
    return BFastStreamingResponse(event_generator())
```

- **Content-Type:** `application/x-bfast-stream`
- **Parameters:**
    - `content`: `Iterable` or `AsyncIterable` yielding Python objects.
    - `compress` *(bool, default=True)*: Compress individual frames with LZ4.
    - `include_handshake` *(bool, default=True)*: Include streaming handshake frame.
    - `include_eos` *(bool, default=True)*: Include End-Of-Stream frame.

---

### Django & Django Ninja (`b_fast.django`)

### BFastRenderer
Native Django Ninja `BaseRenderer` for high-throughput binary endpoints.

```python
from ninja import NinjaAPI
from b_fast.django import BFastRenderer

api = NinjaAPI(renderer=BFastRenderer())
```

- **Media Type:** `application/x-bfast`
- **Parameters:**
    - `compress` *(bool, default=True)*: Compress response with LZ4.

#### `BFastHttpResponse` (Django)
Standard Django `HttpResponse` subclass returning B-FAST binary data.

#### `BFastStreamingHttpResponse` (Django)
Standard Django `StreamingHttpResponse` subclass streaming framed B-FAST binary chunks.

---

## 📊 Data Science Module (`b_fast.data`)

### encode_dataframe
Serializes a Polars DataFrame/Series, Pandas DataFrame/Series, or PyArrow Table into B-FAST format.

```python
encode_dataframe(df: Any, orient: str = "records", compress: bool = True) -> bytes
```

- **Parameters:**
    - `df`: Tabular data object.
    - `orient` *(str, default="records")*: Layout orientation:
        - `"records"`: List of row dicts (`[ {col: val}, ... ]`). Best for REST APIs and frontends.
        - `"columns"`: Columnar dictionary (`{ col: [vals] }`). Blazing fast, minimal memory.
        - `"split"`: Dictionary with `{'columns': [...], 'data': [[...], ...]}`.
    - `compress` *(bool, default=True)*: Whether to apply LZ4 compression.
- **Returns:** B-FAST binary `bytes`.

### decode_dataframe
Reconstructs tabular data from B-FAST binary bytes.

```python
decode_dataframe(data: Union[bytes, bytearray], engine: str = "auto") -> Any
```

- **Parameters:**
    - `data`: B-FAST binary buffer.
    - `engine` *(str, default="auto")*: Engine to instantiate: `"auto"`, `"polars"`, `"pandas"`, or `"arrow"`.
- **Returns:** Instantiated DataFrame or Table.

---

## 🤖 FastMCP 2.0 Module (`b_fast.fastmcp`)

### FastMCPBFast
Drop-in replacement for FastMCP server providing pre-configured B-FAST binary tools and resources.

```python
FastMCPBFast(name: str, **kwargs)
```

### bfast_tool
Decorator that serializes tool return values into compressed B-FAST base64 blobs, reducing MCP token overhead by up to 85%.

```python
@bfast_tool(compress: bool = True)
```

### bfast_resource
Decorator defining a binary B-FAST resource on a FastMCP server.

```python
@bfast_resource(uri: str, name: Optional[str] = None, compress: bool = True)
```

### decode_mcp_resource
Decodes an embedded B-FAST resource or tool result back into Python structures.

```python
decode_mcp_resource(result: Any) -> Any
```

---

## 💻 TypeScript Client (`bfast-client`)

```bash
npm install bfast-client
```

### bfastFetch

```typescript
import { bfastFetch } from 'bfast-client';

const users = await bfastFetch<User[]>('/api/users', {
    schema: UserSchema, // optional Zod / Standard Schema validation
    compress: true,
});
```

### bfastQueryOptions

```typescript
import { useQuery } from '@tanstack/react-query';
import { bfastQueryOptions } from 'bfast-client';

const { data } = useQuery(
    bfastQueryOptions<User>({
        queryKey: ['user', 1],
        url: '/api/users/1',
        schema: UserSchema,
        staleTime: 5000,
    })
);
```

### bfastInfiniteQueryOptions
Generates infinite query options for pagination and continuous loading with `useInfiniteQuery`.

### BFastDecoder

```typescript
import { BFastDecoder } from 'bfast-client';

const data = BFastDecoder.decode<User>(buffer, {
    typedArrays: false, // Set true for zero-copy Float64Array
    schema: UserSchema, // Optional runtime schema validation
});
```

### BFastEncoder

```typescript
import { BFastEncoder } from 'bfast-client';

const bytes = BFastEncoder.encode(payload, { compress: true });
```

### Streaming Utilities

#### `decodeReadableStream<T>(stream: ReadableStream<Uint8Array>, options?: StreamDecodeOptions<T>): AsyncGenerator<T>`
Consumes a browser `ReadableStream` and yields decoded and validated objects as frames arrive over HTTP.

#### `BFastStreamEncoder.encodeFrame(data: any): Uint8Array`
Encodes an individual JavaScript object into a length-prefixed streaming frame.
