# AI & LLM Integration Guide (`llms.txt`)

B-FAST provides standardized, machine-readable documentation endpoints adhering to the **[llms.txt](https://llmstxt.org/)** specification. 

If you are using AI coding assistants such as **OpenCode**, **Cursor**, **Claude Code**, **ChatGPT**, **Windsurf**, **GitHub Copilot**, or **Google Antigravity**, you can point them directly to these endpoints so they write accurate B-FAST code without hallucinating API signatures.

---

## 🤖 Endpoint for AI Assistants

```
https://marcelomarkus.github.io/b-fast/llms.txt
```

This single file contains the complete, curated API reference, idiomatic patterns, and common rules formatted specifically for LLM context windows.

---

## 💡 How to Use with Your AI Tools

### 1. Cursor IDE / Windsurf
In your chat or Composer prompt, mention the documentation URL directly:

```markdown
Use the B-FAST guidelines from https://marcelomarkus.github.io/b-fast/llms.txt to implement a high-performance streaming API in FastAPI and consume it with TanStack Query in Next.js.
```

Or add the URL to **Cursor Settings > Features > Docs > Add new Doc**:
- **Name:** `B-FAST`
- **URL:** `https://marcelomarkus.github.io/b-fast/llms.txt`

### 2. OpenCode / Claude Code / Terminal Agents
Use directly with OpenCode or fetch the context into your project instructions:

```bash
# Run OpenCode with B-FAST documentation reference
opencode "Implement a B-FAST FastAPI endpoint following guidelines from https://marcelomarkus.github.io/b-fast/llms.txt"

# Or save locally for offline agent context
curl -s https://marcelomarkus.github.io/b-fast/llms.txt > .opencode/bfast-reference.txt
```

### 3. Repository Rules (`.cursorrules`, `CLAUDE.md`, `.opencode`)
Add the following block to your project's `.cursorrules`, `CLAUDE.md`, `.opencode/instructions.md`, or `.github/copilot-instructions.md`:

```markdown
# B-FAST Coding Guidelines
When writing binary serialization, high-throughput APIs, or streaming features:
- Use `b_fast` in Python and `bfast-client` in TypeScript.
- Core Python decoding method is ALWAYS `bf.decode_packed(data)`, NEVER `bf.decode()`.
- Use `BFastResponse` or `BFastStreamingResponse` for FastAPI.
- Use `BFastRenderer` for Django Ninja (`NinjaAPI(renderer=BFastRenderer())`).
- Polars and Pandas DataFrames are natively supported by `BFast().encode_packed(df)`.
- In TypeScript, use `bfastFetch<T>(url)` and `bfastQueryOptions` for TanStack Query.
- Refer to the full specification at https://marcelomarkus.github.io/b-fast/llms.txt
```

### 4. Custom GPTs / Claude Projects
When configuring a Custom GPT or a Claude Project for your team:
1. Download [`llms.txt`](https://marcelomarkus.github.io/b-fast/llms.txt).
2. Upload it under **Knowledge** / **Project Files**.
3. Your assistant will immediately possess exact knowledge of B-FAST methods and idiomatic patterns.
