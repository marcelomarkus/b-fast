import time
import json
import orjson
import pickle
import b_fast
import numpy as np
from pydantic import BaseModel

def detailed_benchmark():
    print("🔍 B-FAST Detailed Performance Analysis")
    print("=" * 60)
    
    # Test 1: Simple data (no Pydantic overhead)
    simple_data = {
        "id": 123,
        "name": "test",
        "active": True,
        "score": 99.5,
        "tags": ["fast", "binary", "rust"]
    }
    
    # Test 2: NumPy only
    numpy_data = {
        "array": np.random.rand(1000),
        "metadata": {"size": 1000}
    }
    
    # Test 3: String interning advantage
    repeated_keys_data = [
        {"user_id": i, "user_name": f"user_{i}", "user_email": f"user_{i}@test.com"}
        for i in range(1000)
    ]
    
    # Test 4: Pydantic models
    class User(BaseModel):
        id: int
        name: str
        email: str
    
    pydantic_data = [User(id=i, name=f"user_{i}", email=f"user_{i}@test.com") for i in range(1000)]
    
    tests = [
        ("Simple Dict", simple_data),
        ("NumPy Array", numpy_data),
        ("Repeated Keys", repeated_keys_data),
        ("Pydantic Models", pydantic_data)
    ]
    
    bf_encoder = b_fast.BFast()
    iterations = 100
    
    for test_name, test_data in tests:
        print(f"\n🧪 {test_name}")
        print("-" * 40)
        
        # JSON baseline
        json_data = test_data
        if test_name == "Pydantic Models":
            json_data = [u.model_dump() for u in test_data]
        elif test_name == "NumPy Array":
            json_data = {"array": test_data["array"].tolist(), "metadata": test_data["metadata"]}
        
        # Benchmark JSON
        start = time.perf_counter()
        for _ in range(iterations):
            json.dumps(json_data).encode('utf-8')
        json_time = (time.perf_counter() - start) / iterations * 1000
        
        # Benchmark orjson
        start = time.perf_counter()
        for _ in range(iterations):
            orjson.dumps(json_data)
        orjson_time = (time.perf_counter() - start) / iterations * 1000
        
        # Benchmark B-FAST
        start = time.perf_counter()
        for _ in range(iterations):
            bf_encoder.encode_packed(test_data, compress=False)
        bfast_time = (time.perf_counter() - start) / iterations * 1000
        
        # Results
        print(f"   JSON:    {json_time:>6.2f}ms")
        print(f"   orjson:  {orjson_time:>6.2f}ms")
        print(f"   B-FAST:  {bfast_time:>6.2f}ms")
        print(f"   Speedup: {json_time/bfast_time:>6.1f}x vs JSON, {orjson_time/bfast_time:>6.1f}x vs orjson")
        
        # Size comparison
        json_size = len(json.dumps(json_data).encode('utf-8'))
        bfast_size = len(bf_encoder.encode_packed(test_data, compress=False))
        print(f"   Size:    JSON {json_size}b, B-FAST {bfast_size}b ({(1-bfast_size/json_size)*100:.1f}% smaller)")

def micro_benchmark():
    print("\n" + "=" * 60)
    print("🔬 MICRO-BENCHMARKS")
    print("=" * 60)
    
    bf_encoder = b_fast.BFast()
    iterations = 10000
    
    # Test individual operations
    micro_tests = [
        ("Integer", 42),
        ("String", "hello world"),
        ("Boolean", True),
        ("None", None),
        ("Small List", [1, 2, 3, 4, 5]),
        ("Small Dict", {"a": 1, "b": 2, "c": 3}),
    ]
    
    for test_name, test_data in micro_tests:
        # B-FAST
        start = time.perf_counter()
        for _ in range(iterations):
            bf_encoder.encode_packed(test_data, compress=False)
        bfast_time = (time.perf_counter() - start) / iterations * 1000000  # microseconds
        
        # JSON
        start = time.perf_counter()
        for _ in range(iterations):
            json.dumps(test_data).encode('utf-8')
        json_time = (time.perf_counter() - start) / iterations * 1000000  # microseconds
        
        print(f"{test_name:>12}: B-FAST {bfast_time:>6.1f}μs, JSON {json_time:>6.1f}μs, Ratio {json_time/bfast_time:>4.1f}x")

if __name__ == "__main__":
    detailed_benchmark()
    micro_benchmark()
