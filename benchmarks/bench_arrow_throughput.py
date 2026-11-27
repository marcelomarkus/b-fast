#!/usr/bin/env python3
"""
B-FAST Arrow C Data Interface Throughput Benchmark.
Measures serialization throughput and memory utilization when streaming from
Arrow C Data Interface compatible structures.
"""

import time
import tracemalloc
from typing import Any

import orjson
import polars as pl

from b_fast import BFast, encode_dataframe


def create_arrow_dataset(num_rows: int = 100_000) -> pl.DataFrame:
    """Generate a realistic data-lake analytics dataset."""
    return pl.DataFrame(
        {
            "id": list(range(num_rows)),
            "metric_a": [float(i) * 1.25 for i in range(num_rows)],
            "metric_b": [float(i % 500) * 0.05 for i in range(num_rows)],
            "category": ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]
            * (num_rows // 5),
            "region": [
                "us-east-1",
                "eu-central-1",
                "ap-southeast-1",
                "sa-east-1",
            ]
            * (num_rows // 4),
            "is_active": [True, False, True, True] * (num_rows // 4),
        }
    )


def run_benchmark(num_rows: int = 100_000, rounds: int = 5):
    print(f"\n{'=' * 85}")
    print(f"B-FAST Benchmark: Arrow C Data Interface ({num_rows:,} rows)")
    print(f"{'=' * 85}\n")

    df = create_arrow_dataset(num_rows)
    encoder = BFast()

    # Estimate uncompressed raw JSON payload size for accurate throughput (MB/s) calculation
    sample_json = orjson.dumps(df.head(1000).to_dicts())
    est_total_mb = (len(sample_json) / 1000 * num_rows) / (1024 * 1024)
    print(f"Dataset Shape: {df.shape[0]:,} rows × {df.shape[1]} columns")
    print(f"Equivalent JSON Size: ~{est_total_mb:.2f} MB\n")

    results: list[dict[str, Any]] = []

    # -------------------------------------------------------------
    # 1. Baseline: Polars df.to_dicts()
    # -------------------------------------------------------------
    tracemalloc.start()
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        _ = df.to_dicts()
        times.append((time.perf_counter() - t0) * 1000)
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    to_dicts_mem_mb = peak / (1024 * 1024)
    to_dicts_avg_ms = sum(times) / len(times)
    to_dicts_rps = num_rows / (to_dicts_avg_ms / 1000)

    results.append(
        {
            "name": "1. Polars df.to_dicts() (Raw list[dict] alloc)",
            "time_ms": to_dicts_avg_ms,
            "rps": to_dicts_rps,
            "mb_s": est_total_mb / (to_dicts_avg_ms / 1000),
            "mem_mb": to_dicts_mem_mb,
            "speedup": 1.0,
        }
    )

    # -------------------------------------------------------------
    # 2. orjson.dumps(df.to_dicts())
    # -------------------------------------------------------------
    tracemalloc.start()
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        _ = orjson.dumps(df.to_dicts())
        times.append((time.perf_counter() - t0) * 1000)
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    orjson_mem_mb = peak / (1024 * 1024)
    orjson_avg_ms = sum(times) / len(times)
    orjson_rps = num_rows / (orjson_avg_ms / 1000)

    results.append(
        {
            "name": "2. orjson.dumps(df.to_dicts())",
            "time_ms": orjson_avg_ms,
            "rps": orjson_rps,
            "mb_s": est_total_mb / (orjson_avg_ms / 1000),
            "mem_mb": orjson_mem_mb,
            "speedup": to_dicts_avg_ms / orjson_avg_ms,
        }
    )

    # -------------------------------------------------------------
    # 3. B-FAST Native Arrow C Stream (records orientation, uncompressed)
    # -------------------------------------------------------------
    tracemalloc.start()
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        _ = encoder.encode_packed(df, compress=False)
        times.append((time.perf_counter() - t0) * 1000)
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    bfast_rec_mem_mb = peak / (1024 * 1024)
    bfast_rec_avg_ms = sum(times) / len(times)
    bfast_rec_rps = num_rows / (bfast_rec_avg_ms / 1000)

    results.append(
        {
            "name": "3. B-FAST Zero-Copy Arrow (orient='records', uncompressed)",
            "time_ms": bfast_rec_avg_ms,
            "rps": bfast_rec_rps,
            "mb_s": est_total_mb / (bfast_rec_avg_ms / 1000),
            "mem_mb": bfast_rec_mem_mb,
            "speedup": to_dicts_avg_ms / bfast_rec_avg_ms,
        }
    )

    # -------------------------------------------------------------
    # 4. B-FAST Native Arrow C Stream (records orientation, LZ4 compressed)
    # -------------------------------------------------------------
    tracemalloc.start()
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        _ = encoder.encode_packed(df, compress=True)
        times.append((time.perf_counter() - t0) * 1000)
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    bfast_lz4_mem_mb = peak / (1024 * 1024)
    bfast_lz4_avg_ms = sum(times) / len(times)
    bfast_lz4_rps = num_rows / (bfast_lz4_avg_ms / 1000)

    results.append(
        {
            "name": "4. B-FAST Zero-Copy Arrow (orient='records', LZ4 compressed)",
            "time_ms": bfast_lz4_avg_ms,
            "rps": bfast_lz4_rps,
            "mb_s": est_total_mb / (bfast_lz4_avg_ms / 1000),
            "mem_mb": bfast_lz4_mem_mb,
            "speedup": to_dicts_avg_ms / bfast_lz4_avg_ms,
        }
    )

    # -------------------------------------------------------------
    # 5. B-FAST Native Arrow C Stream (columnar orientation, uncompressed)
    # -------------------------------------------------------------
    tracemalloc.start()
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        _ = encode_dataframe(df, orient="columns", compress=False)
        times.append((time.perf_counter() - t0) * 1000)
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    bfast_col_mem_mb = peak / (1024 * 1024)
    bfast_col_avg_ms = sum(times) / len(times)
    bfast_col_rps = num_rows / (bfast_col_avg_ms / 1000)

    results.append(
        {
            "name": "5. B-FAST Zero-Copy Arrow (orient='columns', uncompressed)",
            "time_ms": bfast_col_avg_ms,
            "rps": bfast_col_rps,
            "mb_s": est_total_mb / (bfast_col_avg_ms / 1000),
            "mem_mb": bfast_col_mem_mb,
            "speedup": to_dicts_avg_ms / bfast_col_avg_ms,
        }
    )

    # Print Table
    print(
        f"{'Strategy':<55} | {'Latency':<9} | {'Throughput':<15} | {'MB/s':<9} | {'RAM Peak':<9} | {'Speedup'}"
    )
    print("-" * 115)
    for r in results:
        print(
            f"{r['name']:<55} | {r['time_ms']:>6.2f} ms | {r['rps']:>12,.0f} r/s | {r['mb_s']:>7.1f} | {r['mem_mb']:>6.2f} MB | {r['speedup']:>5.1f}x"
        )
    print("-" * 115)

    speedup_vs_orjson = orjson_avg_ms / bfast_col_avg_ms
    mem_saved = to_dicts_mem_mb - bfast_col_mem_mb
    print(f"\nSummary: B-FAST Arrow Columnar is {speedup_vs_orjson:.1f}x vs orjson")
    print(f"Memory Delta: {mem_saved:.1f} MB lower peak allocation")


if __name__ == "__main__":
    run_benchmark(num_rows=100_000, rounds=5)
    run_benchmark(num_rows=500_000, rounds=3)
