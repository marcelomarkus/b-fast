"""Benchmark: Paralelismo vs Serial"""
import time
from pydantic import BaseModel
import b_fast

class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]

def benchmark_parallel():
    encoder = b_fast.BFast()
    
    # Teste com diferentes tamanhos
    sizes = [100, 500, 1000, 5000, 10000]
    
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
        
        # Benchmark
        times = []
        for _ in range(10):
            start = time.perf_counter()
            result = encoder.encode_packed(users, compress=False)
            end = time.perf_counter()
            times.append((end - start) * 1000)
        
        avg_time = sum(times) / len(times)
        print(f"{size:5d} items: {avg_time:6.2f}ms (payload: {len(result):,} bytes)")

if __name__ == "__main__":
    print("🚀 Benchmark: Paralelismo em Thread Nativa")
    print("=" * 60)
    benchmark_parallel()
