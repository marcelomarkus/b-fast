"""B-FAST Compression Benchmark"""

import time

from pydantic import BaseModel

import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]
    description: str


def benchmark_compression():
    print("=" * 70)
    print("B-FAST Compression Benchmark")
    print("=" * 70)

    # Small payload (1,000 items)
    print("\n📦 Small Payload (1,000 items)")
    print("-" * 70)
    users_small = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
            description=f"Description {i}",
        )
        for i in range(1000)
    ]

    encoder = b_fast.BFast()

    # Warmup
    for _ in range(3):
        encoder.encode_packed(users_small, compress=False)
        encoder.encode_packed(users_small, compress=True)

    # Without compression
    times = []
    for _ in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users_small, compress=False)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"Without compression:  {sum(times)/len(times):6.2f}ms  |  {len(result):,} bytes"
    )

    # With compression
    times = []
    for _ in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users_small, compress=True)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"With compression:     {sum(times)/len(times):6.2f}ms  |  {len(result):,} bytes"
    )

    # Medium payload (10,000 items)
    print("\n📦 Medium Payload (10,000 items)")
    print("-" * 70)
    users_medium = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
            description=f"Description {i}",
        )
        for i in range(10000)
    ]

    # Warmup
    for _ in range(3):
        encoder.encode_packed(users_medium, compress=False)
        encoder.encode_packed(users_medium, compress=True)

    # Without compression
    times = []
    for _ in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users_medium, compress=False)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"Without compression:  {sum(times)/len(times):6.2f}ms  |  {len(result):,} bytes"
    )

    # With compression
    times = []
    for _ in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users_medium, compress=True)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"With compression:     {sum(times)/len(times):6.2f}ms  |  {len(result):,} bytes"
    )

    # Large payload (50,000 items)
    print("\n📦 Large Payload (50,000 items)")
    print("-" * 70)
    users_large = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(10)],
            description=f"This is a longer description for user {i} " * 3,
        )
        for i in range(50000)
    ]

    # Warmup
    for _ in range(2):
        encoder.encode_packed(users_large, compress=False)
        encoder.encode_packed(users_large, compress=True)

    # Without compression
    times = []
    for _ in range(5):
        start = time.perf_counter()
        result = encoder.encode_packed(users_large, compress=False)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"Without compression:  {sum(times)/len(times):7.2f}ms  |  {len(result):,} bytes"
    )

    # With compression
    times = []
    for _ in range(5):
        start = time.perf_counter()
        result = encoder.encode_packed(users_large, compress=True)
        times.append((time.perf_counter() - start) * 1000)
    print(
        f"With compression:     {sum(times)/len(times):7.2f}ms  |  {len(result):,} bytes"
    )

    print("\n" + "=" * 70)
    print("✅ Benchmark completed successfully")
    print("=" * 70)


if __name__ == "__main__":
    benchmark_compression()
