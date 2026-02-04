import json
import timeit

import matplotlib.pyplot as plt
import numpy as np
import orjson
from pydantic import BaseModel

import b_fast


# 1. Configuração do Cenário
class UserData(BaseModel):
    id: int
    name: str
    active: bool
    scores: list


# Criamos 10.000 objetos para um teste de estresse real
data_load = [
    UserData(id=i, name=f"User_Performance_{i}", active=True, scores=[10, 20, 30])
    for i in range(10000)
]

# Adicionamos um Array NumPy para o B-FAST mostrar vantagem extra
numpy_data = np.random.rand(1000)
full_payload = {"users": data_load, "matrix": numpy_data}

# Instanciamos o seu encoder
encoder = b_fast.BFast()


def test_bfast():
    return encoder.encode_packed(full_payload, compress=True)


def test_orjson():
    # Orjson precisa converter Pydantic e NumPy manualmente
    processed = {
        "users": [u.model_dump() for u in data_load],
        "matrix": numpy_data.tolist(),
    }
    return orjson.dumps(processed)


def test_json():
    processed = {
        "users": [u.model_dump() for u in data_load],
        "matrix": numpy_data.tolist(),
    }
    return json.dumps(processed).encode()


# 2. Execução dos Testes
print("🧪 Iniciando Benchmarks...")
iters = 50

t_bf = timeit.timeit(test_bfast, number=iters) / iters
t_oj = timeit.timeit(test_orjson, number=iters) / iters
t_js = timeit.timeit(test_json, number=iters) / iters

# 3. Gerando o Gráfico
labels = ["JSON (Standard)", "Orjson", "B-FAST (Sua Lib)"]
times = [t_js * 1000, t_oj * 1000, t_bf * 1000]  # Converter para milissegundos

plt.figure(figsize=(10, 6))
bars = plt.bar(labels, times, color=["#ff9999", "#66b3ff", "#99ff99"])

plt.ylabel("Tempo Médio por Execução (ms)")
plt.title("Benchmark de Serialização: Latência (Menor é Melhor)")
plt.grid(axis="y", linestyle="--", alpha=0.7)

# Adicionando os valores nas barras
for bar in bars:
    yval = bar.get_height()
    plt.text(
        bar.get_x() + bar.get_width() / 2,
        yval + 0.1,
        f"{yval:.2f}ms",
        ha="center",
        va="bottom",
        fontweight="bold",
    )

plt.savefig("benchmark_results.png")
print("✅ Gráfico gerado com sucesso: 'benchmark_results.png'")
plt.show()
