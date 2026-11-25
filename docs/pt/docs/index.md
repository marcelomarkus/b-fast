<p align="center">
  <img src="assets/bfast-black.svg#gh-light-mode-only" alt="B-FAST" width="500" />
  <img src="assets/bfast-white.svg#gh-dark-mode-only" alt="B-FAST" width="500" />
</p>

<p align="center">
  <a href="https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml"><img src="https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml/badge.svg" alt="CI"></a>
  <a href="https://marcelomarkus.github.io/b-fast/pt/"><img src="https://github.com/marcelomarkus/b-fast/actions/workflows/docs.yml/badge.svg" alt="Documentação"></a>
  <a href="https://pypi.org/project/bfast-py/"><img src="https://img.shields.io/pypi/v/bfast-py?color=blue&logo=pypi&logoColor=white" alt="Versão PyPI"></a>
  <a href="https://pypi.org/project/bfast-py/"><img src="https://img.shields.io/pypi/pyversions/bfast-py?logo=python&logoColor=white" alt="Versões Python"></a>
  <a href="https://www.npmjs.com/package/bfast-client"><img src="https://img.shields.io/npm/v/bfast-client?color=crimson&logo=npm&logoColor=white" alt="Versão npm"></a>
  <a href="https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp"><img src="https://img.shields.io/badge/Chrome%20Web%20Store-B--FAST%20Inspector-blue?logo=googlechrome&logoColor=white" alt="Chrome Web Store"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="Licença: MIT"></a>
  <a href="https://marcelomarkus.github.io/b-fast/pt/performance/"><img src="https://img.shields.io/badge/Speedup-At%C3%A9%205.7x%20vs%20orjson-brightgreen?logo=speedtest&logoColor=white" alt="Performance"></a>
  <a href="https://app.codspeed.io/marcelomarkus/b-fast?utm_source=badge"><img src="https://img.shields.io/endpoint?url=https://codspeed.io/badge.json" alt="CodSpeed"/></a>
</p>

<h3 align="center">O protocolo binário de alta performance para Python e TypeScript moderno.</h3>
<p align="center">
  <strong>Mais rápido que orjson. Mais simples que Protobuf. Inspecionável no seu DevTools.</strong>
</p>

O B-FAST é um protocolo de serialização binária de ultra-alta performance desenvolvido para substituir o JSON e formatos binários legados em rotas críticas. Oferece latência sub-microssegundo, integração nativa com Pydantic e streaming com zero boilerplate de schemas.

> *"Performance não é apenas sobre velocidade — é sobre eficiência onde mais importa."*

### ⚡ Visão Geral: O Paradigma Binário Moderno

| Capacidade | JSON / orjson | Protocol Buffers (gRPC) | ⚡ **B-FAST** |
|:---|:---:|:---:|:---:|
| **Formato no Fio** | Texto (Pesado) | Binário Compacto | **Binário + Compressão Adaptativa** |
| **Exige Schemas / Compilação?** | ❌ Não | ⚠️ Arquivos `.proto` obrigatórios | **❌ Não (Plug & Play)** |
| **Modelos Pydantic & Python** | Lento (`model_dump()`) | Mapeamento manual | **⚡ Acesso Direto de Memória** |
| **Depuração no Navegador** | ✅ Texto legível nativo | ❌ Binário ilegível no DevTools | **⚡ Extensão [B-FAST Inspector](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** |
| **Throughput de Streaming** | ~22.000 frames/s | Moderado | **🚀 > 3.100.000 frames/s** |
| **Dados Científicos & NumPy** | Serialização lenta de listas | Bytes manuais opacos | **⚡ Velocidade Nativa de Arrays (14-96x)** |

## 🚀 Funcionalidades & Arquitetura

<div class="grid cards" markdown>

-   __⚡ Performance Extrema__

    ---

    **2.01 ms** para 10k objetos (**4.1x mais rápido** que orjson, **6.0x vs JSON**). Até **24x mais rápido** em serialização de primitivos via fast-paths Rust.

    [:octicons-arrow-right-24: Ver Benchmarks](performance.md)

-   __🌊 Streaming em Tempo Real & MCP__

    ---

    Streaming binário contínuo (`application/x-bfast-stream`), transferência HTTP chunked e suporte ao Model Context Protocol (MCP).

    [:octicons-arrow-right-24: Guia de FastMCP & Streaming](mcp.md)

-   __🐍 APIs Python & Data Science__

    ---

    Integrações diretas para **FastAPI**, **Django Ninja**, **Polars** e **Pandas** com serialização nativa de DataFrames.

    [:octicons-arrow-right-24: Guia de Integrações](integrations.md)

-   __💻 TypeScript Moderno & TanStack Query__

    ---

    Cliente para navegadores e Node.js com suporte a **TanStack Query**, validação de schemas em runtime (**Zod / Standard Schema**) e Wasm LZ4.

    [:octicons-arrow-right-24: Guia Frontend](frontend.md)

-   __🤖 Pronto para IAs & LLMs (`llms.txt`)__

    ---

    Endpoint padronizado para **OpenCode**, **Cursor**, **Claude Code** e **Copilot** gerarem código sem alucinações.

    [:octicons-arrow-right-24: Guia para IAs & LLMs](ai.md)

-   __📚 Referência Completa da API__

    ---

    Referência técnica completa para todas as classes, métodos, parâmetros e utilitários em Python e TypeScript.

    [:octicons-arrow-right-24: Referência da API](api.md)

</div>

## 💡 A Filosofia: Serialização Inteligente

O B-FAST nasceu do reconhecimento de que aplicações modernas precisam de mais do que apenas serialização rápida — elas precisam de **serialização inteligente** que se adapta às restrições do mundo real. Após extensas otimizações, o B-FAST opera na **escala de sub-microssegundos** (**676 ns** encode / **754 ns** decode para 100 objetos), alcançando **4.1x mais velocidade** que o orjson para objetos, **5.7x mais rápido** em redes lentas e mais de **3.180.000 frames/s** em protocolo de streaming.

> **Filosofia:** Acreditamos que o futuro da transferência de dados não está apenas na velocidade bruta da CPU, mas em protocolos inteligentes que minimizam a sobrecarga de rede mantendo performance de ponta. O B-FAST representa nossa contribuição para uma web mais eficiente e consciente do uso de banda.

## 📊 Performance

<p align="center">
  <img src="assets/benchmark_chart.png" alt="B-FAST Performance Benchmarks" width="900">
</p>

<p align="center">
  <em>Benchmarks empíricos completos: o B-FAST entrega encode 1.9x mais rápido e decode 4.6x mais rápido que o Protobuf em Python real, serialização 4.1x mais rápida que o orjson e mais de 3.18M frames/s em streaming sustentado.</em>
</p>

### 🚀 Objetos Simples (10.000)
| Formato | Tempo (ms) | Speedup vs JSON | Speedup vs orjson |
|---------|:----------:|:---------------:|:-----------------:|
| JSON (stdlib) | 12.0ms | 1.0x *(Base)* | — |
| orjson | 8.19ms | 1.5x | 1.0x *(Base)* |
| **B-FAST** | **2.01ms** | **🚀 6.0x mais rápido** | **⚡ 4.1x mais rápido** |

*O B-FAST é **4.1x mais rápido** que a biblioteca mais veloz do ecossistema Python (`orjson`) e **6.0x mais rápido** que o `json` padrão.*

### 🌊 Protocolo de Streaming (1.000 frames)
| Métrica | Performance | Speedup / Throughput |
|---------|-------------|----------------------|
| **Streaming Decode (Alinhado)** | **11.8ms** | **~85.000 frames/s** |
| **Streaming Decode (Fragmentado)** | **13.6ms** | **~73.500 frames/s** |
| **Latência por Frame Único** | **139.2µs** | **Parsing instantâneo em tempo real** |
| **Throughput Sustentado de Stream** | **12.500 frames/s** | **Feeds de eventos em alta frequência** |

### 🔄 Round-Trip (Encode + Rede + Decode)

#### 📡 100 Mbps (Rede Lenta)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 114.5ms | 0.8x |
| orjson | 91.7ms | 1.0x |
| **B-FAST (Comprimido)** | **16.1ms** | **🚀 5.7x** |

#### 📡 1 Gbps (Rede Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 29.4ms | 0.5x |
| orjson | 15.3ms | 1.0x |
| **B-FAST (Comprimido)** | **7.2ms** | **🚀 2.1x** |

#### 📡 10 Gbps (Rede Ultra-Rápida)
| Formato | Tempo Total | Speedup vs orjson |
|---------|-------------|-------------------|
| JSON | 20.9ms | 0.4x |
| orjson | 7.7ms | 1.0x |
| **B-FAST (Comprimido)** | **6.3ms** | **🚀 1.2x** |

### 🥊 B-FAST vs Protocol Buffers (Protobuf)

O Protocol Buffers é amplamente reconhecido como padrão da indústria para serialização binária. Contudo, em aplicações Python modernas (como serviços FastAPI, APIs e microsserviços de dados), o uso de Protobuf introduz fricção e sobrecarga de conversão consideráveis.

Em um benchmark ponta a ponta serializando e deserializando 10.000 objetos estruturados de aplicação:

| Característica / Métrica | Protocol Buffers | B-FAST | Vantagem |
|--------------------------|:----------------:|:------:|----------|
| **Necessidade de Esquema** | Exige arquivos `.proto` e compilação | **Nenhum** (Plug-and-play) | Sem boilerplate ou etapa de build |
| **Encode no Mundo Real** *(A partir de modelos da aplicação)* | 13.5ms *(Model → Proto → Wire)* | **7.2ms** *(Direto)* | **🚀 1.87x mais rápido** |
| **Decode Completo** *(Para dicionários/listas nativos do Python)* | 87.1ms *(`MessageToDict`)* | **18.9ms** *(Nativo)* | **🚀 4.6x mais rápido** |
| **Payload Comprimido** | 374 KB | **410 KB** | Compacidade equivalente |
| **Depuração no DevTools** | Exige proxies externos e schemas | **Extensão Oficial para DevTools** | Inspeção direta no Network tab |

**Por que isso importa na prática:**
- **Sem Custo de Conversão:** Embora o Protobuf seja rápido em C++ puro, aplicações em Python gastam ciclos significativos copiando dados entre modelos da aplicação (como Pydantic ou dicionários) e as classes de mensagem geradas pelo Protobuf. O B-FAST trabalha diretamente com os seus objetos Python existentes.
- **Usabilidade Imediata:** O decode inicial do Protobuf entrega apenas wrappers de mensagem especializados; convertê-los para coleções padrão do Python leva mais de 87ms. O B-FAST entrega estruturas nativas do Python prontas para consumo imediatamente após o decode.
- **Ergonomia:** Você obtém serialização binária compacta e alta taxa de transferência sem precisar manter contratos `.proto`, versionar schemas ou executar ferramentas de compilação.

## 🎯 Casos de Uso Ideais

- **📱 Mobile/IoT**: 89% economia de dados + 5.7x performance em redes lentas
- **🌐 APIs com redes lentas**: Até 5.7x mais rápido que orjson
- **📊 Data pipelines**: 14-96x speedup para arrays NumPy
- **🗜️ Storage/Cache**: Compressão superior integrada
- **🚀 Objetos simples**: 4.1x mais rápido que orjson
- **🌊 Streaming em Tempo Real**: > 12.500 frames/s com parsing de chunks sem alocações extras

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

# Serializar (compress=True por padrão)
data = b_fast.encode(users)  # ou bf.encode_packed(users)
print(f"Tamanho: {len(data)} bytes")

# Deserializar (decompress=True por padrão)
decoded = b_fast.decode(data)  # ou bf.decode_packed(data)
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

### 🔍 Extensão DevTools: B-FAST Inspector

Protocolos binários não significam perder visibilidade no navegador. Com a extensão oficial **B-FAST Inspector**, você tem a mesma experiência de inspeção do JSON diretamente no DevTools:

- ⚡ **Interceptação Automática:** Captura e decodifica qualquer resposta `application/x-bfast` no Network tab.
- 🌳 **Árvore Interativa:** Explore os dados decodificados em uma árvore visual expansível e colorida.
- ⏱️ **Métricas & Economia:** Visualize economia de banda e latências de decode em tempo real.
- 📸 **Cartões de Performance:** Gere cartões de benchmark prontos para compartilhamento.

👉 **Instale direto da [Chrome Web Store](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)**

## 🔗 Links Úteis

- [Começando](getting_started.md) - Tutorial completo
- [Frontend](frontend.md) - Integração TypeScript
- [Performance](performance.md) - Análise técnica detalhada
- [Solução de Problemas](troubleshooting.md) - Guia de troubleshooting

## 📄 Licença

Distribuído sob a licença MIT. Veja [LICENSE](https://github.com/marcelomarkus/b-fast/blob/main/LICENSE) para mais informações.
