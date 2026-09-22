# Contribuindo com o B-FAST

Obrigado pelo seu interesse em contribuir com o B-FAST! Apoiamos e incentivamos contribuições de toda a comunidade e estamos animados para ver o que você trará para o projeto.

## 🌟 Filosofia

> "O conhecimento é a única riqueza que cresce quando a compartilhamos"

O B-FAST foi construído sobre o princípio do compartilhamento aberto de conhecimento. Toda contribuição, não importa o tamanho, ajuda a tornar a serialização de alta performance acessível a mais desenvolvedores.

## 🚀 Começando

### Pré-requisitos
- **Rust** 1.70+ (para o core da biblioteca)
- **Python** 3.8+ com `uv` ou `pip`
- **Node.js** 18+ (para o cliente TypeScript)
- **Git** para controle de versão

### Configuração do Ambiente de Desenvolvimento

1. **Faça um Fork e clone o repositório:**
   ```bash
   git clone https://github.com/marcelomarkus/b-fast.git
   cd b-fast
   ```

2. **Configure o ambiente Python:**
   ```bash
   uv sync --extra dev
   # ou
   pip install bfast-py[dev]
   ```

3. **Instale as dependências do TypeScript:**
   ```bash
   cd client-ts
   npm install
   cd ..
   ```

4. **Execute os testes para verificar o ambiente:**
   ```bash
   # Testes em Python
   python -m pytest tests/
   
   # Build e testes em TypeScript
   cd client-ts && npm run build
   cd ..
   ```

## 🛠️ Fluxo de Trabalho de Desenvolvimento

### Fazendo Alterações

1. **Crie uma branch de feature:**
   ```bash
   git checkout -b feat/nome-da-sua-feature
   # ou
   git checkout -b fix/descricao-do-bug
   ```
   
   **Convenções de nomenclatura de branches:**
   - `feat/*` - Novas funcionalidades ou melhorias (ex: `feat/streamable-http`)
   - `fix/*` - Correções de bugs (ex: `fix/memory-leak`)
   - `docs/*` - Alterações exclusivas de documentação (ex: `docs/contributing`)
   
   **Nota:** A esteira de CI roda automaticamente em branches `feat/*` e `fix/*`.

2. **Faça suas alterações** seguindo nossos padrões de código.

3. **Execute a suíte completa de testes:**
   ```bash
   # Testes Python
   python -m pytest tests/ -v
   
   # Formatação e checagem de linter
   python -m ruff check .
   
   # Build do TypeScript
   cd client-ts && npm run build && cd ..
   ```

4. **Faça commits com mensagens descritivas:**
   ```bash
   git commit -m "feat: add support for UUID serialization
   
   - Implement UUID type in Rust encoder
   - Add UUID parsing in TypeScript decoder
   - Include tests for UUID round-trip serialization"
   ```

## 📝 Tipos de Contribuição

### 🐛 Relatórios de Bugs (Bug Reports)
- Use o template de bug report no GitHub
- Inclua um exemplo mínimo reproduzível
- Especifique as versões do Python, Node.js e Rust utilizadas
- Inclua mensagens de erro e stack traces completos

### ✨ Sugestões de Recursos (Feature Requests)
- Abra uma issue ou discussão explicando o recurso
- Explique o caso de uso e os benefícios reais
- Considere compatibilidade com versões anteriores e impacto em performance

### 🔧 Contribuições de Código
- **Biblioteca Core em Rust:** `src/lib.rs`, `src/errors.rs`
- **Bindings em Python:** `python/b_fast/`
- **Cliente TypeScript:** `client-ts/`
- **Documentação:** `docs/`, `zensical.toml`, `README.md`
- **Testes:** `tests/`

### 📚 Documentação
- Melhorias na documentação da API
- Exemplos de uso e tutoriais práticos
- Guias de otimização de performance
- Tradução e revisão de conteúdos

## 🎯 Padrões de Código

### Python
- **Formatação & Linter:** Ruff com a configuração do projeto
- **Type hints:** Obrigatório para APIs públicas
- **Testes:** `pytest` com nomes descritivos

### Rust
- **Formatação:** `cargo fmt`
- **Linter:** `cargo clippy`
- **Documentação:** Comentários Rustdoc para APIs públicas
- **Segurança:** Minimizar uso de código `unsafe`, documentando quando estritamente necessário

### TypeScript
- **Formatação:** Prettier (via scripts npm)
- **Linter:** Configuração ESLint do projeto
- **Tipagem:** TypeScript estrito, evite `any` em APIs públicas
- **Compatibilidade:** ES2020+ para ambientes modernos

## 🧪 Diretrizes de Testes

### Categorias de Testes
- **Testes unitários:** Teste de funções e métodos individuais
- **Testes de integração:** Teste de interação entre componentes e serialização
- **Testes de performance:** Benchmarks em rotas críticas ([Guia de Performance](performance.md))
- **Testes de compatibilidade:** Validação cruzada entre Python e TypeScript

### Requisitos para Testes
- Todas as novas funcionalidades devem incluir testes
- Correções de bugs devem incluir testes de regressão
- Mudanças focadas em performance devem incluir dados comparativos de benchmarks
- Mudanças com quebra de compatibilidade exigem notas de migração

## 📋 Processo de Pull Request

1. **Certifique-se de que sua PR:**
   - Possui um título claro e descritivo
   - Referencia issues relacionadas (`Fixes #123`)
   - Inclui testes para o novo código
   - Atualiza a documentação quando aplicável
   - Passa em todas as checagens do CI

2. **Revisão:**
   - Os mantenedores revisarão em até 48 horas
   - Responda aos comentários e sugestões de forma construtiva
   - Mantenha o tom sempre respeitoso

3. **Requisitos para Merge:**
   - Todos os testes passando
   - Aprovação na revisão de código
   - Sem conflitos de merge
   - Documentação sincronizada

## 🤝 Diretrizes da Comunidade

- **Seja Respeitoso:** Use linguagem inclusiva e respeite diferentes pontos de vista.
- **Seja Colaborativo:** Compartilhe conhecimento, dê crédito ao trabalho de outros e ajude recém-chegados.
- **Seja Profissional:** Mantenha discussões construtivas e siga o Código de Conduta.

## 🆘 Onde Obter Ajuda

- **Discussões:** [GitHub Discussions](https://github.com/marcelomarkus/b-fast/discussions)
- **Issues:** [GitHub Issues](https://github.com/marcelomarkus/b-fast/issues)
- **Documentação:** [https://marcelomarkus.github.io/b-fast/pt/](https://marcelomarkus.github.io/b-fast/pt/)

---

Muito obrigado por contribuir com o B-FAST! Juntos tornamos a serialização de altíssima performance acessível para todos. 🚀
