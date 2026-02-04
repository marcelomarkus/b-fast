#!/usr/bin/env python3
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
    print("🎯 Testing B-FAST with 10k Pydantic objects")
    print("=" * 50)

    # Create 10k users (same as main.py)
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

    bf_encoder = b_fast.BFast()

    # Warm up
    for _ in range(3):
        bf_encoder.encode_packed(users, False)

    # Benchmark
    times = []
    for i in range(10):
        start = time.perf_counter()
        result = bf_encoder.encode_packed(users, False)
        end = time.perf_counter()
        times.append((end - start) * 1000)
        print(f"Run {i+1}: {times[-1]:.2f}ms")

    avg_time = sum(times) / len(times)
    print(f"\n📊 Average time: {avg_time:.2f}ms")
    print(f"📦 Payload size: {len(result):,} bytes")

    # Target check
    if avg_time <= 9.0:
        print("🎉 TARGET ACHIEVED! ≤ 9ms")
    else:
        print(f"❌ Target missed. Need {9.0 - avg_time:.2f}ms improvement")


if __name__ == "__main__":
    test_10k_performance()
