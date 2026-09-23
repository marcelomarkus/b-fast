# Integrações de Frameworks e Data Science

O B-FAST oferece integrações nativas e descomplicadas para os principais frameworks web e ecossistemas de Data Science em Python.

---

## 🥷 Django Ninja & Django

### Django Ninja (`BFastRenderer`)

O [Django Ninja](https://django-ninja.dev/) é o framework de APIs de maior crescimento no ecossistema Django. Com o `BFastRenderer`, qualquer API ou roteador do Django Ninja pode servir respostas binárias B-FAST sem código repetitivo:

```python
from b_fast.django import BFastRenderer
from ninja import NinjaAPI

# Aplica o renderizador globalmente na API
api = NinjaAPI(renderer=BFastRenderer())


@api.get("/users")
def get_users(request):
    # Serializado automaticamente em binário B-FAST comprimido
    return [
        {"id": 1, "name": "Alice", "role": "admin"},
        {"id": 2, "name": "Bob", "role": "member"},
    ]
```

Você também pode aplicar o `BFastRenderer` em rotas ou operações específicas:

```python
@api.get("/telemetria", renderer=BFastRenderer())
def get_telemetria(request):
    return {"sensores": [10.5, 20.3, 15.8]}
```

### Django Tradicional (`BFastHttpResponse` & `BFastStreamingHttpResponse`)

Para views convencionais do Django (baseadas em função, classes ou Django REST framework):

```python
from b_fast.django import BFastHttpResponse, BFastStreamingHttpResponse
from django.http import HttpRequest


def user_view(request: HttpRequest):
    data = {"status": "ok", "users": ["Alice", "Bob"]}
    return BFastHttpResponse(data)


def stream_view(request: HttpRequest):
    def event_generator():
        for i in range(100):
            yield {"step": i, "temperatura": 20.0 + i * 0.1}

    return BFastStreamingHttpResponse(event_generator())
```

---

## 📊 Data Science: Polars, Pandas & PyArrow

O B-FAST oferece serialização direta e eficiente para DataFrames, Series e tabelas PyArrow.

### Serialização Nativa em `BFast.encode_packed`

Você pode passar DataFrames do Polars, Pandas ou Tabelas Arrow diretamente para `BFast.encode_packed()`:

```python
from b_fast import BFast
import polars as pl
import pandas as pd

bf = BFast()

# 1. Polars DataFrame
df_pl = pl.DataFrame({"id": [1, 2, 3], "name": ["Alice", "Bob", "Charlie"]})
bytes_pl = bf.encode_packed(df_pl, compress=True)

# 2. Pandas DataFrame
df_pd = pd.DataFrame({"id": [1, 2], "score": [98.5, 91.0]})
bytes_pd = bf.encode_packed(df_pd, compress=True)

# 3. Aninhado dentro de dicionários ou respostas
payload = {
    "status": "success",
    "total": len(df_pl),
    "records": df_pl,  # Serializado automaticamente como registros!
}
bytes_nested = bf.encode_packed(payload, compress=True)
```

No TypeScript, `BFastDecoder.decode(bytes)` recebe imediatamente um array de objetos `[{ id: 1, name: "Alice" }, ...]`, pronto para ser consumido por TanStack Table, AG Grid ou gráficos sem necessidade de `.to_dict(orient="records")` manual no backend!

### Helpers Especializados: `encode_dataframe` & `decode_dataframe`

Para pipelines de dados e microsserviços Python-to-Python de altíssima performance, as funções `encode_dataframe` e `decode_dataframe` dão controle total sobre a orientação dos dados:

```python
from b_fast import encode_dataframe, decode_dataframe
import polars as pl

df = pl.DataFrame({"id": [1, 2, 3], "cidade": ["SP", "RJ", "BH"]})

# 1. Orientação 'records': lista de dicionários de linha (ideal para APIs e Frontend)
data_records = encode_dataframe(df, orient="records")

# 2. Orientação 'columns': dicionário colunar {coluna: [valores]} (ultra-rápido, zero cópias desnecessárias)
data_columns = encode_dataframe(df, orient="columns")

# 3. Orientação 'split': {'columns': [...], 'data': [[...], ...]}
data_split = encode_dataframe(df, orient="split")

# Reconstrução em DataFrames
df_reconst = decode_dataframe(data_columns, engine="polars")  # ou "pandas", "arrow", "auto"
```

### Orientações e Casos de Uso

| Orientação | Estrutura Python | Caso de Uso Ideal |
| :--- | :--- | :--- |
| `records` (padrão) | `[ {col1: val1, col2: val2}, ... ]` | APIs REST, TanStack Table, React, UIs no navegador |
| `columns` | `{ col1: [val1, ...], col2: [val2, ...] }` | Big Data, telemetria, microsserviços, gráficos em tempo real |
| `split` | `{ "columns": [...], "data": [[...], ...] }` | Exportações de banco de dados, compatibilidade com Pandas |

---

## ⚡ FastAPI & Starlette

Para FastAPI e Starlette, utilize `BFastResponse` e `BFastStreamingResponse`:

```python
from fastapi import FastAPI
from b_fast import BFastResponse, BFastStreamingResponse

app = FastAPI()


@app.get("/items", response_class=BFastResponse)
def get_items():
    return [{"id": 1, "value": "A"}, {"id": 2, "value": "B"}]


@app.get("/stream", response_class=BFastStreamingResponse)
async def stream_items():
    async def event_generator():
        for i in range(10):
            yield {"item": i}

    return event_generator()
```
