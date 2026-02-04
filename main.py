import json
import pickle
import time

import orjson
from pydantic import BaseModel

import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]


def benchmark_serializers():
    print("🚀 B-FAST Performance Comparison")
    print("=" * 50)

    # Dados de teste - APENAS users para comparação justa
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

    # Para JSON/orjson (precisa converter)
    json_data = [u.model_dump() for u in users]

    # Configurar encoders
    bf_encoder = b_fast.BFast()

    # Benchmark functions
    def test_json():
        return json.dumps(json_data).encode("utf-8")

    def test_orjson():
        return orjson.dumps(json_data)

    def test_pickle():
        return pickle.dumps(json_data)

    def test_bfast():
        return bf_encoder.encode_packed(users, compress=False)

    def test_bfast_compressed():
        return bf_encoder.encode_packed(users, compress=True)

    # Executar benchmarks com menos iterações (dados maiores)
    iterations = 10

    tests = [
        ("JSON (stdlib)", test_json),
        ("orjson", test_orjson),
        ("Pickle", test_pickle),
        ("B-FAST", test_bfast),
        ("B-FAST + LZ4", test_bfast_compressed),
    ]

    results = []

    for name, test_func in tests:
        print(f"\n🧪 Testing {name}...")

        # Warmup
        for _ in range(5):
            test_func()

        # Benchmark
        start_time = time.perf_counter()
        payloads = []

        for _ in range(iterations):
            payload = test_func()
            payloads.append(payload)

        end_time = time.perf_counter()

        avg_time = (end_time - start_time) / iterations * 1000  # ms
        avg_size = sum(len(p) for p in payloads) / len(payloads)  # bytes

        results.append((name, avg_time, avg_size))

        print(f"   ⏱️  Tempo médio: {avg_time:.2f}ms")
        print(f"   📦 Tamanho médio: {avg_size:.0f} bytes")

    # Resultados finais
    print("\n" + "=" * 50)
    print("📊 RESULTADOS FINAIS")
    print("=" * 50)

    # Encontrar baseline (JSON)
    json_time = results[0][1]
    json_size = results[0][2]

    print(
        f"{'Método':<15} {'Tempo (ms)':<12} {'Speedup':<10} {'Tamanho':<12} {'Redução':<10}"
    )
    print("-" * 65)

    for name, avg_time, avg_size in results:
        speedup = json_time / avg_time
        size_reduction = (1 - avg_size / json_size) * 100

        print(
            f"{name:<15} {avg_time:>8.2f}ms   {speedup:>6.1f}x     {avg_size:>8.0f}b   {size_reduction:>6.1f}%"
        )

    # Destaque B-FAST
    bfast_result = next(r for r in results if "B-FAST" in r[0] and "LZ4" not in r[0])
    bfast_compressed = next(r for r in results if "B-FAST + LZ4" in r[0])

    print("\n🎯 B-FAST Highlights:")
    print(f"   • {json_time/bfast_result[1]:.1f}x mais rápido que JSON")
    print(
        f"   • {(1 - bfast_compressed[2]/json_size)*100:.1f}% menor payload (com LZ4)"
    )
    print("   • Zero-copy NumPy arrays")
    print("   • Pydantic native (sem .model_dump())")


if __name__ == "__main__":
    benchmark_serializers()
