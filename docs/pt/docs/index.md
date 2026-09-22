# ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

O B-FAST é um protocolo de serialização binária de ultra-alta performance, desenvolvido em Rust para o ecossistema Python e TypeScript. Ele foi projetado para substituir o JSON em rotas críticas onde latência, uso de CPU e largura de banda são gargalos.

> "Performance não é apenas sobre velocidade—é sobre eficiência onde mais importa"

O B-FAST nasceu do reconhecimento de que aplicações modernas precisam de mais do que apenas serialização rápida—elas precisam de **serialização inteligente** que se adapta às restrições do mundo real. Após extensa otimização, o B-FAST encontrou seu nicho perfeito em ambientes com restrição de largura de banda, alcançando **1.7x mais rápido** que orjson para objetos simples e **5.7x mais rápido** em redes lentas.

**Filosofia:** Acreditamos que o futuro da transferência de dados não está apenas na velocidade bruta da CPU, mas em protocolos inteligentes que minimizam o overhead de rede mantendo excelente performance. O B-FAST representa nossa contribuição para uma web mais eficiente e consciente da largura de banda.

## 🚀 Por que B-FAST?

- **Motor Rust:** Serialização nativa sem o overhead do interpretador Python
- **Streamable HTTP:** Streaming binário contínuo (`application/x-bfast-stream`) com framing delimitado por comprimento e zero-copy
- **Model Context Protocol (MCP):** Transporte nativo Streamable HTTP para ferramentas de agentes de IA
- **Pydantic Native:** Lê atributos de modelos Pydantic diretamente da memória, pulando o lento processo de .model_dump()
- **Zero-Copy NumPy:** Serializa tensores e arrays numéricos diretamente, atingindo 14-96x speedup vs JSON/orjson
- **Compressão Paralela:** LZ4 com processamento multi-thread para payloads grandes (>1MB)
- **Otimizado para Cache:** Alocação alinhada e processamento em lote para máxima eficiência

## 📊 Performance

### 🚀 Objetos Simples (10.000)
| Formato | Tempo (ms) | Speedup |
|---------|------------|---------|
| JSON | 12.0ms | 1.0x |
| orjson | 8.19ms | 1.5x |
| **B-FAST** | **4.83ms** | **🚀 2.5x** |

**B-FAST é 1.7x mais rápido que orjson!**

### 🔄 Round-Trip (Encode + Rede + Decode)

#### 📡 100 Mbps (Rede Lenta)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 114.5ms | 0.8x |
| orjson | 91.7ms | 1.0x |
| **B-FAST + LZ4** | **16.1ms** | **🚀 5.7x** |

#### 📡 1 Gbps (Rede Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 29.4ms | 0.5x |
| orjson | 15.3ms | 1.0x |
| **B-FAST + LZ4** | **7.2ms** | **🚀 2.1x** |

#### 📡 10 Gbps (Rede Ultra-Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 20.9ms | 0.4x |
| orjson | 7.7ms | 1.0x |
| **B-FAST + LZ4** | **6.3ms** | **🚀 1.2x** |

## 🎯 Casos de Uso Ideais

- **📱 Mobile/IoT**: 89% economia de dados + 5.7x performance em redes lentas
- **🌐 APIs com redes lentas**: Até 5.7x mais rápido que orjson
- **📊 Data pipelines**: 14-96x speedup para arrays NumPy
- **🗜️ Storage/Cache**: Compressão superior integrada
- **🚀 Objetos simples**: 1.7x mais rápido que orjson

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

## 🛠️ Uso Básico

### Python
```python
import b_fast
from pydantic import BaseModel

class User(BaseModel):
    id: int
    name: str
    email: str

# Criar encoder
bf = b_fast.BFast()

# Dados de exemplo
users = [User(id=i, name=f"User {i}", email=f"user{i}@example.com") for i in range(1000)]

# Serializar
data = bf.encode_packed(users, compress=True)
print(f"Tamanho: {len(data)} bytes")

# Deserializar
decoded = bf.decode_packed(data)
```

### TypeScript
```typescript
import { BFastDecoder } from 'bfast-client';

async function loadData() {
    const response = await fetch('/api/users');
    const buffer = await response.arrayBuffer();
    
    // Decodifica e descomprime automaticamente
    const users = BFastDecoder.decode(buffer);
    console.log(users);
}
```

## 🔗 Links Úteis

- [Começando](getting_started.md) - Tutorial completo
- [Frontend](frontend.md) - Integração TypeScript
- [Performance](performance.md) - Análise técnica detalhada
- [Solução de Problemas](troubleshooting.md) - Guia de troubleshooting

## 📄 Licença

Distribuído sob a licença MIT. Veja [LICENSE](https://github.com/marcelomarkus/b-fast/blob/main/LICENSE) para mais informações.
