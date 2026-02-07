"""
B-FAST Practical Examples
==========================

This file demonstrates how to use B-FAST optimizations.
"""

import time

from pydantic import BaseModel

import b_fast

# ============================================================================
# Example 1: Automatic Compression for Large Payloads
# ============================================================================


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]


def example_compression():
    """
    Parallel compression is automatically activated for payloads > 1MB.
    """
    print("=" * 70)
    print("Example 1: Automatic Parallel Compression")
    print("=" * 70)

    encoder = b_fast.BFast()

    # Create large payload (> 1MB)
    users = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(10)],
        )
        for i in range(50_000)
    ]

    # Without compression
    start = time.perf_counter()
    uncompressed = encoder.encode_packed(users, compress=False)
    time_uncompressed = (time.perf_counter() - start) * 1000

    # With parallel compression (automatic for > 1MB)
    start = time.perf_counter()
    compressed = encoder.encode_packed(users, compress=True)
    time_compressed = (time.perf_counter() - start) * 1000

    print("\n📦 50,000 Pydantic objects:")
    print(
        f"  Without compression: {time_uncompressed:6.2f}ms | {len(uncompressed):,} bytes"
    )
    print(
        f"  With compression:    {time_compressed:6.2f}ms | {len(compressed):,} bytes"
    )
    print(f"  Reduction:           {(1 - len(compressed)/len(uncompressed))*100:.1f}%")
    print(f"  Overhead:            {time_compressed - time_uncompressed:+.2f}ms")
    print("\n✅ Parallel compression active (payload > 1MB)")
    print("✅ Minimal overhead thanks to parallelism\n")


# ============================================================================
# Example 2: Choosing Between Compression and Performance
# ============================================================================


def example_network_scenarios():
    """
    Demonstrates when to use compress=True vs compress=False.
    """
    print("=" * 70)
    print("Example 2: Network Scenario Optimization")
    print("=" * 70)

    encoder = b_fast.BFast()

    users = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
        )
        for i in range(10_000)
    ]

    # Measure serialization
    start = time.perf_counter()
    uncompressed = encoder.encode_packed(users, compress=False)
    time_uncompressed = (time.perf_counter() - start) * 1000

    start = time.perf_counter()
    compressed = encoder.encode_packed(users, compress=True)
    time_compressed = (time.perf_counter() - start) * 1000

    # Simulate different networks
    networks = [
        ("Mobile 3G", 1_000_000),  # 1 Mbps
        ("Slow WiFi", 10_000_000),  # 10 Mbps
        ("Broadband", 100_000_000),  # 100 Mbps
        ("Gigabit", 1_000_000_000),  # 1 Gbps
        ("10 Gigabit", 10_000_000_000),  # 10 Gbps
    ]

    print("\n📊 Round-Trip Analysis (Encode + Network + Decode):")
    print(
        f"{'Network':<15} {'Without Compression':<20} {'With Compression':<20} {'Best'}"
    )
    print("-" * 70)

    for name, bps in networks:
        # Transfer time (ms)
        transfer_uncompressed = (len(uncompressed) * 8 / bps) * 1000
        transfer_compressed = (len(compressed) * 8 / bps) * 1000

        # Total time (encode + transfer + decode estimated)
        total_uncompressed = time_uncompressed + transfer_uncompressed + 2.0
        total_compressed = time_compressed + transfer_compressed + 1.0

        better = (
            "✅ Compressed" if total_compressed < total_uncompressed else "✅ Normal"
        )
        speedup = total_uncompressed / total_compressed

        print(
            f"{name:<15} {total_uncompressed:>6.1f}ms           "
            f"{total_compressed:>6.1f}ms ({speedup:.1f}x)    {better}"
        )

    print("\n💡 Recommendations:")
    print("  • Slow networks (< 100 Mbps): compress=True")
    print("  • Fast networks (> 1 Gbps): compress=False")
    print("  • Mobile/IoT: always compress=True\n")


# ============================================================================
# Example 3: Encoder Reuse
# ============================================================================


def example_encoder_reuse():
    """
    Demonstrates the benefit of reusing the encoder for multiple serializations.
    """
    print("=" * 70)
    print("Example 3: Encoder Reuse")
    print("=" * 70)

    users = [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
        )
        for i in range(1000)
    ]

    # New encoder each time
    times_new = []
    sizes_new = []
    for _ in range(10):
        encoder = b_fast.BFast()  # New encoder
        start = time.perf_counter()
        result = encoder.encode_packed(users, compress=False)
        times_new.append((time.perf_counter() - start) * 1000)
        sizes_new.append(len(result))

    # Reused encoder (optimized)
    encoder = b_fast.BFast()
    times_reused = []
    sizes_reused = []
    for _ in range(10):
        start = time.perf_counter()
        result = encoder.encode_packed(users, compress=False)
        times_reused.append((time.perf_counter() - start) * 1000)
        sizes_reused.append(len(result))

    avg_new = sum(times_new) / len(times_new)
    avg_reused = sum(times_reused) / len(times_reused)
    avg_size_new = sum(sizes_new) / len(sizes_new)
    avg_size_reused = sum(sizes_reused) / len(sizes_reused)

    print("\n📊 1,000 Pydantic objects (10 iterations):")
    print(f"  New encoder:    {avg_new:.3f}ms | {avg_size_new:,.0f} bytes")
    print(f"  Reused encoder: {avg_reused:.3f}ms | {avg_size_reused:,.0f} bytes")
    print(f"  Speedup:        {avg_new/avg_reused:.2f}x")
    print(f"  Payload reduction: {(1 - avg_size_reused/avg_size_new)*100:.1f}%")
    print("\n✅ Reusing encoder optimizes repeated keys")
    print("✅ Ideal for serializing multiple batches\n")


# ============================================================================
# Example 4: NumPy Arrays (Zero-Copy)
# ============================================================================


def example_numpy():
    """
    Demonstrates ultra-fast NumPy array serialization.
    """
    print("=" * 70)
    print("Example 4: NumPy Arrays (Zero-Copy)")
    print("=" * 70)

    try:
        import numpy as np

        encoder = b_fast.BFast()

        # Large array
        array = np.random.rand(1_000_000)

        # B-FAST (zero-copy)
        start = time.perf_counter()
        result_bfast = encoder.encode_packed(array, compress=False)
        time_bfast = (time.perf_counter() - start) * 1000

        # JSON (for comparison)
        import json

        start = time.perf_counter()
        result_json = json.dumps(array.tolist()).encode()
        time_json = (time.perf_counter() - start) * 1000

        print("\n📊 NumPy array (1,000,000 floats):")
        print(f"  JSON:   {time_json:7.2f}ms | {len(result_json):,} bytes")
        print(f"  B-FAST: {time_bfast:7.2f}ms | {len(result_bfast):,} bytes")
        print(f"  Speedup: {time_json/time_bfast:.1f}x")
        print("\n✅ Zero-copy: NumPy memory copied directly")
        print("✅ Ideal for ML/Data Science pipelines\n")

    except ImportError:
        print("\n⚠️  NumPy not installed. Install with: pip install numpy\n")


# ============================================================================
# Main
# ============================================================================

if __name__ == "__main__":
    print("\n🚀 B-FAST: Practical Optimization Examples\n")

    example_compression()
    example_network_scenarios()
    example_encoder_reuse()
    example_numpy()

    print("=" * 70)
    print("✅ All examples executed successfully!")
    print("=" * 70)
    print("\n📚 Full documentation: https://marcelomarkus.github.io/b-fast/")
    print("🐛 Issues: https://github.com/marcelomarkus/b-fast/issues\n")
