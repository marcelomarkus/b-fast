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

### Integração com Model Context Protocol (MCP) 🤖
Transmita grandes massas de dados de ferramentas de IA utilizando o B-FAST Streamable HTTP:

```python
from b_fast import (
    is_bfast_stream_requested,
    stream_mcp_async_tool_results,
    wrap_mcp_tool_output,
)

# Verifica a negociação de conteúdo do cliente MCP
if is_bfast_stream_requested(request.headers):
    # Transmite o gerador diretamente em binário
    return stream_mcp_async_tool_results(tool_data_generator())
```

## Próximos Passos

- [Integração Frontend](frontend.md) - Configuração do cliente TypeScript & streaming
- [Performance](performance.md) - Benchmarks detalhados
- [Solução de Problemas](troubleshooting.md) - Problemas comuns
