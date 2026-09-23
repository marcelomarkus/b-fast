# ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

<p align="center">
  <a href="https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml"><img src="https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml/badge.svg" alt="CI"></a>
  <a href="https://marcelomarkus.github.io/b-fast/"><img src="https://github.com/marcelomarkus/b-fast/actions/workflows/docs.yml/badge.svg" alt="Documentation"></a>
  <a href="https://pypi.org/project/bfast-py/"><img src="https://img.shields.io/pypi/v/bfast-py?color=blue&logo=pypi&logoColor=white" alt="PyPI version"></a>
  <a href="https://pypi.org/project/bfast-py/"><img src="https://img.shields.io/pypi/pyversions/bfast-py?logo=python&logoColor=white" alt="Python versions"></a>
  <a href="https://www.npmjs.com/package/bfast-client"><img src="https://img.shields.io/npm/v/bfast-client?color=crimson&logo=npm&logoColor=white" alt="npm version"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://marcelomarkus.github.io/b-fast/performance/"><img src="https://img.shields.io/badge/Speedup-Up%20to%205.7x%20vs%20orjson-brightgreen?logo=speedtest&logoColor=white" alt="Performance"></a>
  <a href="https://app.codspeed.io/marcelomarkus/b-fast?utm_source=badge"><img src="https://img.shields.io/endpoint?url=https://codspeed.io/badge.json" alt="CodSpeed"/></a>
</p>

B-FAST is an ultra-high performance binary serialization protocol, developed in Rust for Python and TypeScript ecosystems. It's designed to replace JSON in critical routes where latency, CPU usage, and bandwidth are bottlenecks.

> "Performance is not just about speed—it's about efficiency where it matters most"

B-FAST was born from the recognition that modern applications need more than just fast serialization—they need **smart serialization** that adapts to real-world constraints. After extensive optimization, B-FAST has found its perfect niche in bandwidth-constrained environments, achieving **1.7x faster** than orjson for simple objects and **5.7x faster** on slow networks.

**Philosophy:** We believe that the future of data transfer lies not in raw CPU speed alone, but in intelligent protocols that minimize network overhead while maintaining excellent performance. B-FAST represents our contribution to a more efficient, bandwidth-conscious web.

## 🚀 Features & Architecture

<div class="grid cards" markdown>

-   __⚡ Extreme Performance__

    ---

    **2.01 ms** for 10k objects (**4.1x faster** than orjson, **6.0x vs JSON**). Up to **24x faster** primitive serialization via Rust fast-paths.

    [:octicons-arrow-right-24: View Benchmarks](performance.md)

-   __🌊 Real-Time Streaming & MCP__

    ---

    Continuous binary streaming (`application/x-bfast-stream`), chunked HTTP transfer, and Model Context Protocol (MCP) tool results.

    [:octicons-arrow-right-24: FastMCP & Streaming Guide](mcp.md)

-   __🐍 FastAPI Integration__

    ---

    Drop-in `BFastResponse` and `BFastStreamingResponse` supporting both synchronous and asynchronous generators (`AsyncGenerator`).

    [:octicons-arrow-right-24: Getting Started](getting_started.md)

-   __💻 TypeScript Client__

    ---

    High-performance client for browsers and Node.js (`bfast-client`) with streaming `decodeReadableStream` support.

    [:octicons-arrow-right-24: Frontend Guide](frontend.md)

-   __🧮 Zero-Copy NumPy & SIMD__

    ---

    Direct memory mapping for tensors and numerical arrays (**14-96x speedup**) with cache-aligned batch processing.

    [:octicons-arrow-right-24: Optimizations](optimizations.md)

-   __📚 API Reference__

    ---

    Complete reference for all classes, methods, parameters, and helper utilities in Python and TypeScript.

    [:octicons-arrow-right-24: API Reference](api.md)

</div>

## 📊 Performance

### 🚀 Simple Objects (10,000)
| Format | Time (ms) | Speedup |
|--------|-----------|---------|
| JSON | 12.0ms | 1.0x |
| orjson | 8.19ms | 1.5x |
| **B-FAST** | **2.01ms** | **🚀 6.0x** |

**B-FAST is 4.1x faster than orjson!**

### 🌊 Streaming Protocol (1,000 frames)
| Metric | Performance | Speedup / Throughput |
|--------|-------------|----------------------|
| **Streaming Decode (Aligned)** | **11.8ms** | **~85,000 frames/s** |
| **Streaming Decode (Fragmented)** | **13.6ms** | **~73,500 frames/s** |
| **Single Frame Latency** | **139.2µs** | **Real-time instant parsing** |
| **Sustained Stream Throughput** | **12,500 frames/s** | **High-frequency event feeds** |

### 🔄 Round-Trip (Encode + Network + Decode)

#### 📡 100 Mbps (Slow Network)
| Format | Total Time | Speedup vs orjson |
|--------|------------|-------------------|
| JSON | 114.5ms | 0.8x |
| orjson | 91.7ms | 1.0x |
| **B-FAST + LZ4** | **16.1ms** | **🚀 5.7x** |

#### 📡 1 Gbps (Fast Network)
| Format | Total Time | Speedup vs orjson |
|--------|------------|-------------------|
| JSON | 29.4ms | 0.5x |
| orjson | 15.3ms | 1.0x |
| **B-FAST + LZ4** | **7.2ms** | **🚀 2.1x** |

#### 📡 10 Gbps (Ultra-Fast Network)
| Format | Total Time | Speedup vs orjson |
|--------|------------|-------------------|
| JSON | 20.9ms | 0.4x |
| orjson | 7.7ms | 1.0x |
| **B-FAST + LZ4** | **6.3ms** | **🚀 1.2x** |

## 🎯 Ideal Use Cases

- **📱 Mobile/IoT**: 89% data savings + 5.7x performance on slow networks
- **🌐 APIs with slow networks**: Up to 5.7x faster than orjson
- **📊 Data pipelines**: 14-96x speedup for NumPy arrays
- **🗜️ Storage/Cache**: Superior integrated compression
- **🚀 Simple objects**: 4.1x faster than orjson
- **🌊 Real-time Streaming**: > 12,500 frames/s with zero-allocation chunk parsing

## 📦 Installation

### Backend (Python)
```bash
uv add bfast-py
```
or
```bash
pip install bfast-py
```

### Frontend (TypeScript)
```bash
npm install bfast-client
```

## 🛠️ Basic Usage

### Python
```python
import b_fast
from pydantic import BaseModel

class User(BaseModel):
    id: int
    name: str
    email: str

# Create encoder
bf = b_fast.BFast()

# Sample data
users = [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]

# Serialize
data = bf.encode_packed(users, compress=True)
print(f"Size: {len(data)} bytes")

# Deserialize
decoded = bf.decode_packed(data)
```

### TypeScript
```typescript
import { BFastDecoder } from 'bfast-client';

async function loadData() {
    const response = await fetch('/api/users');
    const buffer = await response.arrayBuffer();
    
    // Decode and decompress automatically
    const users = BFastDecoder.decode(buffer);
    console.log(users);
}
```

## 🔗 Useful Links

- [Getting Started](getting_started.md) - Complete tutorial
- [Frontend](frontend.md) - TypeScript integration
- [Performance](performance.md) - Detailed technical analysis
- [Troubleshooting](troubleshooting.md) - Troubleshooting guide

## 📄 License

Distributed under the MIT License. See [LICENSE](https://github.com/marcelomarkus/b-fast/blob/main/LICENSE) for more information.
