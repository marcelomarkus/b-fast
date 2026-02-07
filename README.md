## ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

O B-FAST é um protocolo de serialização binária de ultra-alta performance, desenvolvido em Rust para o ecossistema Python e TypeScript. Ele foi projetado para substituir o JSON em rotas críticas onde latência, uso de CPU e largura de banda são gargalos.

## 📚 Documentação
Documentação completa disponível em: **https://marcelomarkus.github.io/b-fast/**


## 🚀 Por que B-FAST?
- **Motor Rust:** Serialização nativa sem o overhead do interpretador Python.
- **Pydantic Native:** Lê atributos de modelos Pydantic diretamente da memória, pulando o lento processo de .model_dump().
- **Zero-Copy NumPy:** Serializa tensores e arrays numéricos diretamente, atingindo a velocidade máxima de I/O de memória.
- **Compressão Paralela:** LZ4 com processamento multi-thread para payloads grandes (>1MB).
- **Bit-Packing:** Inteiros pequenos e booleanos ocupam apenas 4 bits dentro da tag de tipo.
- **Otimizado para Cache:** Alocação alinhada e processamento em batch para máxima eficiência.

## 📊 Benchmark (Latência Média)
Comparação de serialização de uma lista de 10.000 modelos Pydantic complexos:

### 🚀 Serialização (Encode)
| Formato | Tempo (ms) | Speedup | Tamanho do Payload | Redução |
|---------|------------|---------|-------------------|---------|
| JSON (Standard) | 10.14ms | 1.0x | 1.18 MB | 0% |
| orjson | 1.55ms | 6.6x | 1.06 MB | 10% |
| Pickle | 2.73ms | 3.7x | 808 KB | 32% |
| **B-FAST** | **4.67ms** | **2.2x** | **998 KB** | **15%** |
| **B-FAST + LZ4** | **5.27ms** | **1.9x** | **252 KB** | **79%** |

### 🔄 Round-Trip (Encode + Network + Decode)
Teste completo incluindo transferência de rede e deserialização:

#### 📡 100 Mbps (Rede Lenta)
| Formato | Tempo Total | Speedup vs JSON |
|---------|-------------|-----------------|
| JSON | 114.3ms | 1.0x |
| orjson | 92.3ms | 1.2x |
| **B-FAST + LZ4** | **28.3ms** | **🚀 4.0x** |

#### 📡 1 Gbps (Rede Rápida)
| Formato | Tempo Total | Speedup vs JSON |
|---------|-------------|-----------------|
| JSON | 29.3ms | 1.0x |
| orjson | 15.9ms | 1.8x |
| **B-FAST + LZ4** | **10.2ms** | **🚀 2.9x** |

#### 📡 10 Gbps (Rede Ultra-Rápida)
| Formato | Tempo Total | Speedup vs JSON |
|---------|-------------|-----------------|
| JSON | 20.8ms | 1.0x |
| orjson | 8.3ms | 2.5x |
| **B-FAST + LZ4** | **8.4ms** | **🚀 2.5x** |

### 🎯 Casos de Uso Ideais
- **📱 Mobile/IoT**: 79% economia de dados + 2.2x performance
- **🌐 APIs com rede lenta**: Até 4x mais rápido que JSON
- **📊 Data pipelines**: 148x speedup para NumPy arrays
- **🗜️ Storage/Cache**: Compressão superior integrada

## 📦 Instalação

### Backend (Python)
```bash
uv add bfast-py
```
ou
```bash
pip install bfast-py
```

### Frontend (TypeScript)
```bash
npm install bfast-client
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
import { BFastDecoder } from 'bfast-client';

async function loadData() {
    const response = await fetch('/users');
    const buffer = await response.arrayBuffer();
    
    // Decodifica e descomprime LZ4 automaticamente
    const users = BFastDecoder.decode(buffer);
    console.log(users);
}
```

## About B-FAST

> "Performance is not just about speed—it's about efficiency where it matters most"

B-FAST was born from the recognition that modern applications need more than just fast serialization—they need **smart serialization** that adapts to real-world constraints. After extensive optimization achieving **2.2x faster serialization** and **79% payload reduction**, B-FAST has found its perfect niche in bandwidth-constrained environments.

**Key Achievements:**
- 🚀 **4.0x faster** than JSON on 100 Mbps networks (round-trip)
- 📦 **79% smaller** payloads with built-in LZ4 compression
- ⚡ **148x speedup** for NumPy arrays
- 🎯 **Competitive** even on ultra-fast 10 Gbps networks

**Developed by:** [marcelomarkus](https://github.com/marcelomarkus)

**Philosophy:** We believe that the future of data transfer lies not in raw CPU speed alone, but in intelligent protocols that minimize network overhead while maintaining excellent performance. B-FAST represents our contribution to a more efficient, bandwidth-conscious web.

## 📄 Licença
Distribuído sob a licença MIT. Veja LICENSE para mais informações.