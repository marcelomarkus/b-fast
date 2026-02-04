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


def debug_pydantic_cache():
    print("🔍 Debug: Pydantic Cache Performance")
    print("=" * 50)

    # Test 1: Single object (should be fast)
    user = User(
        id=1, name="Test", email="test@test.com", active=True, scores=[1.0, 2.0, 3.0]
    )
    bf_encoder = b_fast.BFast()

    print("🧪 Single object test:")
    times = []
    for _i in range(1000):
        start = time.perf_counter()
        bf_encoder.encode_packed(user, False)
        end = time.perf_counter()
        times.append((end - start) * 1000000)  # microseconds

    avg_single = sum(times) / len(times)
    print(f"  Average: {avg_single:.1f}μs per object")

    # Test 2: List of 10 identical objects (should benefit from cache)
    users_10 = [user] * 10

    print("\n🧪 List of 10 identical objects:")
    times = []
    for _i in range(100):
        start = time.perf_counter()
        bf_encoder.encode_packed(users_10, False)
        end = time.perf_counter()
        times.append((end - start) * 1000)

    avg_10 = sum(times) / len(times)
    per_obj_10 = avg_10 / 10 * 1000  # microseconds per object
    print(f"  Average: {avg_10:.2f}ms total, {per_obj_10:.1f}μs per object")

    # Test 3: List of 100 different objects (no cache benefit)
    users_100_diff = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@test.com",
            active=i % 2 == 0,
            scores=[float(i), float(i * 2), float(i * 3)],
        )
        for i in range(100)
    ]

    print("\n🧪 List of 100 different objects:")
    times = []
    for _i in range(10):
        start = time.perf_counter()
        bf_encoder.encode_packed(users_100_diff, False)
        end = time.perf_counter()
        times.append((end - start) * 1000)

    avg_100_diff = sum(times) / len(times)
    per_obj_100_diff = avg_100_diff / 100 * 1000  # microseconds per object
    print(f"  Average: {avg_100_diff:.2f}ms total, {per_obj_100_diff:.1f}μs per object")

    # Test 4: List of 100 identical objects (should be MUCH faster due to cache)
    users_100_same = [user] * 100

    print("\n🧪 List of 100 identical objects (cache test):")
    times = []
    for _i in range(10):
        start = time.perf_counter()
        bf_encoder.encode_packed(users_100_same, False)
        end = time.perf_counter()
        times.append((end - start) * 1000)

    avg_100_same = sum(times) / len(times)
    per_obj_100_same = avg_100_same / 100 * 1000  # microseconds per object
    print(f"  Average: {avg_100_same:.2f}ms total, {per_obj_100_same:.1f}μs per object")

    # Analysis
    print("\n📊 Analysis:")
    print(f"  Single object: {avg_single:.1f}μs")
    print(f"  10 identical: {per_obj_10:.1f}μs per object")
    print(f"  100 different: {per_obj_100_diff:.1f}μs per object")
    print(f"  100 identical: {per_obj_100_same:.1f}μs per object")

    if per_obj_100_same < per_obj_100_diff * 0.8:
        print("  ✅ Cache is working! Identical objects are faster")
    else:
        print("  ❌ Cache not working effectively")

    # Calculate what we need for 9ms target
    target_per_obj = 9000 / 10000  # 0.9μs per object for 10k objects in 9ms
    print(f"\n🎯 Target: {target_per_obj:.1f}μs per object for 9ms/10k target")
    print(f"   Current best: {per_obj_100_same:.1f}μs per object")
    print(f"   Speedup needed: {per_obj_100_same / target_per_obj:.1f}x")


if __name__ == "__main__":
    debug_pydantic_cache()
