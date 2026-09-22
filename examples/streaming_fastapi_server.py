"""
Exemplo de Servidor FastAPI com B-FAST Streamable HTTP.

Executar com:
    uvicorn examples.streaming_fastapi_server:app --reload --port 8000
"""

import asyncio
import uuid
from datetime import datetime, timezone
from typing import AsyncGenerator, Generator

import numpy as np
from fastapi import FastAPI, Request
from pydantic import BaseModel

from b_fast import BFastStreamingResponse, is_bfast_stream_requested

app = FastAPI(title="B-FAST Streamable HTTP Demo")


class SensorReading(BaseModel):
    sensor_id: str
    temperature: float
    humidity: float
    timestamp: datetime


# -------------------------------------------------------------------
# 1. Streaming Assíncrono em Tempo Real (ex: IoT, Telemetria, LLM Tokens)
# -------------------------------------------------------------------
@app.get("/telemetry/stream")
async def stream_telemetry() -> BFastStreamingResponse:
    """Emite leituras de sensores em tempo real com intervalo assíncrono."""

    async def generate_readings() -> AsyncGenerator[SensorReading, None]:
        for i in range(20):
            await asyncio.sleep(0.05)  # Simula I/O assíncrono
            yield SensorReading(
                sensor_id=f"sensor_{i % 3}",
                temperature=20.0 + (i * 0.4),
                humidity=55.0 + (i * 0.2),
                timestamp=datetime.now(timezone.utc),
            )

    return BFastStreamingResponse(generate_readings())


# -------------------------------------------------------------------
# 2. Streaming Síncrono de Lotes de Banco de Dados (50.000 registros)
# -------------------------------------------------------------------
@app.get("/database/export")
def stream_database_cursor() -> BFastStreamingResponse:
    """Emite grandes volumes de dados em lotes (batching) sem estourar a RAM do servidor."""

    def fetch_db_batches() -> Generator[list[dict], None, None]:
        # Simula leitura de cursor de banco de dados em lotes de 1.000 itens
        for batch_num in range(10):
            batch = [
                {
                    "user_id": str(uuid.uuid4()),
                    "account_number": 100000 + (batch_num * 1000) + j,
                    "balance": 1500.50 + j,
                    "active": True,
                }
                for j in range(1000)
            ]
            yield batch

    return BFastStreamingResponse(fetch_db_batches(), compress=True)


# -------------------------------------------------------------------
# 3. Streaming Zero-Copy de Matrizes e Tensores NumPy (IA / Machine Learning)
# -------------------------------------------------------------------
@app.get("/ml/embeddings")
def stream_embeddings() -> BFastStreamingResponse:
    """Emite blocos de vetores numéricos de alta dimensão sem converter para Base64."""

    def generate_embeddings() -> Generator[dict, None, None]:
        for layer in range(5):
            # Gera matriz de embeddings simulada (ex: 128 vetores de 64 dimensões)
            weights = np.random.randn(128, 64).astype(np.float32)
            yield {
                "layer_index": layer,
                "layer_name": f"transformer_block_{layer}",
                "weights": weights,
            }

    return BFastStreamingResponse(generate_embeddings(), compress=True)


# -------------------------------------------------------------------
# 4. Endpoint MCP (Model Context Protocol) com Negociação de Conteúdo
# -------------------------------------------------------------------
@app.post("/mcp/tools/execute")
async def execute_mcp_tool(request: Request):
    """Demonstração de integração com o transporte Streamable HTTP do MCP."""
    headers = request.headers

    # Se o client do agente MCP solicitar B-FAST, aceleramos a entrega
    if is_bfast_stream_requested(headers):

        async def tool_stream():
            for step in range(5):
                await asyncio.sleep(0.02)
                yield {
                    "tool": "code_search",
                    "step": step,
                    "matches": [f"src/file_{step}_{k}.py" for k in range(50)],
                }

        return BFastStreamingResponse(tool_stream())

    # Fallback tradicional para JSON padrão se o client não suportar B-FAST
    return {"status": "fallback_json", "message": "Standard JSON response"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="127.0.0.1", port=8000)
