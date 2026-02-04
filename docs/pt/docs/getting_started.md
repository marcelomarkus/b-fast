# 🚀 Começando com B-FAST

Este guia irá te ajudar a configurar e usar o B-FAST em seus projetos Python e TypeScript.

## 📦 Instalação

### Python
```bash
pip install bfast-py
```

### TypeScript/JavaScript
```bash
npm install bfast-client
```

## 🛠️ Configuração Básica

### 1. Backend Python com FastAPI

```python
from fastapi import FastAPI, Response
from pydantic import BaseModel
import b_fast

class BFastResponse(Response):
    media_type = "application/x-bfast"
    
    def __init__(self, content=None, *args, **kwargs):
        super().__init__(content, *args, **kwargs)
        self.encoder = b_fast.BFast()

    def render(self, content) -> bytes:
        return self.encoder.encode_packed(content, compress=True)

app = FastAPI()

class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [
        User(id=i, name=f"User {i}", email=f"user{i}@example.com", active=True)
        for i in range(1000)
    ]
```

### 2. Frontend TypeScript

```typescript
import { BFastDecoder } from 'bfast-client';

interface User {
    id: number;
    name: string;
    email: string;
    active: boolean;
}

async function loadUsers(): Promise<User[]> {
    const response = await fetch('/users');
    const buffer = await response.arrayBuffer();
    
    // Decodifica automaticamente
    const users = BFastDecoder.decode(buffer) as User[];
    return users;
}

// Uso
loadUsers().then(users => {
    console.log(`Carregados ${users.length} usuários`);
    users.forEach(user => console.log(user.name));
});
```

## 🔧 Configurações Avançadas

### Compressão LZ4

```python
import b_fast

bf = b_fast.BFast()

# Com compressão (recomendado para payloads > 1KB)
data = bf.encode_packed(large_data, compress=True)

# Sem compressão (mais rápido para payloads pequenos)
data = bf.encode_packed(small_data, compress=False)
```

### Arrays NumPy

```python
import numpy as np
import b_fast

bf = b_fast.BFast()

# Array NumPy (148x mais rápido que JSON!)
array = np.random.rand(1000, 100)
data = bf.encode_packed(array, compress=True)

# Decodificar
decoded_array = bf.decode_packed(data)
```

### Reutilização do Encoder

```python
# ✅ Recomendado: Reutilizar o encoder
bf = b_fast.BFast()

for batch in data_batches:
    encoded = bf.encode_packed(batch, compress=True)
    # Processar...

# ❌ Evitar: Criar novo encoder a cada uso
for batch in data_batches:
    bf = b_fast.BFast()  # Ineficiente!
    encoded = bf.encode_packed(batch, compress=True)
```

## 🎯 Casos de Uso Comuns

### 1. API REST com Listas Grandes

```python
@app.get("/products", response_class=BFastResponse)
async def get_products():
    # Lista com milhares de produtos
    products = await db.get_all_products()
    return products  # 78.7% menor que JSON!
```

### 2. Cache Redis

```python
import redis
import b_fast

bf = b_fast.BFast()
redis_client = redis.Redis()

# Salvar no cache
data = bf.encode_packed(expensive_computation_result, compress=True)
redis_client.set("cache_key", data)

# Recuperar do cache
cached_data = redis_client.get("cache_key")
result = bf.decode_packed(cached_data)
```

### 3. Transferência de Dados Científicos

```python
import pandas as pd
import numpy as np

# DataFrame para dict
df_dict = df.to_dict('records')
data = bf.encode_packed(df_dict, compress=True)

# Arrays NumPy diretamente
arrays = {
    'features': feature_matrix,  # numpy array
    'labels': label_vector,      # numpy array
    'metadata': metadata_dict
}
data = bf.encode_packed(arrays, compress=True)
```

## 🚨 Troubleshooting

### Erro: "Module not found"
```bash
# Reinstalar com força
pip install --force-reinstall bfast-py
```

### Performance não esperada
```python
# Verificar se está usando compressão adequadamente
small_data = bf.encode_packed(data, compress=False)  # < 1KB
large_data = bf.encode_packed(data, compress=True)   # > 1KB
```

### Problemas de Compatibilidade
```python
# Verificar versão
import b_fast
print(b_fast.__version__)  # Deve ser >= 1.0.6
```

## 📚 Próximos Passos

- [Frontend](frontend.md) - Integração avançada com TypeScript
- [Performance](performance.md) - Análise técnica detalhada
- [Solução de Problemas](troubleshooting.md) - Guia completo de troubleshooting
