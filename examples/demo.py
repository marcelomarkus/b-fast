"""B-FAST Format Comparison Demo"""

import time

from pydantic import BaseModel

import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]


# Create test data
users = [
    User(
        id=i,
        name=f"User {i}",
        email=f"user{i}@example.com",
        active=i % 2 == 0,
        scores=[float(i * j) for j in range(5)],
    )
    for i in range(10000)
]

print("=" * 70)
print("B-FAST: Format Comparison")
print("=" * 70)

encoder = b_fast.BFast()

# 1. Normal
start = time.perf_counter()
normal = encoder.encode_packed(users, compress=False)
time_normal = (time.perf_counter() - start) * 1000

# 2. Compressed (with automatic parallelism for large payloads)
start = time.perf_counter()
compressed = encoder.encode_packed(users, compress=True)
time_compressed = (time.perf_counter() - start) * 1000

print("\n📊 Results (10,000 Pydantic objects):\n")
print("1. B-FAST Normal:")
print(f"   Time:    {time_normal:6.2f}ms")
print(f"   Size:    {len(normal):,} bytes")
print("   Use:     REST APIs, general transfer")

print("\n2. B-FAST + LZ4 (Compressed):")
print(f"   Time:    {time_compressed:6.2f}ms")
print(
    f"   Size:    {len(compressed):,} bytes ({(1-len(compressed)/len(normal))*100:.1f}% smaller)"
)
print("   Use:     Slow networks, Mobile/IoT")
print("   Note:    Parallel compression active for payloads > 1MB")

print("\n" + "=" * 70)
print("✅ Optimizations implemented:")
print("   • Parallel LZ4 compression (Rayon)")
print("   • SIMD batch processing")
print("   • Cache-aligned memory allocation")
print("=" * 70)
