# Otimização e Performance

## Quando Usar Compressão LZ4

### Recomendações por Tamanho de Payload
- **< 512 bytes:** Desative compressão (overhead > benefício)
- **512 bytes - 10KB:** Teste ambos os modos
- **> 10KB:** Sempre ative compressão

```python
encoder = b_fast.BFast()

# Payload pequeno - sem compressão
small_data = {"id": 1, "status": "ok"}
binary = encoder.encode(small_data)

# Payload grande - com compressão
large_data = {"users": [{"id": i, "name": f"User {i}"} for i in range(1000)]}
binary = encoder.encode_packed(large_data, compress=True)
```

## Reutilização do Encoder

### String Interning Inteligente
O encoder B-FAST mantém uma tabela de strings interna que melhora com o uso:

```python
# ❌ Ineficiente - cria novo encoder a cada chamada
def bad_serialize(data):
    encoder = b_fast.BFast()  # Nova string table vazia
    return encoder.encode(data)

# ✅ Eficiente - reutiliza string table
encoder = b_fast.BFast()  # Instância global/singleton

def good_serialize(data):
    return encoder.encode(data)  # String table acumula conhecimento
```

### FastAPI com Encoder Singleton
```python
from fastapi import FastAPI, Response
import b_fast

# Encoder global para toda aplicação
app_encoder = b_fast.BFast()

class BFastResponse(Response):
    media_type = "application/x-bfast"
    
    def render(self, content) -> bytes:
        return app_encoder.encode_packed(content, compress=True)
```

## NumPy Zero-Copy

### Performance Máxima com Arrays
```python
import numpy as np
import b_fast

encoder = b_fast.BFast()

# ✅ Zero-copy - lê buffer direto da memória
array = np.array([1, 2, 3, 4, 5], dtype=np.float32)
data = {"tensor": array}
binary = encoder.encode(data)  # ~20x mais rápido

# ❌ Evite conversões desnecessárias
array_as_list = array.tolist()  # Copia dados para Python
data = {"tensor": array_as_list}
binary = encoder.encode(data)  # Muito mais lento
```

### Tipos NumPy Suportados
- `int8`, `int16`, `int32`, `int64`
- `uint8`, `uint16`, `uint32`, `uint64`
- `float32`, `float64`
- `bool`

## Compatibilidade Pydantic

### Pydantic v2 (Recomendado)
```python
from pydantic import BaseModel
import b_fast

class User(BaseModel):
    id: int
    name: str
    email: str

encoder = b_fast.BFast()
users = [User(id=1, name="João", email="joao@example.com")]

# Pydantic v2: acesso direto à estrutura Rust interna
binary = encoder.encode(users)  # Performance máxima
```

### Pydantic v1 (Compatível)
```python
# Funciona, mas com performance reduzida
# Recomenda-se migrar para v2 quando possível
```

## Benchmarks de Referência

### Ambiente de Teste
- CPU: Intel i7-12700K
- RAM: 32GB DDR4-3200
- Python: 3.11
- Payload: 10.000 objetos Pydantic

### Resultados
| Método | Tempo | Tamanho | Compressão |
|--------|-------|---------|------------|
| JSON padrão | 45.2ms | 1.2MB | - |
| orjson | 12.8ms | 1.1MB | - |
| B-FAST | 1.4ms | 240KB | LZ4 |
| B-FAST (sem compressão) | 0.8ms | 580KB | - |

## Dicas Gerais

1. **Meça sempre:** Use `time.perf_counter()` para validar ganhos
2. **Profile primeiro:** Identifique gargalos antes de otimizar
3. **Teste em produção:** Benchmarks locais podem não refletir a realidade
4. **Monitore memória:** Payloads muito grandes podem impactar RAM