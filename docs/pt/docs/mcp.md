# Integração Model Context Protocol (MCP) & FastMCP

⚡ **B-FAST** oferece integração nativa e de alta performance com o **FastMCP** e **MCPServer** (SDK oficial do Model Context Protocol da Anthropic).

Por padrão, o MCP transmite os retornos de ferramentas via JSON-RPC em STDIO, SSE ou HTTP. Quando as ferramentas retornam grandes conjuntos de dados (DataFrames, consultas SQL, logs de telemetria, embeddings vetoriais ou arrays NumPy), o JSON-RPC tradicional causa alta latência, sobrecarga de CPU e inflação de tokens.

Com o B-FAST, os retornos das ferramentas são serializados **até 15x mais rápido**, com tamanho **até 80% menor** com LZ4, e empacotados como recursos nativos `EmbeddedResource` (`BlobResourceContents`) do MCP.

---

## Instalação

Instale o B-FAST com suporte opcional ao FastMCP:

```bash
pip install "bfast-py[fastmcp]"
```

Ou diretamente:

```bash
pip install bfast-py fastmcp
```

---

## Integração com FastMCP

### 1. Retorno de Tools em Alta Velocidade (`@bfast_tool`)

Use o decorator `@bfast_tool` para serializar automaticamente o resultado de ferramentas em binário B-FAST. Funciona com funções síncronas e assíncronas:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import bfast_tool

mcp = FastMCP("AnalyticsServer")


# Método 1: Combinado com @mcp.tool()
@mcp.tool()
@bfast_tool(compress=True)
def query_sensor_metrics(sensor_id: str, count: int = 1000) -> dict:
    """Busca telemetria de sensores de alta frequência."""
    return {
        "sensor": sensor_id,
        "timestamps": [1700000000 + i for i in range(count)],
        "readings": [20.5 + (i % 5) for i in range(count)],
    }


# Método 2: Passando o servidor diretamente ao @bfast_tool
@bfast_tool(mcp, compress=True, description="Consulta banco de usuários")
async def get_users(role: str) -> list:
    return [
        {"id": 1, "name": "Alice", "role": role},
        {"id": 2, "name": "Bob", "role": role},
    ]
```

#### O que o Agente ou Cliente MCP Recebe:
A ferramenta devolve:
1. `TextContent`: Um resumo legível para o LLM (ex.: `[B-FAST binary payload (1000 items): 4200 bytes (compressed with LZ4) available in embedded resource 'bfast://...']`).
2. `EmbeddedResource`: Objeto `BlobResourceContents` com `mime_type="application/x-bfast"` contendo os bytes binários compactados codificados em base64.

---

### 2. Recursos Binários MCP (`@bfast_resource`)

Exponha snapshots de dados como recursos MCP com MIME type nativo `application/x-bfast`:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import bfast_resource

mcp = FastMCP("DataServer")


@bfast_resource(mcp, "bfast://models/weights", compress=True)
def get_weights() -> list:
    return [0.125, 0.456, 0.789, 1.024]
```

---

### 3. Adaptador Completo (`FastMCPBFast`)

Instancie ou empacote qualquer servidor com métodos B-FAST de primeira classe:

```python
from b_fast.fastmcp import FastMCPBFast

mcp = FastMCPBFast("EnterpriseService")


@mcp.bfast_tool(compress=True)
def process_data(batch_id: int):
    return {"status": "success", "batch": batch_id}


@mcp.bfast_resource("bfast://cluster/status")
def cluster_status():
    return {"nodes": 12, "healthy": True}
```

---

## Decodificação no Cliente TypeScript (`bfast-client`)

Quando o seu agente, frontend ou extensão recebe o resultado da ferramenta do servidor MCP, a decodificação é feita em uma única linha:

```typescript
import { decodeMcpResource } from 'bfast-client';

// Passa o CallToolResult retornado pelo cliente MCP
const result = await mcpClient.callTool({ name: 'query_sensor_metrics', arguments: { sensor_id: 'S-1' } });

// Decodifica o recurso B-FAST automaticamente (com aceleração nativa WebAssembly LZ4)
const data = decodeMcpResource(result);
console.log(data.sensor);    // 'S-1'
console.log(data.readings);  // [20.5, 21.5, ...]
```

---

## Decodificação em Python (`decode_mcp_resource`)

Clientes em Python também decodificam o resultado diretamente:

```python
from b_fast.fastmcp import decode_mcp_resource

# Aceita CallToolResult, EmbeddedResource, BlobResourceContents ou string base64
data = decode_mcp_resource(tool_result)
print(data["sensor"])
```

---

## Streaming de Resultados MCP

Para ferramentas que produzem fluxos contínuos de dados:

```python
from fastmcp import FastMCP
from b_fast.fastmcp import stream_mcp_async_tool_results

mcp = FastMCP("StreamServer")


@mcp.tool()
async def stream_large_dataset():
    async def data_generator():
        for batch_id in range(50):
            yield {"batch": batch_id, "data": [1, 2, 3]}

    return stream_mcp_async_tool_results(data_generator(), compress=True)
```
