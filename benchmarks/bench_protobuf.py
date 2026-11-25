#!/usr/bin/env python3
"""
Benchmark: B-FAST vs Protocol Buffers (Protobuf)
Evaluates:
  1. Payload Size & Compression Ratio (Raw vs Compressed)
  2. Encode Performance (Pure Serializer vs Real-world Pydantic E2E)
  3. Decode Performance (Message Parse, Field Access, Full Python Dicts)
  4. NumPy Array Serialization (B-FAST vs Protobuf)
  5. Simulated Network Round-Trip
"""

import os
import sys
import time
import zlib

import numpy as np
import orjson
from google.protobuf.json_format import MessageToDict
from pydantic import BaseModel

# Add current directory to path
sys.path.insert(0, os.path.dirname(__file__))

import user_pb2

import b_fast


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]
    description: str


def generate_users(count: int) -> list[User]:
    return [
        User(
            id=i,
            name=f"User {i}",
            email=f"user_{i}@example.com",
            active=(i % 2 == 0),
            scores=[
                float(i * 0.5),
                float(i * 1.5),
                float(i * 2.5),
                float(i * 3.5),
                float(i * 4.5),
            ],
            description=f"Profile description for user {i} with some metadata text.",
        )
        for i in range(count)
    ]


def populate_protobuf_list(users: list[User]) -> user_pb2.UserList:
    proto_list = user_pb2.UserList()
    for u in users:
        p = proto_list.users.add()
        p.id = u.id
        p.name = u.name
        p.email = u.email
        p.active = u.active
        p.scores.extend(u.scores)
        p.description = u.description
    return proto_list


def measure(fn, iterations=20, warmup=5):
    for _ in range(warmup):
        fn()
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        fn()
        t1 = time.perf_counter()
        times.append((t1 - t0) * 1000.0)  # ms
    times.sort()
    # Trim 10% outliers
    trim = max(1, len(times) // 10)
    trimmed = times[trim:-trim] if len(times) > 4 else times
    avg_ms = sum(trimmed) / len(trimmed)
    min_ms = times[0]
    return avg_ms, min_ms


def run_benchmark(item_count: int = 10000, iterations: int = 15):
    print("\n" + "=" * 84)
    print(f"🚀 BENCHMARK: B-FAST vs PROTOCOL BUFFERS ({item_count:,} Objects)")
    print("=" * 84)

    print(f"• Generating {item_count:,} Pydantic objects & Protobuf messages...")
    users = generate_users(item_count)
    users_dict = [u.model_dump() for u in users]
    proto_list = populate_protobuf_list(users)

    bfast = b_fast.BFast()

    # Pre-encode payloads
    bfast_raw = bfast.encode_packed(users, compress=False)
    bfast_compressed = bfast.encode_packed(users, compress=True)
    proto_raw = proto_list.SerializeToString()
    proto_compressed = zlib.compress(proto_raw, level=1)
    orjson_raw = orjson.dumps(users_dict)

    # -------------------------------------------------------------------------
    # 1. PAYLOAD SIZES
    # -------------------------------------------------------------------------
    print("\n📦 1. TAMANHO DO PAYLOAD (WIRE SIZE)")
    print("-" * 84)
    print(
        f"{'Formato':<26} | {'Tamanho':<16} | {'Tamanho (KB)':<14} | {'% do orjson':<15}"
    )
    print("-" * 84)

    base_sz = len(orjson_raw)
    formats = [
        ("orjson (JSON baseline)", len(orjson_raw)),
        ("Protobuf (Raw Wire)", len(proto_raw)),
        ("Protobuf (Comprimido)", len(proto_compressed)),
        ("B-FAST (Raw Wire)", len(bfast_raw)),
        ("B-FAST (Comprimido)", len(bfast_compressed)),
    ]
    for name, sz in formats:
        print(
            f"{name:<26} | {sz:>13,} B | {sz/1024:>11.2f} KB | {(sz/base_sz)*100:>14.1f}%"
        )

    # -------------------------------------------------------------------------
    # 2. ENCODE (SERIALIZATION)
    # -------------------------------------------------------------------------
    print("\n⚡ 2. SERIALIZAÇÃO (ENCODE)")
    print("-" * 84)
    print("A) Cenário Real em Python (Origem: Objetos Pydantic da Aplicação)")
    print("-" * 84)

    def proto_e2e_encode():
        p_list = user_pb2.UserList()
        for u in users:
            p = p_list.users.add()
            p.id = u.id
            p.name = u.name
            p.email = u.email
            p.active = u.active
            p.scores.extend(u.scores)
            p.description = u.description
        return p_list.SerializeToString()

    pb_e2e_avg, _ = measure(
        proto_e2e_encode, iterations=max(5, iterations // 2), warmup=2
    )
    bf_e2e_avg, bf_e2e_min = measure(
        lambda: bfast.encode_packed(users, compress=False), iterations=iterations
    )
    bf_comp_e2e_avg, _ = measure(
        lambda: bfast.encode_packed(users, compress=True), iterations=iterations
    )

    print(
        f"{'Método':<36} | {'Média (ms)':<14} | {'Mínimo (ms)':<14} | {'Comparativo':<15}"
    )
    print("-" * 84)
    print(
        f"{'Protobuf E2E (Model -> Proto -> Bytes)':<36} | {pb_e2e_avg:>11.2f}ms | {'-':>14} | {'1.00x (baseline)':>15}"
    )
    print(
        f"{'B-FAST Raw (Direto de Pydantic)':<36} | {bf_e2e_avg:>11.2f}ms | {bf_e2e_min:>11.2f}ms | {pb_e2e_avg/bf_e2e_avg:>14.2f}x mais rápido"
    )
    print(
        f"{'B-FAST Comprimido':<36} | {bf_comp_e2e_avg:>11.2f}ms | {'-':>14} | {pb_e2e_avg/bf_comp_e2e_avg:>14.2f}x"
    )

    print("\nB) Serialização Pura da Engine")
    print("-" * 84)
    pb_raw_enc, pb_raw_enc_min = measure(
        lambda: proto_list.SerializeToString(), iterations=iterations
    )
    print(
        f"{'Protobuf SerializeToString()':<36} | {pb_raw_enc:>11.2f}ms | {pb_raw_enc_min:>11.2f}ms | {'-':>15}"
    )
    print(
        f"{'B-FAST Encode (direto de Pydantic)':<36} | {bf_e2e_avg:>11.2f}ms | {bf_e2e_min:>11.2f}ms | {'-':>15}"
    )

    # -------------------------------------------------------------------------
    # 3. DECODE (DESERIALIZATION)
    # -------------------------------------------------------------------------
    print("\n⚡ 3. DESERIALIZAÇÃO (DECODE)")
    print("-" * 84)

    # 1. ParseFromString only
    def pb_parse_only():
        target = user_pb2.UserList()
        target.ParseFromString(proto_raw)
        return target

    pb_parse_avg, _ = measure(pb_parse_only, iterations=iterations)

    # 2. Parse + Access fields (reading attributes in Python)
    def pb_parse_access():
        target = user_pb2.UserList()
        target.ParseFromString(proto_raw)
        count = 0
        for u in target.users:
            count += u.id + len(u.name) + len(u.scores)
        return count

    pb_access_avg, _ = measure(pb_parse_access, iterations=iterations)

    # 3. Parse + Full conversion to Python dictionaries (equivalent to B-FAST output)
    def pb_parse_dict():
        target = user_pb2.UserList()
        target.ParseFromString(proto_raw)
        return MessageToDict(target)

    pb_dict_avg, _ = measure(pb_parse_dict, iterations=5, warmup=1)

    # 4. B-FAST Decode (Returns native Python dicts/lists directly)
    bf_dec_avg, bf_dec_min = measure(
        lambda: bfast.decode_packed(bfast_raw), iterations=iterations
    )
    bf_comp_dec_avg, _ = measure(
        lambda: bfast.decode_packed(bfast_compressed), iterations=iterations
    )

    print(f"{'Operação de Decode':<42} | {'Tempo Médio (ms)':<18} | {'Retorno':<18}")
    print("-" * 84)
    print(
        f"{'Protobuf: ParseFromString apenas':<42} | {pb_parse_avg:>15.2f}ms | {'Mensagem':<18}"
    )
    print(
        f"{'Protobuf: Parse + Acesso aos atributos':<42} | {pb_access_avg:>15.2f}ms | {'Leitura em Python':<18}"
    )
    print(
        f"{'Protobuf: Parse + MessageToDict (Dicts)':<42} | {pb_dict_avg:>15.2f}ms | {'Dicts nativos':<18}"
    )
    print(f"{'B-FAST: Raw Decode':<42} | {bf_dec_avg:>15.2f}ms | {'Dicts nativos':<18}")
    print(
        f"{'B-FAST: Decode (Comprimido)':<42} | {bf_comp_dec_avg:>15.2f}ms | {'Dicts nativos':<18}"
    )

    speedup_dict = pb_dict_avg / bf_dec_avg
    print("\n💡 Detalhe de Consumo:")
    print(
        f"   • O Protobuf faz parse inicial da mensagem em {pb_parse_avg:.2f}ms, mas para"
    )
    print(
        f"     converter para dicionários Python convencionais, leva {pb_dict_avg:.2f}ms."
    )
    print(f"   • O B-FAST entrega estruturas nativas prontas em {bf_dec_avg:.2f}ms")
    print(f"     ({speedup_dict:.1f}x mais rápido que MessageToDict do Protobuf).")

    # -------------------------------------------------------------------------
    # 4. NUMPY SERIALIZATION
    # -------------------------------------------------------------------------
    print("\n📊 4. CARGAS CIENTÍFICAS / NUMPY (Array 100,000 float64)")
    print("-" * 84)
    arr = np.linspace(0.0, 1000.0, 100_000, dtype=np.float64)
    bf_numpy_bytes = bfast.encode_packed(arr, compress=False)

    bf_numpy_enc, _ = measure(
        lambda: bfast.encode_packed(arr, compress=False), iterations=20
    )
    bf_numpy_dec, _ = measure(
        lambda: bfast.decode_packed(bf_numpy_bytes), iterations=20
    )

    print(
        f"• B-FAST NumPy Encode: {bf_numpy_enc:.3f}ms ({len(bf_numpy_bytes):,} bytes)"
    )
    print(f"• B-FAST NumPy Decode: {bf_numpy_dec:.3f}ms")
    print(
        "• Protobuf: Não possui suporte nativo a ndarray (exige converter para lista ou lidar com raw bytes)."
    )

    # -------------------------------------------------------------------------
    # 5. ROUND-TRIP EM REDE SIMULADA (100 Mbps e 1 Gbps)
    # -------------------------------------------------------------------------
    print("\n🌐 5. SIMULAÇÃO DE ROUND-TRIP EM REDE (Encode + Transferência + Decode)")
    print("-" * 84)
    networks = [
        ("Rede Típica (100 Mbps)", 100 * 1024 * 1024 / 8),
        ("Rede Rápida (1 Gbps)", 1000 * 1024 * 1024 / 8),
    ]

    for net_name, bytes_per_sec in networks:
        print(f"\n📍 {net_name}:")
        print(
            f"{'Formato':<28} | {'Transferência':<15} | {'CPU (Enc+Dec)':<15} | {'Tempo Total':<15}"
        )
        print("-" * 78)

        scenarios = [
            ("Protobuf (Raw)", len(proto_raw), pb_raw_enc + pb_access_avg),
            (
                "Protobuf (Comprimido)",
                len(proto_compressed),
                (pb_raw_enc + 0.9) + (pb_parse_avg + 0.4),
            ),
            ("B-FAST (Raw)", len(bfast_raw), bf_e2e_avg + bf_dec_avg),
            (
                "B-FAST (Comprimido)",
                len(bfast_compressed),
                bf_comp_e2e_avg + bf_comp_dec_avg,
            ),
        ]

        for s_name, s_bytes, s_cpu in scenarios:
            t_net = (s_bytes / bytes_per_sec) * 1000.0  # ms
            t_total = t_net + s_cpu
            print(
                f"{s_name:<28} | {t_net:>12.2f}ms | {s_cpu:>12.2f}ms | {t_total:>12.2f}ms"
            )

    print("\n" + "=" * 84)


if __name__ == "__main__":
    count = 10000
    if len(sys.argv) > 1:
        count = int(sys.argv[1])
    run_benchmark(item_count=count)
