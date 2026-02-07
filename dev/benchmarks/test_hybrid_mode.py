#!/usr/bin/env python3
"""
B-FAST Hybrid Mode Benchmark
Demonstrates automatic mode switching between simple and complex types
"""

import time
from datetime import date, datetime
from decimal import Decimal
from uuid import UUID

import orjson
from pydantic import BaseModel

import b_fast


# Simple model (triggers fast mode)
class SimpleUser(BaseModel):
    id: int
    name: str
    email: str
    age: int
    active: bool


# Complex model (triggers complex mode)
class ComplexUser(BaseModel):
    id: int
    name: str
    email: str
    created_at: datetime
    birth_date: date
    user_id: UUID
    balance: Decimal
    active: bool


def benchmark(data, label):
    encoder = b_fast.BFast()

    # B-FAST
    times = []
    for _ in range(100):
        start = time.perf_counter()
        encoder.encode_packed(data, compress=False)
        times.append((time.perf_counter() - start) * 1000)
    bfast_time = sum(times) / len(times)

    # orjson
    times = []
    for _ in range(100):
        start = time.perf_counter()
        orjson.dumps([d.model_dump(mode="json") for d in data])
        times.append((time.perf_counter() - start) * 1000)
    orjson_time = sum(times) / len(times)

    ratio = orjson_time / bfast_time
    winner = "🚀 B-FAST" if ratio > 1 else "orjson"

    print(f"\n{label}:")
    print(f"   B-FAST: {bfast_time:.2f}ms")
    print(f"   orjson: {orjson_time:.2f}ms")
    print(f"   {winner} is {abs(ratio):.2f}x faster")

    return bfast_time, orjson_time


print("=" * 70)
print("B-FAST Hybrid Mode Benchmark")
print("=" * 70)

# Test 1: Simple objects (10k)
simple_data = [
    SimpleUser(
        id=i,
        name=f"User{i}",
        email=f"user{i}@test.com",
        age=25 + (i % 50),
        active=i % 2 == 0,
    )
    for i in range(10000)
]

# Test 2: Complex objects (10k)
complex_data = [
    ComplexUser(
        id=i,
        name=f"User{i}",
        email=f"user{i}@test.com",
        created_at=datetime(2024, 1, 15, 10, 30, 45),
        birth_date=date(1990, 5, 20),
        user_id=UUID("550e8400-e29b-41d4-a716-446655440000"),
        balance=Decimal("1234.56"),
        active=i % 2 == 0,
    )
    for i in range(10000)
]

bfast_simple, orjson_simple = benchmark(simple_data, "📊 Simple Objects (10,000)")
bfast_complex, orjson_complex = benchmark(complex_data, "📊 Complex Objects (10,000)")

print("\n" + "=" * 70)
print("Summary:")
print("=" * 70)
print(
    f"✅ Simple mode:  B-FAST is {orjson_simple/bfast_simple:.2f}x faster than orjson"
)
print(f"✅ Complex mode: B-FAST is {bfast_complex:.2f}ms (preserves types)")
print("\n💡 B-FAST automatically detects and optimizes for both scenarios!")
print("=" * 70)
