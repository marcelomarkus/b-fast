# 📑 Referência da API

Referência técnica completa para todas as classes, métodos e funções do ecossistema B-FAST.

---

## 🧭 Visão Geral

<div class="grid cards" markdown>

-   __⚡ Módulo Principal (`b_fast`)__

    ---

    [`BFast`](#classe-bfast) e [`BFastError`](#bfasterror-excecao). Motor Rust com SIMD para serialização e deserialização empacotada de alto desempenho.

-   __🚀 Integração Web (`b_fast.integration`)__

    ---

    [`BFastResponse`](#bfastresponse) e [`BFastStreamingResponse`](#bfaststreamingresponse) para FastAPI e Starlette.

-   __🤖 MCP (`b_fast.mcp`)__

    ---

    Funções utilitárias para o transporte Streamable HTTP do Model Context Protocol (MCP).

-   __💻 TypeScript (`bfast-client`)__

    ---

    [`BFastDecoder`](#bfastdecoder-typescript) e [`Utilitários de Streaming`](#utilitarios-de-streaming-typescript) para navegador e Node.js.

</div>

---

## ⚡ Módulo Principal

### Classe `BFast`

O motor central de serialização e deserialização implementado em Rust com PyO3.

```python
from b_fast import BFast

encoder = BFast()
```

#### Métodos

##### `encode_packed(obj: Any, compress: bool = True) -> bytes`
Serializa qualquer objeto Python (primitivos, dicionários, listas, modelos Pydantic, arrays NumPy) no formato binário B-FAST.

- **Parâmetros:**
    - `obj`: Objeto a ser serializado. Suporta modelos Pydantic v2, NumPy ndarrays, datetime, UUID, Decimal, dicts, lists e tipos primitivos.
    - `compress` *(bool, padrão=True)*: Ativa a compressão LZ4. Payloads maiores que 1MB utilizam automaticamente compressão paralela multi-thread.
- **Retorno:** `bytes` com o payload binário empacotado.
- **Exceções:** Dispara `BFastError` se um tipo não suportado for encontrado.

##### `decode_packed(data: Union[bytes, bytearray, memoryview]) -> Any`
Deserializa um payload binário B-FAST de volta para estruturas nativas do Python.

- **Parâmetros:**
    - `data`: Buffer binário a ser decodificado.
- **Retorno:** Objeto Python nativo (dict, list, primitivo, array NumPy, datetime, etc.).
- **Exceções:** Dispara `BFastError` caso os dados estejam corrompidos ou o checksum falhe.

---

### `BFastError` (Exceção)

Disparada quando a serialização ou deserialização falha. É uma subclasse de `ValueError`.

---



## 🚀 Módulo de Integração Web (`b_fast.integration`)

### `BFastResponse`

Resposta pronta para FastAPI/Starlette que serializa o conteúdo diretamente em B-FAST.

```python
from b_fast import BFastResponse

@app.get("/dados")
def get_dados():
    return BFastResponse(meus_dados, compress=True)
```

- **Content-Type:** `application/x-bfast`
- **Parâmetros:**
    - `content`: Dados a serem serializados.
    - `compress` *(bool, padrão=True)*: Se aplica compressão LZ4.
    - `status_code` *(int, padrão=200)*: Código de status HTTP.
    - `headers` *(dict, opcional)*: Cabeçalhos HTTP adicionais.

---

### `BFastStreamingResponse`

Resposta de streaming para FastAPI/Starlette para envio contínuo de dados chunked.

```python
from b_fast import BFastStreamingResponse

@app.get("/stream")
async def stream():
    async def event_generator():
        for i in range(10):
            yield {"contador": i}
    return BFastStreamingResponse(event_generator())
```

- **Content-Type:** `application/x-bfast-stream`
- **Parâmetros:**
    - `content`: `Generator` ou `AsyncGenerator` gerando objetos Python.
    - `compress` *(bool, padrão=False)*: Comprime frames individuais.
    - `status_code` *(int, padrão=200)*: Código de status HTTP.

---

## 🤖 Módulo MCP (`b_fast.mcp`)

### Funções

##### `is_bfast_stream_requested(request: Request) -> bool`
Verifica se a requisição HTTP contém `Accept: application/x-bfast-stream`.

##### `wrap_mcp_tool_output(result: Any, request: Optional[Request] = None, compress: bool = False)`
Empacota o resultado da ferramenta em `BFastResponse` caso solicitado pelo cliente, ou retorna o dicionário padrão.

##### `stream_mcp_tool_results(results_generator: Iterable[Any], request: Optional[Request] = None, compress: bool = False)`
Transmite resultados síncronos de ferramentas MCP via `BFastStreamingResponse`.

##### `stream_mcp_async_tool_results(results_async_generator: AsyncIterable[Any], request: Optional[Request] = None, compress: bool = False)`
Transmite resultados assíncronos de ferramentas MCP via `BFastStreamingResponse`.

---

## 💻 Cliente TypeScript (`bfast-client`)

```bash
npm install bfast-client
```

### `BFastDecoder` (TypeScript)

```typescript
import { BFastDecoder } from "bfast-client";

const dados = BFastDecoder.decode(arrayBuffer);
```

#### `BFastDecoder.decode(buffer: ArrayBuffer | Uint8Array): any`
Decodifica um payload binário B-FAST padrão (detecta e descomprime automaticamente LZ4 se presente).

### Utilitários de Streaming (TypeScript)

#### `decodeReadableStream(stream: ReadableStream<Uint8Array>, options?: StreamDecodeOptions): AsyncGenerator<any>`
Função auxiliar que consome uma `ReadableStream` da Fetch API e gera objetos decodificados sob demanda. Gerencia automaticamente a remontagem de chunks e o encerramento do stream.

```typescript
for await (const item of decodeReadableStream(response.body)) {
    console.log(item);
}
```

#### `decodeNodeStream(stream: NodeJS.ReadableStream, options?: StreamDecodeOptions): AsyncGenerator<any>`
Função auxiliar que consome uma stream legível do Node.js e gera objetos decodificados.
