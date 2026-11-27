#!/usr/bin/env python3
"""
⚡ B-FAST Disruptive Streaming Benchmark
Validates high-throughput streaming performance with concrete numbers:
- Unbuffered streaming (frame-by-frame) vs Adaptive Buffer Coalescing (64KB chunks)
- Standard JSON streaming vs B-FAST Binary Streaming
- Socket transport throughput (objects/sec and MB/s)
"""

import asyncio
import json
import socket
import threading
import time
from typing import Any

from pydantic import BaseModel

from b_fast.streaming import (
    BFastSocketStreamer,
    BFastStreamDecoder,
    BFastStreamEncoder,
)


class TelemetryEvent(BaseModel):
    id: int
    sensor: str
    temperature: float
    pressure: float
    status: str
    active: bool


def create_dataset(count: int = 10000) -> list[dict[str, Any]]:
    return [
        {
            "id": i,
            "sensor": f"sensor_node_{i % 50}",
            "temperature": 20.0 + (i % 100) * 0.15,
            "pressure": 1013.25 + (i % 20) * 0.5,
            "status": "nominal" if i % 10 != 0 else "warning",
            "active": True,
        }
        for i in range(count)
    ]


def bench_unbuffered_vs_coalesced(items: list[dict[str, Any]]):
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()
    count = len(items)

    print(
        f"\n📊 Benchmark 1: Frame-by-Frame vs Adaptive Buffer Coalescing ({count:,} records)"
    )
    print("-" * 80)

    # 1. Unbuffered (chunk_size=0: classic frame-by-frame)
    start = time.perf_counter()
    unbuf_chunks = list(encoder.encode_stream(items, compress=False, chunk_size=0))
    unbuf_encode_time = (time.perf_counter() - start) * 1000

    start = time.perf_counter()
    for chunk in unbuf_chunks:
        decoder.feed(chunk)
    unbuf_decode_time = (time.perf_counter() - start) * 1000
    decoder.reset()

    unbuf_total_bytes = sum(len(c) for c in unbuf_chunks)
    unbuf_chunks_count = len(unbuf_chunks)

    # 2. Coalesced (chunk_size=65536: 64KB blocks)
    start = time.perf_counter()
    coalesced_chunks = list(
        encoder.encode_stream(items, compress=False, chunk_size=65536)
    )
    coalesced_encode_time = (time.perf_counter() - start) * 1000

    start = time.perf_counter()
    for chunk in coalesced_chunks:
        decoder.feed(chunk)
    coalesced_decode_time = (time.perf_counter() - start) * 1000
    decoder.reset()

    coalesced_total_bytes = sum(len(c) for c in coalesced_chunks)
    coalesced_chunks_count = len(coalesced_chunks)

    # 3. JSON Baseline
    start = time.perf_counter()
    json_lines = [json.dumps(it).encode("utf-8") + b"\n" for it in items]
    json_encode_time = (time.perf_counter() - start) * 1000
    json_total_bytes = sum(len(c) for c in json_lines)

    start = time.perf_counter()
    for line in json_lines:
        json.loads(line)
    json_decode_time = (time.perf_counter() - start) * 1000

    # 4. orjson Baseline (Fastest C/Rust JSON library)
    try:
        import orjson

        start = time.perf_counter()
        orjson_lines = [orjson.dumps(it) + b"\n" for it in items]
        orjson_encode_time = (time.perf_counter() - start) * 1000
        orjson_total_bytes = sum(len(c) for c in orjson_lines)

        start = time.perf_counter()
        for line in orjson_lines:
            orjson.loads(line)
        orjson_decode_time = (time.perf_counter() - start) * 1000
        has_orjson = True
    except ImportError:
        has_orjson = False

    print(
        f"{'Metric':<28} | {'Std JSON':<12} | {'orjson (C)':<12} | {'B-FAST Unbuf':<14} | {'B-FAST Coalesced (64KB)':<24}"
    )
    print("-" * 102)
    orjson_chunks_str = f"{len(orjson_lines):,}" if has_orjson else "N/A"
    orjson_enc_str = f"{orjson_encode_time:.2f}" if has_orjson else "N/A"
    orjson_dec_str = f"{orjson_decode_time:.2f}" if has_orjson else "N/A"
    orjson_tot_str = (
        f"{(orjson_encode_time + orjson_decode_time):.2f}" if has_orjson else "N/A"
    )
    orjson_size_str = f"{orjson_total_bytes:,}" if has_orjson else "N/A"

    print(
        f"{'Chunks / ASGI Messages':<28} | {len(json_lines):<12,} | {orjson_chunks_str:<12} | {unbuf_chunks_count:<14,} | {coalesced_chunks_count:<24,}"
    )
    print(
        f"{'Encode Time (ms)':<28} | {json_encode_time:<12.2f} | {orjson_enc_str:<12} | {unbuf_encode_time:<14.2f} | {coalesced_encode_time:<24.2f}"
    )
    print(
        f"{'Decode Time (ms)':<28} | {json_decode_time:<12.2f} | {orjson_dec_str:<12} | {unbuf_decode_time:<14.2f} | {coalesced_decode_time:<24.2f}"
    )
    print(
        f"{'Total Pipeline (ms)':<28} | {(json_encode_time + json_decode_time):<12.2f} | {orjson_tot_str:<12} | {(unbuf_encode_time + unbuf_decode_time):<14.2f} | {(coalesced_encode_time + coalesced_decode_time):<24.2f}"
    )
    print(
        f"{'Payload Size (bytes)':<28} | {json_total_bytes:<12,} | {orjson_size_str:<12} | {unbuf_total_bytes:<14,} | {coalesced_total_bytes:<24,}"
    )

    message_reduction = unbuf_chunks_count / coalesced_chunks_count
    decode_speedup = unbuf_decode_time / coalesced_decode_time

    print("-" * 102)
    print(
        f"⚡ ASGI Message Churn Reduction: {message_reduction:.1f}x fewer event-loop yields ({unbuf_chunks_count:,} -> {coalesced_chunks_count})"
    )
    print(
        f"🚀 Decode Acceleration via Buffer Coalescing: {decode_speedup:.2f}x faster ({unbuf_decode_time:.2f}ms -> {coalesced_decode_time:.2f}ms)"
    )


def bench_socket_streaming_throughput(items: list[dict[str, Any]]):
    count = len(items)
    print(
        f"\n📊 Benchmark 2: Socket Transport Throughput ({count:,} records over TCP socketpair)"
    )
    print("-" * 75)

    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()
    received_items = []
    stop_flag = False

    def reader():
        while not stop_flag:
            try:
                s2.settimeout(0.05)
                chunk = s2.recv(131072)
                if not chunk:
                    break
                received_items.extend(decoder.feed(chunk))
            except TimeoutError:
                continue

    t = threading.Thread(target=reader)
    t.start()

    # Stream over native socket
    streamer = BFastSocketStreamer(s1, compress=True, native=True, buffer_size=65536)

    start = time.perf_counter()
    total_bytes = streamer.stream(items)
    stream_time = time.perf_counter() - start

    stop_flag = True
    t.join()
    streamer.close()
    s1.close()
    s2.close()

    throughput_ops = count / stream_time
    throughput_mb = (total_bytes / (1024 * 1024)) / stream_time

    print(f"  • Items Streamed & Decoded: {len(received_items):,} / {count:,}")
    print(f"  • Time to Wire:             {stream_time * 1000:.2f} ms")
    print(f"  • Throughput (Records/sec): {throughput_ops:,.0f} items/sec")
    print(
        f"  • Network Wire Speed:       {throughput_mb:.2f} MB/s (compressed payload)"
    )
    print(f"  • Total Bytes Transmitted:  {total_bytes:,} bytes")


async def bench_async_asgi_throughput(items: list[dict[str, Any]]):
    count = len(items)
    encoder = BFastStreamEncoder()

    async def item_generator():
        for item in items:
            yield item

    start = time.perf_counter()
    chunks = []
    async for chunk in encoder.encode_async_stream(
        item_generator(), compress=True, chunk_size=65536
    ):
        chunks.append(chunk)
    elapsed = time.perf_counter() - start

    total_bytes = sum(len(c) for c in chunks)
    print("\n📊 Benchmark 3: Async Generator Throughput (ASGI Coalesced Stream)")
    print("-" * 75)
    print(f"  • Async Records Processed:  {count:,}")
    print(
        f"  • Chunks Dispatched:        {len(chunks):,} (avg {total_bytes // max(1, len(chunks)):,} bytes/chunk)"
    )
    print(f"  • Elapsed Time:             {elapsed * 1000:.2f} ms")
    print(f"  • Processing Rate:          {count / elapsed:,.0f} items/sec")


def main():
    print("=" * 75)
    print("⚡ B-FAST DISRUPTIVE MULTIPLATFORM STREAMING PERFORMANCE REPORT")
    print("=" * 75)

    dataset_10k = create_dataset(10000)
    bench_unbuffered_vs_coalesced(dataset_10k)
    bench_socket_streaming_throughput(dataset_10k)
    asyncio.run(bench_async_asgi_throughput(dataset_10k))

    dataset_50k = create_dataset(50000)
    bench_unbuffered_vs_coalesced(dataset_50k)
    bench_socket_streaming_throughput(dataset_50k)

    print("\n" + "=" * 75)
    print("✅ Benchmark executed successfully with concrete verifiable metrics.")
    print("=" * 75)


if __name__ == "__main__":
    main()
