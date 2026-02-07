# ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

B-FAST is an ultra-high performance binary serialization protocol, developed in Rust for Python and TypeScript ecosystems. It's designed to replace JSON in critical routes where latency, CPU usage, and bandwidth are bottlenecks.

> "Performance is not just about speed—it's about efficiency where it matters most"

B-FAST was born from the recognition that modern applications need more than just fast serialization—they need **smart serialization** that adapts to real-world constraints. After extensive optimization, B-FAST has found its perfect niche in bandwidth-constrained environments, achieving **1.7x faster** than orjson for simple objects and **5.7x faster** on slow networks.

**Philosophy:** We believe that the future of data transfer lies not in raw CPU speed alone, but in intelligent protocols that minimize network overhead while maintaining excellent performance. B-FAST represents our contribution to a more efficient, bandwidth-conscious web.

## 🚀 Why B-FAST?

- **Rust Engine:** Native serialization without Python interpreter overhead
- **Pydantic Native:** Reads Pydantic model attributes directly from memory, skipping the slow .model_dump() process
- **Zero-Copy NumPy:** Serializes tensors and numeric arrays directly, achieving 14-96x speedup vs JSON/orjson
- **Parallel Compression:** LZ4 with multi-thread processing for large payloads (>1MB)
- **Cache Optimized:** Aligned allocation and batch processing for maximum efficiency

## 📊 Performance

### 🚀 Simple Objects (10,000)
| Format | Time (ms) | Speedup |
|--------|-----------|---------|
| JSON | 12.0ms | 1.0x |
| orjson | 8.19ms | 1.5x |
| **B-FAST** | **4.83ms** | **🚀 2.5x** |

**B-FAST is 1.7x faster than orjson!**

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
- **🚀 Simple objects**: 1.7x faster than orjson
- **🗜️ Storage/Cache**: Superior integrated compression

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
