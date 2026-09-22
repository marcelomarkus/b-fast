# ⚡ B-FAST Architecture & Optimizations

An in-depth look at the performance architecture and engine optimizations that make B-FAST one of the fastest binary serialization formats in Python and TypeScript.

---

## 🧭 Core Optimization Pillars

<div class="grid cards" markdown>

-   __🚀 Concrete Type Fast-Paths__

    ---

    Direct CPython pointer and type checks in Rust (`PyBool`, `PyLong`, `PyFloat`, `PyString`, `PyDict`, `PyList`), completely bypassing dynamic `hasattr` reflection.

-   __🧮 Zero-Copy NumPy Arrays__

    ---

    Direct memory mapping for NumPy tensors and ndarrays, achieving **14-96x speedup** compared to JSON and orjson.

-   __🗜️ Parallel LZ4 Compression__

    ---

    Payloads > 1MB automatically leverage Rayon multi-threading in Rust for near-instant compression with minimal CPU latency.

-   __🧠 Allocation-Free String Interning__

    ---

    String table pre-allocation on decode and pointer-equality dictionary lookups on encode eliminate tens of thousands of redundant allocations.

</div>

---

## 1. Zero-Reflection Fast Execution Paths

Standard serializers often rely on runtime reflection (`hasattr`, `getattr`) to inspect Python objects, which triggers costly attribute lookups and internal exceptions in the CPython runtime.

B-FAST eliminates this overhead by recognizing concrete Python types directly at the native layer:
- **Zero-Exception Overhead**: Standard primitives (`bool`, `int`, `float`, `str`, `dict`, `list`) bypass dynamic attribute probing entirely.
- **Direct Native Mapping**: Python objects are mapped immediately to binary buffers with minimal intermediate state.

**Performance Impact:**
- **Primitives Serialization:** Reduced from **118.4 ms** to **4.8 ms** (**~24x speedup**).
- **Nested Structures:** Reduced from **408.2 ms** to **34.0 ms** (**~12x speedup**).

---

## 2. Zero-Copy NumPy & SIMD Batch Encoding

NumPy arrays are serialized directly from their underlying contiguous C buffers:

```python
import numpy as np
from b_fast import BFast

encoder = BFast()
matrix = np.random.rand(1000, 1000)  # 8MB float64 array

# Direct memory view copy - zero JSON conversion
payload = encoder.encode_packed(matrix, compress=False)
```

### Performance on 8MB Array

| Format | Serialization Time | Speedup vs JSON | Speedup vs orjson |
| :--- | :---: | :---: | :---: |
| **B-FAST** | **3.29 ms** | **🚀 96x** | **🚀 14x** |
| **orjson** | **46.34 ms** | 6.9x | 1.0x |
| **JSON** | **318.21 ms** | 1.0x | 0.15x |

---

## 3. Parallel Multi-Threaded Compression

For payloads larger than 1MB, B-FAST automatically divides the binary buffer into chunks and compresses them in parallel using Rust's Rayon library.

```python
encoder = BFast()

# Automatically engages parallel multi-core compression
compressed = encoder.encode_packed(large_dataset, compress=True)
```

### Compression Guidelines

=== "When to enable `compress=True`"
    - **Slow or mobile networks (< 100 Mbps):** 89% reduction delivers up to **5.7x faster** round-trip transfer.
    - **Large datasets (> 100 KB):** Compression overhead is negligible compared to transmission savings.
    - **Disk or Redis Caching:** Saves extensive memory and cache eviction churn.

=== "When to use `compress=False`"
    - **Ultra-fast internal networks (10+ Gbps):** Raw serialization is already sub-millisecond.
    - **Real-time streaming feeds (< 1 KB per frame):** Avoid per-frame compression latency.
    - **CPU-bound environments:** Direct binary packing without LZ4 pass.

---

## 4. Decoder Pre-Allocation & Lazy Loading

When decoding large arrays of dictionaries or streaming frames:

1. **Pre-allocated String Table:** During the header parse, `PyString` objects are created once and stored in an indexed table. Object keys are resolved via pointer lookup, avoiding 50,000+ string allocations per payload.
2. **Lazy Type Loading:** Python's `datetime`, `date`, `time`, `UUID`, and `Decimal` classes are only imported when extended tags (`0x80`–`0x84`) are encountered in the payload, eliminating ~80 µs of eager import overhead per decode call.

---

## 5. Best Practices for Maximum Throughput

### 1. Reuse Encoder Instances

```python
# ✅ Recommended: Reuses internal buffers and string intern caches
encoder = BFast()
for batch in data_batches:
    payload = encoder.encode_packed(batch)

# ❌ Avoid: Creating a new encoder per batch reallocates buffers
for batch in data_batches:
    payload = BFast().encode_packed(batch)
```

### 2. Stream Large Feeds with `BFastStreamingResponse`

Instead of buffering gigabytes in memory, stream data incrementally with chunked framing:

```python
from b_fast import BFastStreamingResponse

@app.get("/telemetry")
async def stream_telemetry():
    async def feed():
        for chunk in telemetry_source:
            yield chunk
    return BFastStreamingResponse(feed())
```
