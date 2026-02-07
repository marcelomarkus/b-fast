# 🚀 B-FAST Optimizations

## Overview

B-FAST includes several performance optimizations that make it ideal for bandwidth-constrained environments and high-throughput applications.

## Key Features

### Parallel Compression
For payloads larger than 1MB, B-FAST automatically uses parallel compression to minimize overhead:

```python
encoder = b_fast.BFast()

# Automatic parallel compression for large payloads
result = encoder.encode_packed(large_data, compress=True)
```

**Benefits:**
- Minimal compression overhead (~0.5ms for 1.6MB payload)
- Utilizes multiple CPU cores efficiently
- 75-99% payload reduction
- Ideal for slow networks (4x speedup on 100 Mbps)

### When to Use Compression

**Use `compress=True` for:**
- Mobile/IoT applications (data cost savings)
- Slow networks (< 100 Mbps)
- Storage/caching (space efficiency)
- Large payloads (> 100 KB)

**Use `compress=False` for:**
- Ultra-fast networks (> 10 Gbps)
- Small payloads (< 10 KB)
- CPU-constrained environments
- Latency-critical applications

### Encoder Reuse

Reusing the same encoder instance provides better performance for multiple serializations:

```python
encoder = b_fast.BFast()

# Reuse for multiple batches
for batch in data_batches:
    result = encoder.encode_packed(batch, compress=False)
```

**Benefits:**
- Optimized internal state
- Reduced memory allocations
- Better cache utilization
- Smaller payloads for repeated structures

## Performance Characteristics

### Compression Overhead

| Payload Size | Without Compression | With Compression | Overhead |
|--------------|---------------------|------------------|----------|
| 1,000 items | 0.36ms | 0.36ms | 0ms |
| 10,000 items | 3.92ms | 3.94ms | +0.02ms |
| 50,000 items | 27.33ms | 27.97ms | +0.64ms |

### Compression Ratio

| Payload Size | Original | Compressed | Reduction |
|--------------|----------|------------|-----------|
| 1,000 items | 32 KB | 214 bytes | 99.3% |
| 10,000 items | 320 KB | 1.3 KB | 99.6% |
| 50,000 items | 1.6 MB | 6.7 KB | 99.6% |

### Network Performance

**100 Mbps Network (Slow):**
- JSON: 114.3ms
- B-FAST + LZ4: 28.3ms
- **Speedup: 4.0x**

**1 Gbps Network (Fast):**
- JSON: 29.3ms
- B-FAST + LZ4: 10.2ms
- **Speedup: 2.9x**

## Best Practices

### 1. Choose the Right Format

```python
# For APIs and general transfer
result = encoder.encode_packed(data, compress=False)

# For mobile/IoT and slow networks
result = encoder.encode_packed(data, compress=True)
```

### 2. Reuse Encoders

```python
# ✅ Good - reuse encoder
encoder = b_fast.BFast()
for batch in batches:
    result = encoder.encode_packed(batch)

# ❌ Avoid - creating new encoder each time
for batch in batches:
    encoder = b_fast.BFast()  # Inefficient
    result = encoder.encode_packed(batch)
```

### 3. Benchmark Your Use Case

```python
import time

encoder = b_fast.BFast()

# Test without compression
start = time.perf_counter()
result1 = encoder.encode_packed(data, compress=False)
time1 = time.perf_counter() - start

# Test with compression
start = time.perf_counter()
result2 = encoder.encode_packed(data, compress=True)
time2 = time.perf_counter() - start

print(f"Without: {time1*1000:.2f}ms | {len(result1)} bytes")
print(f"With:    {time2*1000:.2f}ms | {len(result2)} bytes")
```

## Technical Details

B-FAST achieves its performance through:

- **Native Rust implementation** - No Python interpreter overhead
- **Direct memory access** - Reads Pydantic models without serialization
- **Zero-copy NumPy** - Arrays transferred at memory I/O speed
- **Efficient compression** - LZ4 with parallel processing
- **Cache optimization** - Aligned memory and batch processing

## Conclusion

B-FAST's optimizations make it ideal for:
- 📱 Mobile and IoT applications
- 🌐 Bandwidth-constrained networks
- 📊 Data-intensive pipelines
- 🗜️ Storage and caching systems

The automatic parallel compression and encoder reuse features ensure optimal performance without requiring manual tuning.
