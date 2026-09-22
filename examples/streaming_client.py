"""
Exemplo de Cliente Python consumindo B-FAST Streamable HTTP com httpx.

Requer:
    pip install httpx bfast-py
"""

import asyncio

import httpx

from b_fast import BFastStreamDecoder


async def consume_telemetry_stream():
    """Consome o stream assíncrono do servidor FastAPI em tempo real."""
    url = "http://127.0.0.1:8000/telemetry/stream"
    decoder = BFastStreamDecoder()

    print("📡 Conectando ao endpoint B-FAST Streamable HTTP...")

    async with httpx.AsyncClient() as client:
        async with client.stream("GET", url) as response:
            print(
                f"Status: {response.status_code}, Content-Type: {response.headers.get('content-type')}"
            )

            # Consome os chunks binários da rede à medida que chegam
            async for chunk in response.aiter_bytes():
                items = decoder.feed(chunk)
                for item in items:
                    print(
                        f"Recebido em tempo real -> Sensor: {item['sensor_id']}, Temp: {item['temperature']:.1f}°C, Time: {item['timestamp']}"
                    )

                if decoder.is_eos:
                    print("Fim do stream alcançado (EOS).")
                    break


if __name__ == "__main__":
    try:
        asyncio.run(consume_telemetry_stream())
    except httpx.ConnectError:
        print("Certifique-se de que o servidor está rodando:")
        print("python examples/streaming_fastapi_server.py")
