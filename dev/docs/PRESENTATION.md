# 🚀 B-FAST: Otimizações de Paralelismo

## Slide 1: Objetivo

**Implementar otimizações inspiradas no `uv`:**

1. ✅ Paralelismo em nível de thread (Rayon)
2. ⚠️ Análise das limitações do GIL
3. 🔄 Zero-copy deserialization (rkyv)

---

## Slide 2: Compressão Paralela ✅

### Implementação

```rust
fn compress_parallel(&self) -> Vec<u8> {
    data.par_chunks(256 * 1024)  // 256KB chunks
        .map(|chunk| compress_prepend_size(chunk))
        .collect()
}
```

### Resultados

| Tamanho | Tempo | Payload | Comprimido | Redução |
|---------|-------|---------|------------|---------|
| 50k items | 47.88ms | 8.4 MB | 2.5 MB | **69.7%** |

**Overhead:** Apenas 2.33ms (+5%)

---

## Slide 3: Ganhos em Rede 🌐

### Round-Trip Performance

| Rede | Sem Compressão | Com Compressão | Speedup |
|------|----------------|----------------|---------|
| Mobile 3G | 9670ms | 2348ms | **4.1x** |
| WiFi Lento | 975ms | 242ms | **4.0x** |
| Broadband | 105ms | 31ms | **3.4x** |
| Gigabit | 18ms | 10ms | **1.9x** |

**Conclusão:** Ideal para redes lentas!

---

## Slide 4: O Problema do GIL ⚠️

### Tentativa de Paralelizar Serialização

```rust
py_objects.par_iter()
    .map(|obj| {
        Python::with_gil(|py| {  // ❌ GIL serializa aqui
            serialize(obj)
        })
    })
```

### Resultado

- Performance **idêntica** ao modo serial
- GIL força execução sequencial
- Apenas **operações puras em Rust** podem ser paralelizadas

---

## Slide 5: Comparação uv vs B-FAST

### uv (Rust Puro)

```
✅ Sem GIL
✅ Paralelismo total
✅ Todas operações paralelizáveis
```

### B-FAST (Rust + Python)

```
⚠️ Com GIL na serialização
✅ Sem GIL na compressão
✅ Paralelismo parcial
```

**Lição:** GIL é uma barreira real para extensões Python

---

## Slide 6: O Que Funciona ✅

### Operações Paralelizáveis

1. **Compressão LZ4** - Bytes puros em Rust
2. **Criptografia** - Sem acesso a Python
3. **NumPy arrays** - Zero-copy, GIL liberado

### Operações NÃO Paralelizáveis

1. **Serialização de objetos Python** - Requer GIL
2. **Acesso a PyDict, PyList** - Requer GIL
3. **Qualquer Python::with_gil()** - Serializa execução

---

## Slide 7: Performance Geral 📊

### Serialização (10k objetos)

| Formato | Tempo | Payload | Speedup |
|---------|-------|---------|---------|
| JSON | 10.14ms | 1.18 MB | 1.0x |
| orjson | 1.55ms | 1.06 MB | 6.6x |
| **B-FAST** | **4.67ms** | **998 KB** | **2.2x** |
| **B-FAST + LZ4** | **5.27ms** | **252 KB** | **1.9x** |

### NumPy Arrays (1M floats)

| Formato | Tempo | Speedup |
|---------|-------|---------|
| JSON | 346.40ms | 1.0x |
| **B-FAST** | **6.10ms** | **56.8x** |

---

## Slide 8: Arquitetura

```
Python Objects
      │
      ▼
┌─────────────────┐
│  Serialização   │  ← SIMD, Cache-aligned
│  (Single Thread)│  ← Direct memory access
│  ⚠️  Com GIL    │
└────────┬────────┘
         │
         ▼
  Binary Data
         │
         ▼
┌─────────────────┐
│   Compressão    │  ← Rayon (4+ threads)
│  (Multi Thread) │  ← Chunks de 256KB
│  ✅  Sem GIL    │
└────────┬────────┘
         │
         ▼
  Compressed Data
```

---

## Slide 9: Recomendações 💡

### Use `compress=True` quando:

- ✅ Rede lenta (< 100 Mbps)
- ✅ Mobile/IoT
- ✅ Storage/Cache
- ✅ Payload grande (> 100 KB)

### Use `compress=False` quando:

- ✅ Rede ultra-rápida (> 10 Gbps)
- ✅ Payload pequeno (< 10 KB)
- ✅ Latência crítica

---

## Slide 10: Zero-Copy (Em Progresso) 🔄

### Objetivo

```rust
// Deserialização instantânea (apenas cast de ponteiro)
let data = unsafe { rkyv::archived_root(&bytes) };
```

### Status

- ✅ Estruturas rkyv criadas
- ⚠️ Incompatibilidade PyO3 0.20 + rkyv 0.7
- 🔄 Integração pendente

### Ganho Esperado

**~100x mais rápido** na deserialização

---

## Slide 11: Principais Aprendizados 🎓

1. **GIL é uma barreira real**
   - Paralelismo só funciona para operações puras em Rust
   - Acesso a objetos Python sempre serializa

2. **Compressão é paralelizável**
   - Operações em bytes não precisam do GIL
   - Overhead mínimo com Rayon

3. **Trade-offs importam**
   - Nem sempre mais rápido é melhor
   - Depende do contexto (rede, CPU, payload)

4. **Zero-copy é promissor**
   - rkyv pode trazer ganhos massivos
   - Mas requer cuidado com portabilidade

---

## Slide 12: Arquivos Criados 📁

### Documentação (7 arquivos)

- `SUMMARY.md` - Sumário executivo completo
- `OPTIMIZATIONS.md` - Detalhes técnicos
- `OPTIMIZATIONS_README.md` - README focado
- `INDEX.md` - Índice completo
- `docs/GIL_ANALYSIS.md` - Análise do GIL
- `docs/ARCHITECTURE.md` - Diagramas
- `PRESENTATION.md` - Esta apresentação

### Código (3 arquivos)

- `src/lib.rs` - Compressão paralela
- `src/zero_copy.rs` - Estruturas rkyv
- `Cargo.toml` - Dependências

### Testes (4 arquivos)

- `benchmarks/test_final.py`
- `benchmarks/test_compression_parallel.py`
- `benchmarks/test_parallel.py`
- `examples/optimizations.py`

---

## Slide 13: Conclusão ✅

### O que foi alcançado:

✅ **Compressão paralela** - 70% redução, overhead mínimo  
✅ **4x speedup** em redes lentas  
✅ **Análise completa** das limitações do GIL  
✅ **Documentação extensiva** (14 arquivos)  
🔄 **Base para zero-copy** com rkyv  

### B-FAST continua sendo ideal para:

- 📱 Mobile/IoT (economia de dados)
- 🌐 APIs com rede lenta (4x speedup)
- 📊 Data pipelines (56x speedup para NumPy)
- 🗜️ Storage/Cache (70-99% economia)

---

## Slide 14: Próximos Passos 🚀

### Curto Prazo

- [ ] Resolver integração rkyv
- [ ] Benchmark de deserialização
- [ ] Atualizar README principal

### Médio Prazo

- [ ] Decoder TypeScript para rkyv
- [ ] Suporte a streaming
- [ ] Compressão adaptativa

### Longo Prazo

- [ ] Mais tipos Python (datetime, Decimal)
- [ ] Schema evolution
- [ ] Integração Arrow/Parquet

---

## Slide 15: Obrigado! 🙏

**Desenvolvido por:** Marcelo Markus  
**GitHub:** [@marcelomarkus](https://github.com/marcelomarkus)  
**Docs:** https://marcelomarkus.github.io/b-fast/

**Versão:** 1.0.7  
**Data:** 2026-02-06

---

### Links Úteis

- 📚 Documentação completa: `INDEX.md`
- 🧪 Executar benchmarks: `python benchmarks/test_final.py`
- 💡 Ver exemplos: `python examples/optimizations.py`
- 🐛 Reportar issues: https://github.com/marcelomarkus/b-fast/issues
