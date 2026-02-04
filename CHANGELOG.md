# Changelog

All notable changes to this project will be documented in this file.

## [1.0.6] - 2026-02-04

### 🚀 Performance Improvements
- **MAJOR**: Achieved 2.3x speedup vs JSON for Pydantic serialization (4.41ms for 10k objects)
- **MAJOR**: 78.7% payload reduction with built-in LZ4 compression (252KB vs 1.18MB)
- **MAJOR**: 4.0x faster than JSON in round-trip tests on 100 Mbps networks
- **MAJOR**: 148x speedup for NumPy arrays vs JSON

### ✨ New Features
- Round-trip performance testing with network simulation
- Real-world benchmark scenarios (100 Mbps, 1 Gbps, 10 Gbps)
- Comprehensive performance documentation
- Organized benchmark suite in `/benchmarks` folder

### 🔧 Technical Optimizations
- SIMD batch processing for homogeneous data structures
- Cache-aligned memory operations (64-byte alignment)
- Branch prediction hints for common data types
- Unrolled loops for 5-field Pydantic objects (User model)
- Hash-based string ID caching (64-entry cache)
- Direct memory access with unsafe operations
- Zero-copy NumPy array serialization
- Page-aligned memory allocation for better performance

### 📊 Benchmark Results
- **Serialization**: 4.41ms (2.3x faster than JSON)
- **With Compression**: 5.11ms (2.0x faster than JSON, 78.7% smaller)
- **NumPy Arrays**: 148x faster than JSON, 11x faster than orjson
- **Round-trip (100 Mbps)**: 28.3ms vs 114.3ms JSON (4.0x faster)
- **Round-trip (1 Gbps)**: 10.2ms vs 29.3ms JSON (2.9x faster)
- **Round-trip (10 Gbps)**: 8.4ms vs 8.3ms orjson (competitive)

### 🎯 Use Cases Clarified
- Mobile and IoT applications (bandwidth-constrained)
- Data pipelines with NumPy arrays (148x speedup)
- Storage and caching systems (78.7% size reduction)
- APIs serving over slower networks (4x improvement)

### 📚 Documentation
- Added comprehensive performance analysis (`docs/performance.md`)
- Added troubleshooting guide (`docs/troubleshooting.md`)
- Updated README with real benchmark results
- Organized benchmark files in dedicated folder

## [1.0.5] - 2026-02-03

### 📚 Documentation
- Added comprehensive documentation site
- Updated README with installation and usage examples
- Added TypeScript client documentation

## [1.0.4] - 2026-02-02

### 🔧 Initial Implementation
- Core B-FAST binary protocol implementation
- Rust backend with PyO3 bindings
- TypeScript client library
- Basic compression support
- Pydantic native serialization
