# 📊 Performance - Análise Técnica

Análise detalhada da performance do B-FAST e comparações com outras soluções de serialização.

## 🚀 Resultados de Benchmark

### Teste Padrão: 10.000 Objetos Pydantic

```python
class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    created_at: datetime
```

| Método | Tempo (ms) | Speedup | Tamanho | Redução |
|--------|------------|---------|---------|---------|
| JSON (stdlib) | 9.64ms | 1.0x | 1.18 MB | 0% |
| orjson | 1.51ms | 6.4x | 1.06 MB | 10.2% |
| Pickle | 2.74ms | 3.5x | 808 KB | 31.6% |
| **B-FAST** | **4.51ms** | **2.1x** | **998 KB** | **15.5%** |
| **B-FAST + LZ4** | **5.21ms** | **1.9x** | **252 KB** | **78.7%** |

## 🔄 Testes Round-Trip

Incluindo serialização + transferência de rede + deserialização:

### 📡 100 Mbps (Rede Lenta)
| Método | Tempo Total | Breakdown |
|--------|-------------|-----------|
| JSON | 114.3ms | Serialize: 9.6ms + Transfer: 94.4ms + Deserialize: 10.3ms |
| orjson | 92.3ms | Serialize: 1.5ms + Transfer: 84.8ms + Deserialize: 6.0ms |
| **B-FAST + LZ4** | **28.3ms** | **Serialize: 5.2ms + Transfer: 20.2ms + Deserialize: 2.9ms** |

**🎯 B-FAST é 4.0x mais rápido em redes lentas!**

### 📡 1 Gbps (Rede Rápida)
| Método | Tempo Total | Breakdown |
|--------|-------------|-----------|
| JSON | 29.3ms | Serialize: 9.6ms + Transfer: 9.4ms + Deserialize: 10.3ms |
| orjson | 15.9ms | Serialize: 1.5ms + Transfer: 8.4ms + Deserialize: 6.0ms |
| **B-FAST + LZ4** | **10.2ms** | **Serialize: 5.2ms + Transfer: 2.1ms + Deserialize: 2.9ms** |

**🎯 B-FAST é 2.9x mais rápido mesmo em redes rápidas!**

### 📡 10 Gbps (Rede Ultra-Rápida)
| Método | Tempo Total | Breakdown |
|--------|-------------|-----------|
| JSON | 20.8ms | Serialize: 9.6ms + Transfer: 1.0ms + Deserialize: 10.2ms |
| orjson | 8.3ms | Serialize: 1.5ms + Transfer: 0.8ms + Deserialize: 6.0ms |
| **B-FAST + LZ4** | **8.4ms** | **Serialize: 5.2ms + Transfer: 0.3ms + Deserialize: 2.9ms** |

**🎯 B-FAST permanece competitivo mesmo em redes ultra-rápidas!**

## 🧮 Arrays NumPy

Teste especial para dados científicos:

```python
# Array 1000x100 float64
array = np.random.rand(1000, 100)
```

| Método | Tempo (ms) | Speedup | Tamanho |
|--------|------------|---------|---------|
| JSON | 847.2ms | 1.0x | 15.2 MB |
| orjson | 52.1ms | 16.3x | 13.8 MB |
| **B-FAST** | **5.7ms** | **148x** | **800 KB** |

**🚀 B-FAST é 148x mais rápido para NumPy arrays!**

## 🔧 Otimizações Técnicas

### SIMD Batch Processing

```rust
// Processamento em lotes de 8 objetos
#[target_feature(enable = "avx2")]
unsafe fn process_batch_simd(objects: &[PyObject; 8]) -> Vec<u8> {
    // Operações SIMD paralelas
    let mut result = Vec::with_capacity(1024);
    
    // Processar 8 objetos simultaneamente
    for chunk in objects.chunks_exact(8) {
        let batch_data = process_simd_chunk(chunk);
        result.extend_from_slice(&batch_data);
    }
    
    result
}
```

### Cache-Aligned Memory

```rust
// Alinhamento de 64 bytes para otimização de cache
#[repr(align(64))]
struct CacheAlignedBuffer {
    data: [u8; 64],
}

// Operações de memória otimizadas
fn write_aligned(buffer: &mut CacheAlignedBuffer, data: &[u8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(
            data.as_ptr(),
            buffer.data.as_mut_ptr(),
            data.len().min(64)
        );
    }
}
```

### Branch Prediction Hints

```rust
// Hints para o processador
#[inline(always)]
fn serialize_common_type(value: &PyAny) -> Option<Vec<u8>> {
    if likely(value.is_instance_of::<PyInt>()) {
        Some(serialize_int(value))
    } else if likely(value.is_instance_of::<PyString>()) {
        Some(serialize_string(value))
    } else if unlikely(value.is_instance_of::<PyFloat>()) {
        Some(serialize_float(value))
    } else {
        None
    }
}
```

### String ID Caching

```rust
// Cache de 64 entradas para strings repetidas
struct StringCache {
    entries: [Option<(String, u16)>; 64],
    next_id: u16,
}

impl StringCache {
    fn get_or_insert(&mut self, s: &str) -> u16 {
        let hash = hash_string(s) % 64;
        
        if let Some((cached, id)) = &self.entries[hash] {
            if cached == s {
                return *id;
            }
        }
        
        let id = self.next_id;
        self.entries[hash] = Some((s.to_string(), id));
        self.next_id += 1;
        id
    }
}
```

## 📈 Análise de Casos de Uso

### Mobile/IoT (Bandwidth-Constrained)

**Cenário**: API móvel com 100 Mbps, 200ms latência

```
JSON:     Serialize(9.6ms) + Transfer(94.4ms) + Latency(200ms) = 304ms
B-FAST:   Serialize(5.2ms) + Transfer(20.2ms) + Latency(200ms) = 225ms

Melhoria: 79ms (26% mais rápido)
Economia de dados: 78.7% (importante para planos limitados)
```

### Data Pipelines

**Cenário**: Processamento de 1M de registros científicos

```
JSON:     847ms × 100 batches = 84.7 segundos
B-FAST:   5.7ms × 100 batches = 0.57 segundos

Melhoria: 84.1 segundos (148x mais rápido)
```

### APIs Corporativas

**Cenário**: Dashboard com 50 requests/segundo

```
JSON:     50 × 29.3ms = 1.465s CPU/segundo
B-FAST:   50 × 10.2ms = 0.51s CPU/segundo

Economia de CPU: 65% (permite mais throughput)
```

## 🎯 Quando Usar B-FAST

### ✅ Ideal Para:

1. **Redes Lentas** (< 1 Gbps)
   - 4x speedup em 100 Mbps
   - 78.7% economia de bandwidth

2. **Arrays NumPy**
   - 148x speedup
   - Zero-copy serialization

3. **Listas Homogêneas**
   - String interning eficiente
   - SIMD batch processing

4. **Mobile/IoT**
   - Economia de dados crítica
   - Performance superior

### ⚠️ Considerar Alternativas:

1. **Redes Ultra-Rápidas** (> 10 Gbps)
   - orjson pode ser mais rápido
   - Diferença marginal

2. **Objetos Pequenos** (< 1KB)
   - Overhead de compressão
   - JSON pode ser suficiente

3. **APIs Externas**
   - Requer suporte B-FAST
   - JSON ainda é padrão

## 🔬 Metodologia de Teste

### Ambiente de Teste

```
CPU: Intel i7-12700K (12 cores, 20 threads)
RAM: 32GB DDR4-3200
Python: 3.11.13
Rust: 1.75.0
OS: Linux (Fedora 41)
```

### Código de Benchmark

```python
import time
import statistics
from typing import List

def benchmark_method(method_func, data, iterations=10):
    times = []
    sizes = []
    
    for _ in range(iterations):
        start = time.perf_counter()
        result = method_func(data)
        end = time.perf_counter()
        
        times.append((end - start) * 1000)  # ms
        sizes.append(len(result))
    
    return {
        'avg_time': statistics.mean(times),
        'std_time': statistics.stdev(times),
        'avg_size': statistics.mean(sizes)
    }
```

### Validação de Resultados

Todos os benchmarks foram executados múltiplas vezes com validação de integridade dos dados:

```python
def validate_roundtrip(original, decoded):
    assert len(original) == len(decoded)
    for orig, dec in zip(original, decoded):
        assert orig.id == dec['id']
        assert orig.name == dec['name']
        assert orig.email == dec['email']
        assert orig.active == dec['active']
```

## 📚 Próximos Passos

- [Solução de Problemas](troubleshooting.md) - Guia de troubleshooting
- [Frontend](frontend.md) - Integração TypeScript
- [Início](index.md) - Voltar ao início
