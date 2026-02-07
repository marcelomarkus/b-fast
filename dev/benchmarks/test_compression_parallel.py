"""Benchmark: Compressão Paralela em Payloads Grandes"""

import time

from pydantic import BaseModel

import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]
    description: str  # Campo extra para aumentar payload


def benchmark_large_payload():
    encoder = b_fast.BFast()

    # Teste com payloads grandes
    sizes = [10_000, 50_000, 100_000]

    for size in sizes:
        users = [
            User(
                id=i,
                name=f"User {i}",
                email=f"user{i}@example.com",
                active=i % 2 == 0,
                scores=[float(i * j) for j in range(10)],
                description=f"This is a long description for user {i} " * 5,
            )
            for i in range(size)
        ]

        # Warmup
        for _ in range(2):
            encoder.encode_packed(users, compress=True)

        # Benchmark
        times = []
        for _ in range(5):
            start = time.perf_counter()
            result = encoder.encode_packed(users, compress=True)
            end = time.perf_counter()
            times.append((end - start) * 1000)

        avg_time = sum(times) / len(times)
        print(f"{size:6d} items: {avg_time:7.2f}ms (compressed: {len(result):,} bytes)")


if __name__ == "__main__":
    print("🚀 Benchmark: Compressão Paralela LZ4")
    print("=" * 60)
    benchmark_large_payload()
