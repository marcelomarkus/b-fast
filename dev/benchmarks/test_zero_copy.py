"""Benchmark: Zero-Copy vs Normal Serialization"""
import time
from pydantic import BaseModel
import b_fast

class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]

def benchmark():
    encoder = b_fast.BFast()
    
    sizes = [100, 1000, 10000]
    
    print("=" * 70)
    print("BENCHMARK: Zero-Copy Serialization (rkyv)")
    print("=" * 70)
    
    for size in sizes:
        users = [
            User(
                id=i,
                name=f"User {i}",
                email=f"user{i}@example.com",
                active=i % 2 == 0,
                scores=[float(i * j) for j in range(5)]
            )
            for i in range(size)
        ]
        
        # Warmup
        for _ in range(3):
            encoder.encode_packed(users, compress=False)
            b_fast.encode_zero_copy(users)
        
        # Benchmark Normal
        times_normal = []
        for _ in range(10):
            start = time.perf_counter()
            result_normal = encoder.encode_packed(users, compress=False)
            times_normal.append((time.perf_counter() - start) * 1000)
        
        # Benchmark Zero-Copy
        times_zero = []
        for _ in range(10):
            start = time.perf_counter()
            result_zero = b_fast.encode_zero_copy(users)
            times_zero.append((time.perf_counter() - start) * 1000)
        
        avg_normal = sum(times_normal) / len(times_normal)
        avg_zero = sum(times_zero) / len(times_zero)
        speedup = avg_normal / avg_zero if avg_zero > 0 else 0
        
        print(f"\n📦 {size:,} items:")
        print(f"  B-FAST normal:  {avg_normal:6.2f}ms | {len(result_normal):,} bytes")
        print(f"  Zero-copy:      {avg_zero:6.2f}ms | {len(result_zero):,} bytes")
        print(f"  Speedup:        {speedup:.2f}x")
    
    print("\n" + "=" * 70)
    print("✅ Zero-copy implementado com sucesso!")
    print("=" * 70)

if __name__ == "__main__":
    benchmark()
