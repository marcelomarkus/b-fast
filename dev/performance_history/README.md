# B-FAST Performance History

Este diretório contém o histórico de testes de performance do B-FAST ao longo do tempo.

## Estrutura

Cada arquivo segue o formato: `YYYY-MM-DD_performance_test.md`

## Histórico

- **2026-02-07**: Implementação do modo híbrido (fast/complex path)
  - 1.7x mais rápido que orjson em objetos simples
  - 8x mais rápido em redes lentas (100k objetos grandes)
  - 90% de redução de tamanho com LZ4

## Como Adicionar Novos Resultados

1. Execute os benchmarks: `python dev/benchmarks/test_hybrid_mode.py`
2. Crie um novo arquivo com a data atual
3. Documente as mudanças e melhorias
4. Compare com versões anteriores
