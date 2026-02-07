#!/usr/bin/env python3
"""B-FAST Performance Test - 10k Objects"""
import time
from pydantic import BaseModel
import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]


def test_10k_performance():
    print("🎯 B-FAST Performance Test: 10,000 Pydantic Objects")
    print("=" * 60)

    # Create test data
    users = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@test.com",
            active=i % 2 == 0,
            scores=[float(i), float(i * 2), float(i * 3)],
        )
        for i in range(10000)
    ]

    encoder = b_fast.BFast()

    # Warm up
    for _ in range(3):
        encoder.encode_packed(users, False)

    # Benchmark
    times = []
    for i in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users, False)
        end = time.perf_counter()
        times.append((end - start) * 1000)
        print(f"Run {i+1}: {times[-1]:.2f}ms")

    avg_time = sum(times) / len(times)
    print(f"\n📊 Average time: {avg_time:.2f}ms")
    print(f"📦 Payload size: {len(result):,} bytes")

    # Performance check
    target = 9.0
    if avg_time <= target:
        print(f"✅ Performance target achieved (≤ {target}ms)")
    else:
        print(f"⚠️  Target: {target}ms | Current: {avg_time:.2f}ms")


if __name__ == "__main__":
    test_10k_performance()
