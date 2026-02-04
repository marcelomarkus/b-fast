# 🔧 Solução de Problemas

Guia completo para resolver problemas comuns com B-FAST.

## 🚨 Problemas de Instalação

### Python: "Module not found"

**Sintoma:**
```python
ImportError: No module named 'b_fast'
```

**Soluções:**

1. **Reinstalar com força:**
```bash
pip uninstall bfast-py
pip install --force-reinstall bfast-py
```

2. **Verificar ambiente virtual:**
```bash
which python
pip list | grep bfast
```

3. **Compilação manual (se necessário):**
```bash
git clone https://github.com/marcelomarkus/b-fast.git
cd b-fast
pip install -e . --force-reinstall
```

### TypeScript: "Cannot resolve module"

**Sintoma:**
```typescript
Cannot find module 'bfast-client'
```

**Soluções:**

1. **Reinstalar:**
```bash
npm uninstall bfast-client
npm install bfast-client
```

2. **Verificar tipos:**
```bash
npm install @types/node  # Se necessário
```

3. **Configurar tsconfig.json:**
```json
{
  "compilerOptions": {
    "moduleResolution": "node",
    "esModuleInterop": true
  }
}
```

## ⚡ Problemas de Performance

### Performance Abaixo do Esperado

**Sintomas:**
- B-FAST mais lento que JSON
- Uso excessivo de CPU
- Payloads maiores que esperado

**Diagnóstico:**

```python
import time
import b_fast

bf = b_fast.BFast()
data = [{"id": i, "name": f"User {i}"} for i in range(1000)]

# Teste sem compressão
start = time.perf_counter()
result1 = bf.encode_packed(data, compress=False)
time1 = (time.perf_counter() - start) * 1000

# Teste com compressão
start = time.perf_counter()
result2 = bf.encode_packed(data, compress=True)
time2 = (time.perf_counter() - start) * 1000

print(f"Sem compressão: {time1:.2f}ms, {len(result1)} bytes")
print(f"Com compressão: {time2:.2f}ms, {len(result2)} bytes")
```

**Soluções:**

1. **Ajustar compressão:**
```python
# Para payloads pequenos (< 1KB)
data = bf.encode_packed(small_data, compress=False)

# Para payloads grandes (> 1KB)
data = bf.encode_packed(large_data, compress=True)
```

2. **Reutilizar encoder:**
```python
# ✅ Correto
bf = b_fast.BFast()
for batch in batches:
    result = bf.encode_packed(batch, compress=True)

# ❌ Ineficiente
for batch in batches:
    bf = b_fast.BFast()  # Cria novo encoder
    result = bf.encode_packed(batch, compress=True)
```

3. **Verificar tipos de dados:**
```python
# ✅ Otimizado para Pydantic
from pydantic import BaseModel

class User(BaseModel):
    id: int
    name: str

users = [User(id=i, name=f"User {i}") for i in range(1000)]
data = bf.encode_packed(users, compress=True)

# ❌ Menos otimizado
users = [{"id": i, "name": f"User {i}"} for i in range(1000)]
data = bf.encode_packed(users, compress=True)
```

### Memory Leaks

**Sintoma:**
```
Uso de memória crescendo continuamente
```

**Solução:**
```python
import gc

# Limpar cache periodicamente
bf = b_fast.BFast()

for i, batch in enumerate(large_batches):
    result = bf.encode_packed(batch, compress=True)
    
    # Limpar a cada 100 batches
    if i % 100 == 0:
        gc.collect()
```

## 🔍 Problemas de Serialização

### "Unsupported type" Error

**Sintoma:**
```python
TypeError: Unsupported type for serialization: <class 'datetime.datetime'>
```

**Soluções:**

1. **Converter para tipos suportados:**
```python
from datetime import datetime
from pydantic import BaseModel

class User(BaseModel):
    id: int
    name: str
    created_at: str  # ✅ String ao invés de datetime
    
    @classmethod
    def from_datetime(cls, id: int, name: str, created_at: datetime):
        return cls(
            id=id,
            name=name,
            created_at=created_at.isoformat()  # Converter para string
        )
```

2. **Usar serializers customizados:**
```python
import json
from datetime import datetime

def serialize_with_dates(data):
    # Pré-processar dados complexos
    if isinstance(data, list):
        processed = []
        for item in data:
            if hasattr(item, 'created_at') and isinstance(item.created_at, datetime):
                item_dict = item.dict() if hasattr(item, 'dict') else item
                item_dict['created_at'] = item.created_at.isoformat()
                processed.append(item_dict)
            else:
                processed.append(item)
        return bf.encode_packed(processed, compress=True)
    
    return bf.encode_packed(data, compress=True)
```

### Dados Corrompidos

**Sintoma:**
```python
# Dados decodificados diferentes dos originais
original != decoded
```

**Diagnóstico:**
```python
def validate_roundtrip(original_data):
    # Serializar
    encoded = bf.encode_packed(original_data, compress=True)
    
    # Deserializar
    decoded = bf.decode_packed(encoded)
    
    # Comparar
    if original_data != decoded:
        print("❌ Dados corrompidos!")
        print(f"Original: {original_data[:3]}...")
        print(f"Decoded:  {decoded[:3]}...")
        return False
    
    print("✅ Roundtrip OK")
    return True
```

**Soluções:**

1. **Verificar encoding de strings:**
```python
# Garantir UTF-8
data = [{"name": name.encode('utf-8').decode('utf-8')} for name in names]
```

2. **Validar tipos NumPy:**
```python
import numpy as np

# ✅ Tipos suportados
array = np.array(data, dtype=np.float64)  # ou int32, int64

# ❌ Tipos problemáticos
array = np.array(data, dtype=np.object_)  # Evitar
```

## 🌐 Problemas de Rede

### Frontend não Decodifica

**Sintoma:**
```javascript
Error: Failed to decode B-FAST data
```

**Verificações:**

1. **Content-Type correto:**
```python
# Backend
@app.get("/data", response_class=BFastResponse)
async def get_data():
    return data

# Verificar headers
curl -I http://localhost:8000/data
# Deve retornar: Content-Type: application/x-bfast
```

2. **ArrayBuffer no frontend:**
```typescript
// ✅ Correto
const response = await fetch('/api/data');
const buffer = await response.arrayBuffer();
const data = BFastDecoder.decode(buffer);

// ❌ Incorreto
const response = await fetch('/api/data');
const text = await response.text();  // Não funciona!
```

3. **Verificar compressão:**
```typescript
// Debug: verificar se dados estão comprimidos
function debugBuffer(buffer: ArrayBuffer) {
    const view = new Uint8Array(buffer);
    console.log('First 16 bytes:', Array.from(view.slice(0, 16)));
    
    // B-FAST comprimido geralmente começa com [4, 34, 77, 24]
    if (view[0] === 4 && view[1] === 34) {
        console.log('✅ Parece ser B-FAST comprimido');
    } else {
        console.log('⚠️  Formato inesperado');
    }
}
```

### CORS Issues

**Sintoma:**
```
Access to fetch at 'http://api.example.com' from origin 'http://localhost:3000' has been blocked by CORS policy
```

**Solução:**
```python
from fastapi.middleware.cors import CORSMiddleware

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
    expose_headers=["Content-Type"]  # Importante para B-FAST
)
```

## 🐛 Debug e Logging

### Habilitar Logs Detalhados

```python
import logging

# Configurar logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger('b_fast')

# Wrapper com logs
def debug_encode(data, compress=True):
    logger.debug(f"Encoding {len(data)} items, compress={compress}")
    
    start = time.perf_counter()
    result = bf.encode_packed(data, compress=compress)
    duration = (time.perf_counter() - start) * 1000
    
    logger.debug(f"Encoded in {duration:.2f}ms, size: {len(result)} bytes")
    return result
```

### Profiling de Performance

```python
import cProfile
import pstats

def profile_bfast():
    pr = cProfile.Profile()
    pr.enable()
    
    # Seu código B-FAST aqui
    data = [{"id": i, "name": f"User {i}"} for i in range(10000)]
    result = bf.encode_packed(data, compress=True)
    
    pr.disable()
    
    # Analisar resultados
    stats = pstats.Stats(pr)
    stats.sort_stats('cumulative')
    stats.print_stats(10)  # Top 10 funções mais lentas
```

### Comparação com JSON

```python
import json
import time

def compare_with_json(data):
    # JSON
    json_start = time.perf_counter()
    json_data = json.dumps(data).encode('utf-8')
    json_time = (time.perf_counter() - json_start) * 1000
    
    # B-FAST
    bfast_start = time.perf_counter()
    bfast_data = bf.encode_packed(data, compress=True)
    bfast_time = (time.perf_counter() - bfast_start) * 1000
    
    print(f"JSON:   {json_time:.2f}ms, {len(json_data)} bytes")
    print(f"B-FAST: {bfast_time:.2f}ms, {len(bfast_data)} bytes")
    print(f"Speedup: {json_time/bfast_time:.1f}x")
    print(f"Size reduction: {(1-len(bfast_data)/len(json_data))*100:.1f}%")
```

## 📞 Suporte

### Informações para Reportar Bugs

Ao reportar problemas, inclua:

```python
import b_fast
import sys
import platform

print("=== B-FAST Debug Info ===")
print(f"B-FAST version: {b_fast.__version__}")
print(f"Python version: {sys.version}")
print(f"Platform: {platform.platform()}")
print(f"Architecture: {platform.architecture()}")

# Teste básico
try:
    bf = b_fast.BFast()
    test_data = [{"id": 1, "name": "test"}]
    encoded = bf.encode_packed(test_data, compress=True)
    decoded = bf.decode_packed(encoded)
    print("✅ Basic test passed")
except Exception as e:
    print(f"❌ Basic test failed: {e}")
```

### Links Úteis

- **GitHub Issues**: https://github.com/marcelomarkus/b-fast/issues
- **Documentação**: https://marcelomarkus.github.io/b-fast/
- **Exemplos**: https://github.com/marcelomarkus/b-fast/tree/main/examples

## 📚 Próximos Passos

- [Performance](performance.md) - Análise técnica detalhada
- [Frontend](frontend.md) - Integração TypeScript
- [Início](index.md) - Voltar ao início
