# Guia para IAs & LLMs (`llms.txt`)

O B-FAST disponibiliza endpoints padronizados para leitura automatizada por modelos de linguagem, seguindo a especificação **[llms.txt](https://llmstxt.org/)**.

Se você utiliza assistentes de programação com IA como **OpenCode**, **Cursor**, **Claude Code**, **ChatGPT**, **Windsurf**, **GitHub Copilot** ou **Google Antigravity**, você pode simplesmente apontar esses links para que eles escrevam código B-FAST correto e idiomático, sem alucinar métodos inexistentes.

---

## 🤖 Endpoint para Assistentes de IA

```
https://marcelomarkus.github.io/b-fast/pt/llms.txt
```

Esse arquivo contém o resumo completo, curado e com os métodos exatos e padrões recomendados para consumo direto por modelos de linguagem com baixo uso de tokens.

---

## 💡 Como Usar com suas Ferramentas de IA

### 1. Cursor IDE / Windsurf
No chat ou no Composer, referencie o link diretamente no seu prompt:

```markdown
Utilize as diretrizes do B-FAST disponíveis em https://marcelomarkus.github.io/b-fast/pt/llms.txt para implementar uma rota de streaming no FastAPI e consumi-la com TanStack Query no Next.js.
```

Ou adicione aos documentos indexados no **Cursor (Settings > Features > Docs > Add new Doc)**:
- **Name:** `B-FAST`
- **URL:** `https://marcelomarkus.github.io/b-fast/pt/llms.txt`

### 2. OpenCode / Claude Code / Agentes de Terminal
Utilize diretamente na CLI do OpenCode ou baixe as diretrizes nas instruções do seu projeto:

```bash
# Executar diretamente com OpenCode CLI
opencode "Implemente uma rota de streaming no FastAPI seguindo https://marcelomarkus.github.io/b-fast/pt/llms.txt"

# Ou salve para contexto offline do agente
curl -s https://marcelomarkus.github.io/b-fast/pt/llms.txt > .opencode/bfast-referencia.txt
```

### 3. Regras de Repositório (`.cursorrules`, `CLAUDE.md`, `.opencode`)
Adicione o trecho abaixo ao seu arquivo `.cursorrules`, `CLAUDE.md`, `.opencode/instructions.md` ou `.github/copilot-instructions.md`:

```markdown
# Diretrizes para B-FAST
Ao implementar serialização binária de alta performance ou streaming:
- Utilize `b_fast` no Python e `bfast-client` no TypeScript.
- O método de decodificação no Python é SEMPRE `bf.decode_packed(data)`, NUNCA `bf.decode()`.
- Use `BFastResponse` ou `BFastStreamingResponse` no FastAPI.
- Use `BFastRenderer` no Django Ninja (`NinjaAPI(renderer=BFastRenderer())`).
- DataFrames Polars e Pandas são suportados nativamente por `BFast().encode_packed(df)`.
- No TypeScript, use `bfastFetch<T>(url)` e `bfastQueryOptions` para TanStack Query.
- Consulte a especificação em https://marcelomarkus.github.io/b-fast/pt/llms.txt
```

### 4. Custom GPTs / Claude Projects
Ao configurar um GPT personalizado ou um projeto no Claude:
1. Baixe o arquivo [`llms.txt`](https://marcelomarkus.github.io/b-fast/pt/llms.txt).
2. Faça o upload na seção **Conhecimento** (Knowledge / Project Files).
3. Seu assistente terá conhecimento exato do B-FAST em todas as respostas.
