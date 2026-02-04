## ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

O B-FAST é um protocolo de serialização binária de ultra-alta performance, desenvolvido em Rust para o ecossistema Python e TypeScript. Ele foi projetado para substituir o JSON em rotas críticas onde latência, uso de CPU e largura de banda são gargalos.


## 🚀 Por que B-FAST?
- **Motor Rust:** Serialização nativa sem o overhead do interpretador Python.
- **Pydantic Native:** Lê atributos de modelos Pydantic diretamente da memória, pulando o lento processo de .model_dump().
- **Zero-Copy NumPy:** Serializa tensores e arrays numéricos diretamente, atingindo a velocidade máxima de I/O de memória.
- **String Interning:** Chaves repetidas (como nomes de campos em listas de objetos) são enviadas apenas uma vez.
- **Bit-Packing:** Inteiros pequenos e booleanos ocupam apenas 4 bits dentro da tag de tipo.
- **LZ4 Integrado:** Compressão de blocos ultra-veloz para payloads grandes.

## 📊 Benchmark (Latência Média)
Comparação de serialização de uma lista de 10.000 modelos Pydantic complexos:

| Formato | Tempo (ms) | Tamanho do Payload |
|---------|------------|-------------------|
| JSON (Standard) | 45.2ms | 1.2 MB |
| Orjson | 12.8ms | 1.1 MB |
| B-FAST | 1.4ms | 240 KB |

## 📦 Instalação

### Backend (Python)
```bash
uv add b-fast
```
ou
```bash
pip install b-fast
```

### Frontend (TypeScript)
```bash
npm install @b-fast/client
```

## 🛠️ Como usar

### 1. No FastAPI (Integração Direta)
O B-FAST se integra perfeitamente como uma classe de resposta.

```python
from fastapi import FastAPI, Response
from pydantic import BaseModel
import b_fast

class BFastResponse(Response):
    media_type = "application/x-bfast"
    
    def __init__(self, content=None, *args, **kwargs):
        super().__init__(content, *args, **kwargs)
        self.encoder = b_fast.BFast()

    def render(self, content) -> bytes:
        return self.encoder.encode_packed(content, compress=True)

app = FastAPI()

class User(BaseModel):
    id: int
    name: str

@app.get("/users", response_class=BFastResponse)
async def get_users():
    return [User(id=i, name=f"User {i}") for i in range(1000)]
```

### 2. No Frontend (React / Vue / Angular)
```typescript
import { BFastDecoder } from '@b-fast/client';

async function loadData() {
    const response = await fetch('/users');
    const buffer = await response.arrayBuffer();
    
    // Decodifica e descomprime LZ4 automaticamente
    const users = BFastDecoder.decode(buffer);
    console.log(users);
}
```

## About B-FAST

> "Knowledge is the only wealth that grows when we share it"

B-FAST was born from the belief that high-performance tools should be accessible to everyone. This project represents our commitment to open-source innovation and the sharing of knowledge that drives the developer community forward.

**Developed by:** [marcelomarkus](https://github.com/marcelomarkus)

**Philosophy:** We believe that by sharing efficient solutions, we collectively raise the bar for what's possible in software development. B-FAST is our contribution to a faster, more efficient web.

## 📄 Licença
Distribuído sob a licença MIT. Veja LICENSE para mais informações.