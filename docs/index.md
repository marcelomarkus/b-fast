# ⚡ B-FAST Documentation

B-FAST (Binary Fast Adaptive Serialization Transfer) é um protocolo de serialização binária de ultra-alta performance para Python e TypeScript.

## Características Principais
- **Motor Rust:** Serialização nativa sem overhead do interpretador Python
- **Pydantic Native:** Lê modelos Pydantic diretamente da memória
- **Zero-Copy NumPy:** Serializa arrays na velocidade máxima de I/O
- **String Interning:** Chaves repetidas enviadas apenas uma vez
- **LZ4 Integrado:** Compressão ultra-veloz para payloads grandes

## Performance
- **15x mais rápido** que JSON padrão
- **3x mais rápido** que orjson
- **80% menor** payload comparado ao JSON

## Guias de Uso
- [Início Rápido](getting_started.md) - Instalação e primeiros passos
- [Frontend TypeScript](frontend.md) - Integração com aplicações web
- [Otimização](performance.md) - Dicas para máxima performance
- [Troubleshooting](troubleshooting.md) - Solução de problemas comuns

## About B-FAST

B-FAST represents more than just a serialization library—it's a testament to the power of open knowledge sharing.

> *"Knowledge is the only wealth that grows when we share it"*

This philosophy drives every aspect of B-FAST's development. By making high-performance binary serialization accessible to Python and TypeScript developers, we're contributing to a more efficient and capable web ecosystem.

**Created by:** marcelomarkus  
**Mission:** Democratize high-performance serialization through open-source innovation  
**Community:** Built for developers, by developers who believe in sharing knowledge