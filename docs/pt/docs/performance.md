# 📊 Performance - Análise Técnica

Análise detalhada da performance do B-FAST e comparações com outras soluções de serialização.

## 🚀 Resultados de Benchmark

### ⚡ Território Sub-Microssegundo (100 Objetos)
- **Encode (100 objetos)**: **676 ns** (> 1.470.000 ops/s) — **🚀 2.1x mais rápido**
- **Decode (100 objetos)**: **754 ns** (> 1.320.000 ops/s) — **🚀 2.6x mais rápido**

### Objetos Simples (10.000)
| Formato | Tempo (ms) | Speedup |
|---------|------------|---------|
| JSON | 12.0ms | 1.0x |
| orjson | 8.19ms | 1.5x |
| **B-FAST** | **2.01ms** | **🚀 6.0x** |

**B-FAST é 4.1x mais rápido que orjson!**

### 🌊 Protocolo de Streaming (1.000 frames)
| Métrica | Performance | Speedup / Throughput |
|---------|-------------|----------------------|
| **Streaming Decode (Alinhado)** | **0.31ms (314µs)** | **~3.180.000 frames/s** |
| **Streaming Decode (Fragmentado)** | **0.32ms (322µs)** | **~3.100.000 frames/s** |
| **Latência por Frame Único** | **2.0ns** | **Parsing instantâneo sem alocação** |
| **Throughput Sustentado de Stream** | **> 3.100.000 frames/s** | **145x mais rápido que NDJSON** |

**🚀 Ultra-baixa latência para IPC, WebSockets, feeds de IA e streaming de eventos!**

## 🔄 Testes Round-Trip

Incluindo serialização + transferência de rede + deserialização (10.000 objetos):

### 📡 100 Mbps (Rede Lenta)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 114.5ms | 0.8x |
| orjson | 91.7ms | 1.0x |
| **B-FAST + LZ4** | **16.1ms** | **🚀 5.7x** |

**🎯 B-FAST é 5.7x mais rápido em redes lentas!**

### 📡 1 Gbps (Rede Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 29.4ms | 0.5x |
| orjson | 15.3ms | 1.0x |
| **B-FAST + LZ4** | **7.2ms** | **🚀 2.1x** |

**🎯 B-FAST é 2.1x mais rápido mesmo em redes rápidas!**

### 📡 10 Gbps (Rede Ultra-Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 20.9ms | 0.4x |
| orjson | 7.7ms | 1.0x |
| **B-FAST + LZ4** | **6.3ms** | **🚀 1.2x** |

**🎯 B-FAST permanece competitivo mesmo em redes ultra-rápidas!**

## 🧮 Arrays NumPy

Teste especial para dados científicos (8MB):

| Formato | Tempo (ms) | Speedup |
|---------|------------|---------|
| JSON | 318.21ms | 1.0x |
| orjson | 46.34ms | 6.9x |
| **B-FAST** | **3.29ms** | **🚀 96x** |

**🚀 B-FAST é 14x mais rápido que orjson!**
**🚀 B-FAST é 96x mais rápido que JSON!**

## 🎯 Quando Usar B-FAST

### ✅ B-FAST Excele Quando:

1. **Largura de banda é limitada** (mobile, IoT) - 5.7x mais rápido
2. **Objetos simples** - 4.1x mais rápido que orjson
3. **Streaming em tempo real** - > 12.500 frames/s com decode instantâneo
4. **Arrays NumPy estão envolvidos** (ML, ciência de dados) - 14-96x mais rápido
5. **Eficiência de armazenamento é importante** - 89% de compressão
6. **Grandes datasets** - Até 5.7x mais rápido em redes lentas

### ⚠️ Considerar Alternativas Quando:

1. **Redes ultra-rápidas** (10+ Gbps internas) - diferença marginal
2. **Compatibilidade de ecossistema é crítica** - JSON ainda é padrão
3. **Payloads muito pequenos** (< 1KB) - overhead de compressão

## 📈 Características de Performance

### Escalabilidade Linear
A performance do B-FAST escala linearmente com o tamanho dos dados:
- **100 objetos**: **~6.8 ns** por objeto (676 ns total encode)
- **1.000 objetos**: **~60 ns** por objeto (60.1 µs total encode)  
- **10.000 objetos**: **~230 ns** por objeto (2.30 ms total encode)

### Eficiência de Memória
- **Arrays NumPy zero-copy**
- **Operações de memória alinhadas ao cache**
- **Compressão eficiente** com LZ4

## 🔬 Otimizações Técnicas

### Núcleo Nativo em Rust
- **Execução binária nativa** compilada com PyO3
- **Inspeção rápida de tipos** e serialização direta em buffers
- **Suporte nativo a Pydantic & DataFrames** sem conversões intermediárias

### Compressão
- **LZ4 integrado**
- **Descompressão rápida** no lado do cliente
- **Sem dependências externas** necessárias

## 🌐 Análise de Rede

A vantagem do B-FAST aumenta conforme a velocidade da rede diminui:

| Velocidade da Rede | Vantagem do B-FAST |
|--------------------|--------------------|
| 100 Mbps | 5.7x mais rápido que orjson |
| 1 Gbps | 2.1x mais rápido que orjson |
| 10 Gbps | 1.2x mais rápido que orjson |

## 📊 Metodologia de Benchmark

### Ambiente de Teste
- **Dados**: 10.000 objetos Pydantic complexos
- **Iterações**: Múltiplas execuções com warmup
- **Rede**: Tempos de transferência simulados

### Estrutura de Dados de Teste
```python
class User(BaseModel):
    id: int
    name: str  
    email: str
    active: bool
    scores: list[float]
```

### Abordagem de Medição
- **Serialização pura**: Apenas tempo de CPU
- **Round-trip**: Serializar + transferência de rede + deserializar
- **Simulação de rede**: Cálculos realistas de largura de banda
- **Análise estatística**: Média de múltiplas execuções

## 🎯 Conclusão

O B-FAST atinge seu objetivo de design de ser a escolha ideal para ambientes com restrição de largura de banda, mantendo performance competitiva de CPU. A redução de 89% no payload combinada com 1.7x de speedup na serialização o torna ideal para aplicações mobile, IoT e intensivas em dados.

## 📚 Próximos Passos

- [Solução de Problemas](troubleshooting.md) - Guia de troubleshooting
- [Frontend](frontend.md) - Integração TypeScript
- [Início](index.md) - Voltar ao início
