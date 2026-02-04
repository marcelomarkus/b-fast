# ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

B-FAST is an ultra-high performance binary serialization protocol, developed in Rust for Python and TypeScript ecosystems. It's designed to replace JSON in critical routes where latency, CPU usage, and bandwidth are bottlenecks.

## 🚀 Why B-FAST?

- **Rust Engine:** Native serialization without Python interpreter overhead
- **Pydantic Native:** Reads Pydantic model attributes directly from memory, skipping the slow .model_dump() process
- **Zero-Copy NumPy:** Serializes tensors and numeric arrays directly, achieving maximum memory I/O speed
- **String Interning:** Repeated keys (like field names in object lists) are sent only once
- **Bit-Packing:** Small integers and booleans occupy only 4 bits within the type tag
- **Built-in LZ4:** Ultra-fast block compression for large payloads

## 📊 Performance

Comparison of serializing a list of 10,000 complex Pydantic models:

### 🚀 Serialization (Encode)
| Format | Time (ms) | Speedup | Payload Size | Reduction |
|--------|-----------|---------|--------------|-----------|
| JSON (Standard) | 9.64ms | 1.0x | 1.18 MB | 0% |
| orjson | 1.51ms | 6.4x | 1.06 MB | 10.2% |
| Pickle | 2.74ms | 3.5x | 808 KB | 31.6% |
| **B-FAST** | **4.51ms** | **2.1x** | **998 KB** | **15.5%** |
| **B-FAST + LZ4** | **5.21ms** | **1.9x** | **252 KB** | **78.7%** |

### 🔄 Round-Trip (Encode + Network + Decode)

#### 📡 100 Mbps (Slow Network)
| Format | Total Time | Speedup vs JSON |
|--------|------------|-----------------|
| JSON | 114.3ms | 1.0x |
| orjson | 92.3ms | 1.2x |
| **B-FAST + LZ4** | **28.3ms** | **🚀 4.0x** |

#### 📡 1 Gbps (Fast Network)
| Format | Total Time | Speedup vs JSON |
|--------|------------|-----------------|
| JSON | 29.3ms | 1.0x |
| orjson | 15.9ms | 1.8x |
| **B-FAST + LZ4** | **10.2ms** | **🚀 2.9x** |

## 🎯 Ideal Use Cases

- **📱 Mobile/IoT**: 78.7% data savings + 2.1x performance
- **🌐 APIs over slow networks**: Up to 4x faster than JSON
- **📊 Data pipelines**: 148x speedup for NumPy arrays
- **🗜️ Storage/Cache**: Superior integrated compression

## 📦 Installation

### Backend (Python)
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
