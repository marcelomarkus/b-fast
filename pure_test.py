#!/usr/bin/env python3
import time
import b_fast
from pydantic import BaseModel

class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]

def pure_performance_test():
    print("🚀 Pure B-FAST Performance Test (No Import Overhead)")
    print("=" * 60)
    
    # Create different sized datasets
    sizes = [100, 1000, 5000, 10000]
    
    for size in sizes:
        print(f"\n📊 Testing {size:,} Pydantic objects:")
        
        users = [
            User(id=i, name=f"User {i}", email=f"user{i}@test.com", 
                 active=i % 2 == 0, scores=[float(i), float(i*2), float(i*3)])
            for i in range(size)
        ]
        
        bf_encoder = b_fast.BFast()
        
        # Warm up (eliminate any lazy loading)
        for _ in range(3):
            bf_encoder.encode_packed(users, False)
        
        # Measure pure performance
        times = []
        for i in range(5):
            start = time.perf_counter()
            result = bf_encoder.encode_packed(users, False)
            end = time.perf_counter()
            elapsed = (end - start) * 1000
            times.append(elapsed)
            print(f"  Run {i+1}: {elapsed:.2f}ms")
        
        avg_time = sum(times) / len(times)
        per_object = avg_time / size * 1000  # microseconds per object
        
        print(f"  📈 Average: {avg_time:.2f}ms")
        print(f"  📦 Size: {len(result):,} bytes")
        print(f"  ⚡ Per object: {per_object:.1f}μs")
        
        # Check if we hit target for 10k
        if size == 10000:
            if avg_time <= 9.0:
                print(f"  🎉 TARGET ACHIEVED! ≤ 9ms")
            else:
                print(f"  ❌ Target missed by {avg_time - 9.0:.2f}ms")

if __name__ == "__main__":
    pure_performance_test()
