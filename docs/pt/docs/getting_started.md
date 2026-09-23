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

O B-FAST disponibiliza classes nativas de `Response` e `StreamingResponse` para FastAPI e Starlette.

#### Resposta Padrão
```python
from fastapi import FastAPI
from pydantic import BaseModel
from b_fast import BFastResponse

app = FastAPI()

class User(BaseModel):
    id: int
    name: str
    email: str

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]
```

#### Resposta Streamable HTTP (Streaming Contínuo) 🌊
Transmita frames binários em tempo real sobre HTTP/1.1 (Chunked), HTTP/2 ou HTTP/3:

```python
import asyncio
from fastapi import FastAPI
from b_fast import BFastStreamingResponse

app = FastAPI()

@app.get("/stream-users", response_class=BFastStreamingResponse)
async def stream_users():
    async def user_generator():
        for i in range(100):
            yield {"id": i, "name": f"User {i}", "status": "active"}
            await asyncio.sleep(0.05)
    
    return user_generator()
```

---

### Integração com FastMCP 2.0 🤖
Transmita grandes massas de dados de ferramentas de IA utilizando decoradores B-FAST, reduzindo o consumo de tokens em até 85%:

```python
from b_fast.fastmcp import FastMCPBFast, bfast_tool

mcp = FastMCPBFast("Servidor de Analytics")

@mcp.tool()
@bfast_tool(compress=True)
def consulta_dataset(limite: int = 1000) -> list[dict]:
    return [{"id": i, "valor": i * 1.5} for i in range(limite)]
```

---

## Próximos Passos

- [Guia de Integrações](integrations.md) - Django Ninja, Django, Polars e Pandas
- [Integração Frontend](frontend.md) - Cliente TypeScript, TanStack Query e Zod
- [Guia para IAs & LLMs (`llms.txt`)](ai.md) - Instruções para OpenCode, Cursor e Claude Code
- [Performance & Benchmarks](performance.md) - Benchmarks técnicos vs orjson e JSON
- [Solução de Problemas](troubleshooting.md) - Resolução de dúvidas frequentes
