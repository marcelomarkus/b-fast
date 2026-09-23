# 📑 Referência da API

Referência técnica completa para todas as classes, métodos e funções do ecossistema B-FAST em Python e TypeScript.

---

## 🧭 Visão Geral

<div class="grid cards" markdown>

-   __⚡ Módulo Principal (`b_fast`)__

    ---

    [`BFast`](#classe-bfast) e [`BFastError`](#excecao-bfasterror). Serialização e decodificação binária de alta vazão com núcleo em Rust.

-   __🚀 Frameworks Web (`b_fast.integration` & `b_fast.django`)__

    ---

    [`BFastResponse`](#bfastresponse), [`BFastStreamingResponse`](#bfaststreamingresponse) para FastAPI/Starlette, e [`BFastRenderer`](#bfastrenderer) para Django Ninja.

-   __📊 Data Science (`b_fast.data`)__

    ---

    [`encode_dataframe`](#encode_dataframe) e [`decode_dataframe`](#decode_dataframe) para serialização nativa de DataFrames Polars, Pandas e tabelas PyArrow.

-   __🤖 FastMCP 2.0 (`b_fast.fastmcp`)__

    ---

    [`FastMCPBFast`](#fastmcpbfast), [`bfast_tool`](#bfast_tool) e [`bfast_resource`](#bfast_resource) para comunicação de alta performance em ferramentas de Agentes de IA.

-   __💻 Cliente TypeScript (`bfast-client`)__

    ---

    [`bfastFetch`](#bfastfetch), [`bfastQueryOptions`](#bfastqueryoptions), [`BFastDecoder`](#bfastdecoder) e [`Utilitários de Streaming`](#utilitarios-de-streaming).

</div>

---

## ⚡ Módulo Principal (`b_fast`)

### Classe `BFast`

O motor primário de serialização e decodificação implementado em Rust com PyO3.

```python
from b_fast import BFast

bf = BFast()
```

#### Métodos

##### `encode_packed(obj: Any, compress: bool = True) -> bytes`
Serializa qualquer objeto Python (primitivos, dicionários, listas, modelos Pydantic, arrays NumPy, DataFrames Polars e Pandas) no formato binário B-FAST empacotado.

- **Parâmetros:**
    - `obj`: Objeto a ser serializado. Suporta modelos Pydantic v2, arrays NumPy, DataFrames e Series Polars/Pandas, datetime, UUID, Decimal, dicionários e listas.
    - `compress` *(bool, padrão=True)*: Habilita compressão LZ4. Cargas > 1MB utilizam automaticamente compressão em múltiplos chunks paralelos.
- **Retorno:** `bytes` contendo a carga binária empacotada.
- **Lança:** `BFastError` caso ocorra falha na serialização.

##### `decode_packed(data: Union[bytes, bytearray, memoryview]) -> Any`
Desserializa uma carga binária B-FAST de volta para estruturas nativas do Python.

- **Parâmetros:**
    - `data`: Buffer binário a ser decodificado.
- **Retorno:** Objeto nativo do Python (dict, list, primitivo, array NumPy, datetime, etc.).
- **Lança:** `BFastError` se a carga estiver malformada.

---

### Exceção `BFastError`

Lançada sempre que a serialização ou decodificação encontra dados inválidos. Herda de `ValueError`.

---

## 🚀 Integrações com Frameworks Web

### FastAPI / Starlette (`b_fast.integration`)

#### `BFastResponse`
Classe de resposta para FastAPI/Starlette que codifica os dados diretamente em binário B-FAST.

```python
from b_fast import BFastResponse

@app.get("/dados")
def get_dados():
    return BFastResponse(meus_dados, compress=True)
```

- **Content-Type:** `application/x-bfast`

#### `BFastStreamingResponse`
Resposta em streaming para FastAPI/Starlette para envio contínuo de dados em frames.

```python
from b_fast import BFastStreamingResponse

@app.get("/stream")
async def stream():
    async def gerador():
        for i in range(10):
            yield {"contagem": i}
    return BFastStreamingResponse(gerador())
```

- **Content-Type:** `application/x-bfast-stream`

---

### Django & Django Ninja (`b_fast.django`)

### Django & Django Ninja (`b_fast.django`)

### BFastRenderer
Renderizador nativo para Django Ninja baseado em `BaseRenderer`.

```python
from ninja import NinjaAPI
from b_fast.django import BFastRenderer

api = NinjaAPI(renderer=BFastRenderer())
```

- **Media Type:** `application/x-bfast`

#### `BFastHttpResponse` (Django)
Subclasse de `HttpResponse` do Django que retorna dados em binário B-FAST.

#### `BFastStreamingHttpResponse` (Django)
Subclasse de `StreamingHttpResponse` do Django para envio progressivo de frames B-FAST.

---

## 📊 Módulo de Data Science (`b_fast.data`)

### encode_dataframe
Serializa DataFrames/Series do Polars ou Pandas e Tabelas PyArrow em formato B-FAST.

```python
encode_dataframe(df: Any, orient: str = "records", compress: bool = True) -> bytes
```

- **Parâmetros:**
    - `df`: DataFrame ou Series.
    - `orient` *(str, padrão="records")*: Layout de serialização:
        - `"records"`: Lista de dicionários de linha (`[ {col: val}, ... ]`). Ideal para APIs REST e frontends.
        - `"columns"`: Dicionário colunar (`{ col: [vals] }`). Ultra-rápido, sem cópias de memória adicionais.
        - `"split"`: Dicionário com `{'columns': [...], 'data': [[...], ...]}`.
    - `compress` *(bool, padrão=True)*: Habilita compressão LZ4.

### decode_dataframe
Reconstrói DataFrames a partir de bytes binários do B-FAST.

```python
decode_dataframe(data: Union[bytes, bytearray], engine: str = "auto") -> Any
```

- **Parâmetros:**
    - `data`: Buffer binário B-FAST.
    - `engine` *(str, padrão="auto")*: Motor a ser instanciado (`"auto"`, `"polars"`, `"pandas"` ou `"arrow"`).

---

## 🤖 Módulo FastMCP 2.0 (`b_fast.fastmcp`)

### FastMCPBFast
Subclasse do servidor FastMCP com suporte pré-configurado a ferramentas e recursos binários B-FAST.

```python
FastMCPBFast(name: str, **kwargs)
```

### bfast_tool
Decorador que serializa retornos de ferramentas em blobs base64 comprimidos com B-FAST, reduzindo o consumo de tokens em até 85%.

```python
@bfast_tool(compress: bool = True)
```

### bfast_resource
Decorador para definição de recursos binários B-FAST em servidores FastMCP.

```python
@bfast_resource(uri: str, name: Optional[str] = None, compress: bool = True)
```

---

## 💻 Cliente TypeScript (`bfast-client`)

```bash
npm install bfast-client
```

### bfastFetch

```typescript
import { bfastFetch } from 'bfast-client';

const usuarios = await bfastFetch<Usuario[]>('/api/usuarios', {
    schema: UsuarioSchema, // Validação opcional com Zod ou Standard Schema
    compress: true,
});
```

### bfastQueryOptions

```typescript
import { useQuery } from '@tanstack/react-query';
import { bfastQueryOptions } from 'bfast-client';

const { data } = useQuery(
    bfastQueryOptions<Usuario>({
        queryKey: ['usuario', 1],
        url: '/api/usuarios/1',
        schema: UsuarioSchema,
        staleTime: 5000,
    })
);
```

### bfastInfiniteQueryOptions
Utilitário para paginação e carregamento contínuo no `useInfiniteQuery`.

### BFastDecoder

```typescript
import { BFastDecoder } from 'bfast-client';

const dados = BFastDecoder.decode<Usuario>(buffer, {
    typedArrays: false, // true para Float64Array com zero alocações
    schema: UsuarioSchema,
});
```

### Utilitários de Streaming

#### `decodeReadableStream<T>(stream: ReadableStream<Uint8Array>, options?: StreamDecodeOptions<T>): AsyncGenerator<T>`
Consome streams do Fetch API e itera sobre frames validados em tempo real.
