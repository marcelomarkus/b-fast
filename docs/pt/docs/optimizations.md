# ⚡ Arquitetura & Otimizações do B-FAST

Uma análise aprofundada da arquitetura de performance e das otimizações internas que tornam o B-FAST um dos formatos de serialização binária mais rápidos do ecossistema Python e TypeScript.

---

## 🧭 Pilares de Otimização

<div class="grid cards" markdown>

-   __🚀 Fast-Paths para Tipos Concretos__

    ---

    Checagem direta de ponteiros CPython em Rust (`PyBool`, `PyLong`, `PyFloat`, `PyString`, `PyDict`, `PyList`), eliminando chamadas dinâmicas de reflexão (`hasattr`).

-   __🧮 Arrays NumPy Zero-Copy__

    ---

    Mapeamento direto de memória para tensores e ndarrays do NumPy, atingindo **14-96x mais velocidade** do que JSON e orjson.

-   __🗜️ Compressão Paralela com LZ4__

    ---

    Payloads acima de 1MB acionam automaticamente multi-threading via Rayon em Rust para compressão quase instantânea com mínima latência.

-   __🧠 String Interning sem Alocações Extras__

    ---

    Pré-alocação da tabela de strings no decode e lookups por igualdade de ponteiros no encode economizam dezenas de milhares de chamadas por payload.

</div>

---

## 1. Rotas Rápidas sem Reflexão Dinâmica

Serializadores comuns frequentemente dependem de reflexão em tempo de execução (`hasattr`, `getattr`) para inspecionar objetos Python, o que gera buscas lentas e exceções internas no interpretador CPython.

O B-FAST elimina esse overhead reconhecendo tipos concretos do Python diretamente na camada nativa:
- **Zero Overhead de Exceções**: Primitivos comuns (`bool`, `int`, `float`, `str`, `dict`, `list`) evitam totalmente checagens dinâmicas.
- **Mapeamento Direto**: Objetos Python são traduzidos imediatamente para buffers binários com mínimo estado intermediário.

**Impacto na Performance:**
- **Serialização de Primitivos:** Reduzida de **118.4 ms** para **4.8 ms** (**~24x mais rápido**).
- **Estruturas Aninhadas:** Reduzida de **408.2 ms** para **34.0 ms** (**~12x mais rápido**).

---

## 2. Arrays NumPy Zero-Copy & Lotes SIMD

Arrays NumPy são serializados diretamente a partir de seus buffers contíguos em C:

```python
import numpy as np
from b_fast import BFast

encoder = BFast()
matrix = np.random.rand(1000, 1000)  # Array float64 de 8MB

# Cópia direta de memória - zero conversão para texto ou JSON
payload = encoder.encode_packed(matrix, compress=False)
```

### Performance em Array de 8MB

| Formato | Tempo de Serialização | Speedup vs JSON | Speedup vs orjson |
| :--- | :---: | :---: | :---: |
| **B-FAST** | **3.29 ms** | **🚀 96x** | **🚀 14x** |
| **orjson** | **46.34 ms** | 6.9x | 1.0x |
| **JSON** | **318.21 ms** | 1.0x | 0.15x |

---

## 3. Compressão Multi-Thread Paralela

Para payloads maiores que 1MB, o B-FAST divide automaticamente o buffer binário em blocos independentes e os comprime em paralelo utilizando os múltiplos núcleos de CPU via Rayon no Rust.

```python
encoder = BFast()

# Aciona automaticamente compressão paralela multi-core
compressed = encoder.encode_packed(large_dataset, compress=True)
```

### Diretrizes de Compressão

=== "Quando ativar `compress=True`"
    - **Redes lentas ou móveis (< 100 Mbps):** 89% de redução de tamanho entrega até **5.7x mais velocidade** no round-trip total.
    - **Grandes volumes de dados (> 100 KB):** O custo de CPU da compressão é irrelevante frente ao ganho na transmissão de rede.
    - **Armazenamento em Disco ou Redis:** Economiza memória e reduz a pressão de despejo de cache.

=== "Quando manter `compress=False`"
    - **Redes internas ultra-rápidas (10+ Gbps):** A serialização direta já é sub-milissegundo.
    - **Streaming em tempo real (< 1 KB por frame):** Evita latência adicional por frame individual.
    - **Ambientes com CPU restrita:** Empacotamento binário direto sem passagem pelo LZ4.

---

## 4. Pré-Alocação no Decoder & Lazy Loading

Na decodificação de grandes matrizes de dicionários ou streams:

1. **Tabela de Strings Pré-alocada:** Durante a leitura do cabeçalho, os objetos `PyString` são instanciados uma única vez em uma tabela indexada. As chaves são resolvidas por busca de ponteiros, poupando 50.000+ alocações de string por payload.
2. **Lazy Loading de Tipos:** As classes `datetime`, `date`, `time`, `UUID` e `Decimal` só são importadas se tags estendidas (`0x80`–`0x84`) forem encontradas no payload, eliminando ~80 µs de overhead por chamada de decode comum.

---

## 5. Melhores Práticas para Throughput Máximo

### 1. Reutilize Instâncias do Encoder

```python
# ✅ Recomendado: Reutiliza buffers internos e cache de interning de strings
encoder = BFast()
for batch in data_batches:
    payload = encoder.encode_packed(batch)

# ❌ Evitar: Criar um novo encoder por lote refaz todas as alocações
for batch in data_batches:
    payload = BFast().encode_packed(batch)
```

### 2. Transmita Feeds Grandes com `BFastStreamingResponse`

Em vez de acumular gigabytes em memória antes de enviar, transmita os dados incrementalmente com framing de streaming:

```python
from b_fast import BFastStreamingResponse

@app.get("/telemetria")
async def stream_telemetria():
    async def feed():
        for chunk in fonte_telemetria:
            yield chunk
    return BFastStreamingResponse(feed())
```
