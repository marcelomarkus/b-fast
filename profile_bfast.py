#!/usr/bin/env python3
import time
import b_fast
from pydantic import BaseModel
import cProfile
import pstats
import io

class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]

def profile_bfast():
    print("🔍 Profiling B-FAST Performance")
    print("=" * 50)
    
    # Create test data
    users = [
        User(id=i, name=f"User {i}", email=f"user{i}@test.com", 
             active=i % 2 == 0, scores=[float(i), float(i*2), float(i*3)])
        for i in range(1000)  # Smaller dataset for detailed profiling
    ]
    
    bf_encoder = b_fast.BFast()
    
    # Profile the encoding
    pr = cProfile.Profile()
    pr.enable()
    
    # Run multiple times
    for _ in range(10):
        result = bf_encoder.encode_packed(users, False)
    
    pr.disable()
    
    # Print results
    s = io.StringIO()
    ps = pstats.Stats(pr, stream=s).sort_stats('cumulative')
    ps.print_stats(20)  # Top 20 functions
    
    print(s.getvalue())
    
    # Also test individual components
    print("\n🧪 Component Analysis:")
    
    # Test single user
    single_user = users[0]
    start = time.perf_counter()
    for _ in range(1000):
        bf_encoder.encode_packed(single_user, False)
    end = time.perf_counter()
    print(f"Single user (1000x): {(end-start)*1000:.2f}ms total, {(end-start):.3f}ms per call")
    
    # Test list of primitives
    primitives = [i for i in range(1000)]
    start = time.perf_counter()
    for _ in range(10):
        bf_encoder.encode_packed(primitives, False)
    end = time.perf_counter()
    print(f"1000 integers (10x): {(end-start)*1000:.2f}ms total, {(end-start)*100:.2f}ms per call")

if __name__ == "__main__":
    profile_bfast()
