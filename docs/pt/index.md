# ⚡ B-FAST (Binary Fast Adaptive Serialization Transfer)

O B-FAST é um protocolo de serialização binária de ultra-alta performance, desenvolvido em Rust para o ecossistema Python e TypeScript. Ele foi projetado para substituir o JSON em rotas críticas onde latência, uso de CPU e largura de banda são gargalos.

## 🚀 Por que B-FAST?

- **Motor Rust:** Serialização nativa sem o overhead do interpretador Python
- **Pydantic Native:** Lê atributos de modelos Pydantic diretamente da memória, pulando o lento processo de .model_dump()
- **Zero-Copy NumPy:** Serializa tensores e arrays numéricos diretamente, atingindo a velocidade máxima de I/O de memória
- **String Interning:** Chaves repetidas (como nomes de campos em listas de objetos) são enviadas apenas uma vez
- **Bit-Packing:** Inteiros pequenos e booleanos ocupam apenas 4 bits dentro da tag de tipo
- **LZ4 Integrado:** Compressão de blocos ultra-veloz para payloads grandes

## 📊 Performance

Comparação de serialização de uma lista de 10.000 modelos Pydantic complexos:

### 🚀 Serialização (Encode)
| Formato | Tempo (ms) | Speedup | Tamanho do Payload | Redução |
|---------|------------|---------|-------------------|---------|
| JSON (Standard) | 9.64ms | 1.0x | 1.18 MB | 0% |
| orjson | 1.51ms | 6.4x | 1.06 MB | 10.2% |
| Pickle | 2.74ms | 3.5x | 808 KB | 31.6% |
| **B-FAST** | **4.51ms** | **2.1x** | **998 KB** | **15.5%** |
| **B-FAST + LZ4** | **5.21ms** | **1.9x** | **252 KB** | **78.7%** |

### 🔄 Round-Trip (Encode + Network + Decode)

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

## 🎯 Casos de Uso Ideais

- **📱 Mobile/IoT**: 78.7% economia de dados + 2.1x performance
- **🌐 APIs com rede lenta**: Até 4x mais rápido que JSON
- **📊 Data pipelines**: 148x speedup para NumPy arrays
- **🗜️ Storage/Cache**: Compressão superior integrada

## 📦 Instalação

### Backend (Python)
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
