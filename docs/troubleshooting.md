# 🔧 Troubleshooting

## Common Issues

### Installation Problems

#### "No module named 'b_fast'"
```bash
# Make sure you installed the package
pip install bfast-py

# Or with uv
uv add bfast-py
```

#### Compilation Errors
```bash
# Update pip and try again
pip install --upgrade pip setuptools wheel
pip install bfast-py --force-reinstall
```

### Performance Issues

#### Slower than Expected
1. **Check data structure**: B-FAST excels with Pydantic objects and NumPy arrays
2. **Enable compression**: Use `compress=True` for large payloads
3. **Batch processing**: Process lists of similar objects for best performance

#### Memory Usage
```python
# Reuse encoder instance
encoder = b_fast.BFast()
for batch in data_batches:
    result = encoder.encode_packed(batch, compress=True)
```

### Compatibility Issues

#### Unsupported Data Types
B-FAST currently supports:
- ✅ Pydantic models
- ✅ Basic Python types (int, str, bool, float, None)
- ✅ Lists and dictionaries
- ✅ NumPy arrays (float64)
- ❌ Custom classes (without Pydantic)
- ❌ Complex numbers
- ❌ Datetime objects (convert to timestamp first)

#### TypeScript Client Issues
```typescript
// Make sure to handle binary data correctly
const response = await fetch('/api/data');
const buffer = await response.arrayBuffer();
const data = BFastDecoder.decode(buffer);
```

## Performance Optimization

### When B-FAST is Optimal
- **Network bandwidth is limited** (mobile, IoT)
- **Large datasets with repeated structure** (lists of Pydantic objects)
- **NumPy arrays** (148x speedup vs JSON)
- **Storage efficiency matters** (79% size reduction)

### When to Consider Alternatives
- **Ultra-fast networks** (10+ Gbps internal)
- **Simple data structures** (single values, small objects)
- **CPU-constrained environments**

### Optimization Tips
```python
# 1. Use compression for large payloads
result = encoder.encode_packed(data, compress=True)

# 2. Batch similar objects together
users = [User(...) for _ in range(1000)]  # Good
mixed = [user1, "string", 123, dict()]    # Less optimal

# 3. Reuse encoder instances
encoder = b_fast.BFast()  # Create once
for batch in batches:
    encoder.encode_packed(batch)  # Reuse
```

## Getting Help

### Debug Information
```python
import b_fast
print(f"B-FAST version: {b_fast.__version__}")

# Test basic functionality
encoder = b_fast.BFast()
test_data = {"test": 123}
result = encoder.encode_packed(test_data, False)
print(f"Encoded {len(result)} bytes")
```

### Reporting Issues
When reporting issues, please include:
1. **Python version** and operating system
2. **B-FAST version** (`pip show bfast-py`)
3. **Sample data structure** that causes the issue
4. **Error message** (full traceback)
5. **Expected vs actual behavior**

### Community Support
- **GitHub Issues**: [Report bugs and feature requests](https://github.com/marcelomarkus/b-fast/issues)
- **Discussions**: [Ask questions and share use cases](https://github.com/marcelomarkus/b-fast/discussions)
- **Documentation**: [Complete documentation](https://marcelomarkus.github.io/b-fast/)

## FAQ

### Q: Why is B-FAST slower than orjson for simple data?
A: B-FAST is optimized for bandwidth-constrained scenarios and complex data structures. For simple data on fast networks, orjson may be faster.

### Q: Can I use B-FAST with Django/Flask?
A: Yes! B-FAST works with any Python web framework. Create a custom response class that uses B-FAST encoding.

### Q: Is there a decoder for Python?
A: The decoder is currently in development. The TypeScript client includes a full decoder implementation.

### Q: How does compression work?
A: B-FAST uses built-in LZ4 compression which is extremely fast (0.32ms decompress for 252KB). No external dependencies required.
