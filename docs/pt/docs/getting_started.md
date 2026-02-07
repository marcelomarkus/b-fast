# Guia de Início Rápido - Backend Python

## Instalação
```bash
uv add bfast-py
# ou
pip install bfast-py
```

## Uso Básico

### Serialização Simples
```python
import b_fast

# Criar encoder
encoder = b_fast.BFast()

# Seus dados
data = [{"id": i, "name": f"User {i}"} for i in range(1000)]

# Serializar
encoded = encoder.encode_packed(data, compress=True)
print(f"Tamanho: {len(encoded)} bytes")

# Deserializar
decoded = encoder.decode_packed(encoded)
```

### Compressão
```python
# Com compressão (recomendado para > 1KB)
compressed_data = encoder.encode_packed(data, compress=True)
```

### Integração com FastAPI ⭐ Recomendado

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
    return [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]
```

## Próximos Passos

- [Integração Frontend](frontend.md) - Configuração do cliente TypeScript
- [Performance](performance.md) - Benchmarks detalhados
- [Solução de Problemas](troubleshooting.md) - Problemas comuns
