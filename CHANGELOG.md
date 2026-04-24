# Changelog

All notable changes to this project will be documented in this file.

## [1.2.1] - 2026-04-24

### 🚀 Features
- **Built-in FastAPI Response**: Added `BFastResponse` directly to the `b_fast` package for easier integration.
- **Improved Type Support**: Better handling of `BFastError` and updated `BFastResponse` to use built-in encoder.

### 🔧 Fixes
- **Module Visibility**: Fixed `ModuleNotFoundError`/`ImportError` when using the library in external projects by correctly exporting `BFast` in `__init__.py`.
- **Typings Organization**: Moved and renamed `.pyi` file to `python/b_fast/_b_fast.pyi` to ensure correct recognition by IDEs and type checkers after installation.
- **FastAPI Dependency**: Updated `README` to recommend installation with `[fastapi]` extra for projects using FastAPI.

### 📚 Documentation
- Updated `README` with simplified FastAPI usage example.
- Fixed installation instructions for better clarity.

## [1.1.0] - 2026-02-07

### 🚀 Features
- **Extended Type Support**: datetime, date, time, UUID, Decimal, Enum, tuple, set, frozenset, bytes, bytearray
- **Type Preservation**: Special tags (0xD1-0xD5) for native type preservation
- **Parallel LZ4 Compression**: Multi-threaded compression for payloads >1MB
- **Custom Allocator**: Allocator for optimized memory management

### ⚡ Performance
- **1.7x faster** than orjson for simple objects (4.83ms vs 8.19ms)
- **5.7x faster** than orjson on 100 Mbps networks (16.1ms vs 91.7ms round-trip)
- **14x faster** than orjson for NumPy arrays
- **89% payload reduction** with LZ4 compression
- Memory allocation improvements and buffer pre-allocation
- Optimized type checking and recursion depth protection

### 🧪 Testing
- Added 25 comprehensive tests (100% passing)
- Integration tests for type preservation
- Python → TypeScript type preservation tests
- Benchmarks for all extended types

### 📚 Documentation
- Translated README to English
- Added project philosophy and messaging
- Updated all benchmarks with latest results
- Added "Recommended" badge to FastAPI integration
- Added `uv` installation option
- Fixed multilingual docs structure

### 🔧 Technical
- TypeScript client updated with type decoders
- Code formatted with black and ruff
- Improved error handling and validation

## [1.0.7] - 2026-02-04

### 📚 Documentation
- **MAJOR**: Added multilingual documentation (English + Portuguese)
- Created comprehensive Portuguese documentation with all sections
- Added language selector similar to FastAPI docs
- Improved English documentation with updated benchmarks
- Added build script for multilingual documentation deployment

### 🌐 Internationalization
- Complete Portuguese translation of all documentation pages
- Organized documentation structure: `/` (English) and `/pt/` (Portuguese)
- Language switcher in navigation header
- Localized navigation menus and content

### 📁 Documentation Structure
- `docs/` - English documentation (default)
- `docs/pt/` - Portuguese documentation
- `build_docs.sh` - Automated build script for both languages
- Updated MkDocs configuration for better navigation

### 🔧 Technical Improvements
- Enhanced MkDocs theme with better navigation features
- Added proper language metadata for SEO
- Improved documentation build process
- Better organization of multilingual content

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
