# 🔒 GIL e Paralelismo: Análise Técnica

## O Problema do GIL (Global Interpreter Lock)

### O que é o GIL?

O GIL é um mutex que protege o acesso aos objetos Python, permitindo que apenas uma thread execute código Python por vez. Isso significa que, mesmo em um sistema multi-core, apenas um core pode executar código Python simultaneamente.

### Por que o GIL existe?

1. **Gerenciamento de memória:** Python usa reference counting para garbage collection
2. **Simplicidade:** Evita race conditions em estruturas internas do interpretador
3. **Compatibilidade:** Muitas extensões C assumem que apenas uma thread acessa objetos Python

---

## Tentativa de Paralelismo no B-FAST

### Código Tentado

```rust
use rayon::prelude::*;

// Tentativa de paralelizar serialização
let py_objects: Vec<PyObject> = (0..len)
    .map(|i| list.get_item(i).unwrap().into())
    .collect();

let chunks: Vec<Vec<u8>> = py_objects
    .par_iter()  // ✅ Cria threads nativas do Rust
    .map(|obj| {
        Python::with_gil(|py| {  // ❌ Cada thread precisa do GIL
            let item = obj.as_ref(py);
            serialize_object(item)  // Acessa objeto Python
        })
    })
    .collect();
```

### O que acontece na prática?

```
Thread 1: [Aguardando GIL] ────────────────────────────────
Thread 2: ────────────────── [Aguardando GIL] ────────────
Thread 3: ──────────────────────────────────── [Aguardando GIL]
Thread 4: [Serializa] ──────── [Serializa] ──────── [Serializa]
          ^                    ^                    ^
          GIL adquirido        GIL adquirido        GIL adquirido
```

**Resultado:** Execução serializada, sem ganho de performance.

---

## Onde o Paralelismo FUNCIONA

### 1. Compressão LZ4 (Operação Pura em Rust)

```rust
fn compress_parallel(&self) -> Vec<u8> {
    let chunks: Vec<Vec<u8>> = data
        .par_chunks(CHUNK_SIZE)  // ✅ Dados já em Rust
        .map(|chunk| {
            compress_prepend_size(chunk)  // ✅ Sem acesso a Python
        })
        .collect();
}
```

**Por que funciona?**
- ✅ Dados já foram extraídos de Python
- ✅ Operação pura em Rust (compressão de bytes)
- ✅ Nenhum acesso a objetos Python
- ✅ Sem necessidade do GIL

**Resultado:** Paralelismo real, múltiplos cores trabalhando simultaneamente.

---

### 2. Processamento de NumPy Arrays

```rust
// NumPy arrays são zero-copy
let array = val.extract::<PyReadonlyArrayDyn<f64>>()?;
let raw_data = array.as_slice()?;  // ✅ Ponteiro direto para memória

// Pode processar em paralelo
raw_data.par_chunks(1024)
    .map(|chunk| process_chunk(chunk))  // ✅ Sem GIL
    .collect();
```

**Por que funciona?**
- ✅ NumPy libera o GIL para operações em arrays
- ✅ Acesso direto à memória (zero-copy)
- ✅ Processamento puro em Rust

---

## Comparação: uv vs B-FAST

### uv (Rust puro)

```rust
// uv não tem GIL - tudo é Rust
dependencies.par_iter()
    .map(|dep| resolve_dependency(dep))  // ✅ Paralelismo real
    .collect();
```

**Vantagem:** Sem Python, sem GIL, paralelismo total.

### B-FAST (Rust + Python)

```rust
// B-FAST precisa acessar objetos Python
pydantic_objects.par_iter()
    .map(|obj| {
        Python::with_gil(|py| {  // ❌ GIL serializa
            serialize(obj)
        })
    })
    .collect();
```

**Limitação:** Precisa do GIL para acessar objetos Python.

---

## Soluções e Workarounds

### 1. ✅ Extrair dados antes de paralelizar

```rust
// Extrair todos os dados de Python primeiro (com GIL)
let rust_data: Vec<RustStruct> = python_objects
    .iter()
    .map(|obj| extract_to_rust(obj))
    .collect();

// Agora pode paralelizar (sem GIL)
let results = rust_data
    .par_iter()
    .map(|data| process_pure_rust(data))
    .collect();
```

**Trade-off:** Overhead de conversão Python → Rust.

### 2. ✅ Paralelizar operações puras

```rust
// Serialização: serial (com GIL)
let serialized = serialize_with_gil(objects);

// Compressão: paralela (sem GIL)
let compressed = compress_parallel(serialized);
```

**Implementado no B-FAST:** Compressão paralela.

### 3. 🔄 Usar rkyv para zero-copy

```rust
// Serializar para formato rkyv (com GIL)
let archived = serialize_to_rkyv(objects);

// Deserializar: instantâneo (sem parsing)
let data = unsafe { rkyv::archived_root(&bytes) };
```

**Status:** Em desenvolvimento.

---

## Conclusões

### Limitações do GIL:
1. ❌ Serialização de objetos Python não pode ser paralelizada
2. ❌ Qualquer acesso a `PyAny`, `PyDict`, `PyList` requer GIL
3. ❌ `Python::with_gil()` serializa a execução

### Onde o paralelismo funciona:
1. ✅ Operações puras em Rust (compressão, criptografia)
2. ✅ Processamento de NumPy arrays (zero-copy)
3. ✅ Após extrair dados de Python para structs Rust

### Estratégia do B-FAST:
1. **Serialização:** Otimizar com SIMD, cache alignment, acesso direto à memória
2. **Compressão:** Paralelizar com Rayon (sem GIL)
3. **Futuro:** Zero-copy deserialization com rkyv

---

## Referências

- [Python GIL Documentation](https://docs.python.org/3/glossary.html#term-global-interpreter-lock)
- [PyO3 Parallelism Guide](https://pyo3.rs/v0.20.0/parallelism)
- [Rayon Documentation](https://docs.rs/rayon/)
- [rkyv Zero-Copy Deserialization](https://rkyv.org/)

---

**Autor:** Marcelo Markus  
**Data:** 2026-02-06
