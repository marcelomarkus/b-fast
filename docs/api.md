# 📑 API Reference

Complete reference for all classes, methods, and functions in the B-FAST ecosystem.

---

## 🧭 Overview

<div class="grid cards" markdown>

-   __⚡ Core Module (`b_fast`)__

    ---

    [`BFast`](#bfast-class) and [`BFastError`](#bfasterror-exception). High-throughput packed binary encoding and decoding with Rust SIMD core.

-   __🚀 Integration (`b_fast.integration`)__

    ---

    [`BFastResponse`](#bfastresponse) and [`BFastStreamingResponse`](#bfaststreamingresponse) for FastAPI and Starlette.

-   __🤖 MCP (`b_fast.mcp`)__

    ---

    Helper functions for Anthropic's Model Context Protocol (MCP) Streamable HTTP transport.

-   __💻 TypeScript (`bfast-client`)__

    ---

    [`BFastDecoder`](#bfastdecoder-typescript) and [`Streaming Utilities`](#streaming-utilities-typescript) for browser and Node.js.

</div>

---

## ⚡ Core Module

### `BFast` (Class)

The primary encoder and decoder engine implemented in Rust with PyO3.

```python
from b_fast import BFast

encoder = BFast()
```

#### Methods

##### `encode_packed(obj: Any, compress: bool = True) -> bytes`
Serializes any Python object (primitives, dicts, lists, Pydantic models, NumPy arrays) into the B-FAST packed binary format.

- **Parameters:**
    - `obj`: Object to serialize. Supports Pydantic v2 models, NumPy ndarrays, datetime, UUID, Decimal, dicts, lists, and primitives.
    - `compress` *(bool, default=True)*: Enables LZ4 compression. Payload sizes > 1MB automatically use parallel multi-threaded compression.
- **Returns:** `bytes` containing the packed binary payload.
- **Raises:** `BFastError` if an unsupported type cannot be serialized.

##### `decode_packed(data: Union[bytes, bytearray, memoryview]) -> Any`
Deserializes a B-FAST binary payload back into Python native structures.

- **Parameters:**
    - `data`: Binary buffer to decode.
- **Returns:** Native Python object (dict, list, primitive, NumPy array, datetime, etc.).
- **Raises:** `BFastError` if the payload is malformed or checksum fails.

---

### `BFastError` (Exception)

Raised whenever serialization or deserialization encounters invalid data or unsupported structures. Subclasses Python's `ValueError`.

---



## 🚀 Integration Module (`b_fast.integration`)

### `BFastResponse`

FastAPI/Starlette response that encodes payloads directly into B-FAST.

```python
from b_fast import BFastResponse

@app.get("/data")
def get_data():
    return BFastResponse(my_data, compress=True)
```

- **Content-Type:** `application/x-bfast`
- **Parameters:**
    - `content`: Data to serialize.
    - `compress` *(bool, default=True)*: Whether to apply LZ4 compression.
    - `status_code` *(int, default=200)*: HTTP status code.
    - `headers` *(dict, optional)*: Custom HTTP headers.

---

### `BFastStreamingResponse`

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
    - `content`: `Generator` or `AsyncGenerator` yielding Python objects.
    - `compress` *(bool, default=False)*: Compress individual frames.
    - `status_code` *(int, default=200)*: HTTP status code.

---

## 🤖 MCP Module (`b_fast.mcp`)

### Functions

##### `is_bfast_stream_requested(request: Request) -> bool`
Checks if the incoming HTTP request contains `Accept: application/x-bfast-stream`.

##### `wrap_mcp_tool_output(result: Any, request: Optional[Request] = None, compress: bool = False)`
Wraps tool output into `BFastResponse` if negotiated by the client, or falls back to standard dictionary.

##### `stream_mcp_tool_results(results_generator: Iterable[Any], request: Optional[Request] = None, compress: bool = False)`
Streams synchronous MCP tool results via `BFastStreamingResponse`.

##### `stream_mcp_async_tool_results(results_async_generator: AsyncIterable[Any], request: Optional[Request] = None, compress: bool = False)`
Streams asynchronous MCP tool results via `BFastStreamingResponse`.

---

## 💻 TypeScript Client (`bfast-client`)

```bash
npm install bfast-client
```

### `BFastDecoder` (TypeScript)

```typescript
import { BFastDecoder } from "bfast-client";

const data = BFastDecoder.decode(arrayBuffer);
```

#### `BFastDecoder.decode(buffer: ArrayBuffer | Uint8Array): any`
Decodes a standard B-FAST binary payload (automatically detects and decompresses LZ4 if present).

---

### Streaming Utilities (TypeScript)

#### `decodeReadableStream(stream: ReadableStream<Uint8Array>, options?: StreamDecodeOptions): AsyncGenerator<any>`
Helper that consumes a Fetch API `ReadableStream` and yields decoded objects. Automatically handles chunk reassembly and stream completion.

```typescript
for await (const item of decodeReadableStream(response.body)) {
    console.log(item);
}
```

#### `decodeNodeStream(stream: NodeJS.ReadableStream, options?: StreamDecodeOptions): AsyncGenerator<any>`
Helper that consumes a Node.js readable stream and yields decoded objects.
