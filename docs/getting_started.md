# Guia de Início Rápido - Backend Python

## Instalação
```bash
pip install b-fast
# ou
uv add b-fast
```

## Uso Básico

### Serialização Simples
```python
import b_fast

# Criar encoder (reutilize para melhor performance)
encoder = b_fast.BFast()

data = {"id": 1, "name": "João", "active": True}

# Serializar
binary_data = encoder.encode(data)

# Com compressão (recomendado para payloads > 10KB)
compressed_data = encoder.encode_packed(data, compress=True)
```

### Integração com FastAPI

#### Resposta Customizada
```python
from fastapi import Response
import b_fast

class BFastResponse(Response):
    media_type = "application/x-bfast"
    
    def __init__(self, content=None, *args, **kwargs):
        super().__init__(content, *args, **kwargs)
        self.encoder = b_fast.BFast()

    def render(self, content) -> bytes:
        return self.encoder.encode_packed(content, compress=True)
```

#### Aplicação na Rota
```python
from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class User(BaseModel):
    id: int
    name: str
    email: str

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [User(id=i, name=f"User {i}", email=f"user{i}@example.com") 
            for i in range(1000)]
```

### Trabalhando com Pydantic
```python
from pydantic import BaseModel
import b_fast

class Product(BaseModel):
    id: int
    name: str
    price: float
    in_stock: bool

encoder = b_fast.BFast()
products = [Product(id=1, name="Laptop", price=999.99, in_stock=True)]

# B-FAST lê diretamente da memória do Pydantic (sem .model_dump())
binary_data = encoder.encode(products)
```

### NumPy Arrays
```python
import numpy as np
import b_fast

encoder = b_fast.BFast()

# Zero-copy serialization
array = np.array([1, 2, 3, 4, 5])
data = {"tensor": array, "metadata": {"shape": array.shape}}

binary_data = encoder.encode(data)
```

## Próximos Passos
- [Frontend TypeScript](frontend.md) - Como consumir no cliente
- [Otimização](performance.md) - Dicas de performance