# Frontend TypeScript - Consumindo e Produzindo B-FAST

O pacote [`bfast-client`](file:///home/markus/dev/b-fast/client-ts) é a biblioteca ultra-rápida de serialização, streaming e decodificação binária para JavaScript e TypeScript.

Suporta **Dual-Module (ESM e CommonJS)**, **Browsers**, **Node.js**, **Bun**, **Deno** e **Cloudflare Workers** com **aceleração nativa WebAssembly LZ4 automática**.

---

## Instalação

```bash
npm install bfast-client
```

---

## ⚡ Início Rápido: `bfastFetch`

A maneira mais rápida e ergonômica de se comunicar com endpoints B-FAST:

```typescript
import { bfastFetch } from 'bfast-client';

interface User {
    id: number;
    name: string;
    role: string;
}

// GET: Configura automaticamente cabeçalhos e decodifica a resposta binária
const users = await bfastFetch<User[]>('/api/users');
console.log(users[0].name);

// POST: Serializa automaticamente objetos JS para binário B-FAST
const created = await bfastFetch<User>('/api/users', {
    method: 'POST',
    body: { name: 'Alice', role: 'admin' },
    compress: true, // opcional: compressão LZ4
});
```

---

## Serialização e Decodificação Bidirecional

### Codificação Binária (`BFastEncoder`)

Serializa objetos JavaScript nativos, datas, arrays e arrays tipados diretamente para a representação binária wire format:

```typescript
import { BFastEncoder } from 'bfast-client';

const payload = {
    userId: 42,
    username: 'alice',
    tags: ['admin', 'dev'],
    createdAt: new Date(),
    matrix: new Float64Array([1.5, 2.5, 3.5]),
};

// Gera Uint8Array binário B-FAST puro
const bytes = BFastEncoder.encode(payload);

// Com compressão LZ4 ativada
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Decodificação Binária (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodifica ArrayBuffer ou Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Extração com arrays tipados (Zero-Copy Float64Array)
const numbers = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## 🚀 Aceleração WebAssembly LZ4 (Automática)

O `bfast-client` traz embutido um módulo compilado em Rust (`lz4_flex`) de apenas **10 KB** em base64:

- **100% Automático:** É inicializado por demanda (*lazy auto-init*) no primeiro bloco comprimido recebido.
- **Zero Configuração:** Não precisa de plugins de bundler (Vite, Webpack, Next.js) nem requisições adicionais de rede.
- **Fallback Transparente:** Se o ambiente restringir WebAssembly (ex: CSP corporativo restrito), o cliente chaveia silenciosamente e sem erro para o descompressor puro JavaScript (`lz4js`).
- **Verificação e Extensibilidade:**

```typescript
import { isWasmEnabled, BFastDecoder } from 'bfast-client';

// Verifica se a aceleração Wasm está ativa
console.log('WebAssembly LZ4 ativo:', isWasmEnabled());

// Suporta registrar descompressor customizado se necessário
BFastDecoder.setDecompressor((compressed, uncompressedSize) => {
    return minhaDescompressao(compressed, uncompressedSize);
});
```

---

## 🌊 Streaming em Tempo Real (Bidirecional)

### Consumindo Streams (`decodeReadableStream` e `decodeStream`)

Consuma dados progressivos enviados por `BFastStreamingResponse` ou servidores MCP sem travar a interface:

```typescript
import { decodeReadableStream, decodeStream } from 'bfast-client';

// No navegador com Fetch API ReadableStream
const response = await fetch('/api/stream-users');
for await (const user of decodeReadableStream<User>(response.body!)) {
    console.log('Usuário recebido em tempo real:', user);
}

// Universal (tanto ReadableStream quanto async iterators do Node.js)
for await (const item of decodeStream(response.body!)) {
    console.log('Item recebido:', item);
}
```

### Codificando Frames de Stream (`BFastStreamEncoder`)

```typescript
import { BFastStreamEncoder } from 'bfast-client';

// Handshake de stream
const handshake = BFastStreamEncoder.getHandshake();

// Codifica objetos JS diretamente em frames
const frame = BFastStreamEncoder.encodeFrame({ sensor: 'A', value: 42 });

// Frame de término de stream (End of Stream)
const eos = BFastStreamEncoder.getEosFrame();
```

---

## Integração com Frameworks

### TanStack Query (React Query, Vue Query, Svelte, Solid)

A melhor forma de integrar o B-FAST com React Query ou qualquer sabor do TanStack Query é com o helper `bfastQueryOptions`:

```typescript
import { useQuery } from '@tanstack/react-query';
import { bfastQueryOptions } from 'bfast-client';
import { z } from 'zod';

const UserSchema = z.object({
    id: z.number(),
    name: z.string(),
    email: z.string().email(),
});
type User = z.infer<typeof UserSchema>;

export function UserProfile({ id }: { id: number }) {
    const { data: user, isLoading } = useQuery(
        bfastQueryOptions<User>({
            queryKey: ['user', id],
            url: `/api/users/${id}`,
            schema: UserSchema, // Validação e tipagem estrita em runtime
            staleTime: 10_000,
        })
    );

    if (isLoading) return <span>Carregando...</span>;
    return <h1>{user?.name}</h1>;
}
```

#### Infinite Query (Paginação e Scroll Infinito)

```typescript
import { useInfiniteQuery } from '@tanstack/react-query';
import { bfastInfiniteQueryOptions } from 'bfast-client';

const queryOptions = bfastInfiniteQueryOptions<User[]>({
    queryKey: ['users', 'infinite'],
    initialPageParam: 1,
    getUrl: (page) => `/api/users?page=${page}`,
    getNextPageParam: (lastPage, allPages, lastParam) => {
        return lastPage.length === 20 ? (lastParam as number) + 1 : undefined;
    },
});

const { data, fetchNextPage, hasNextPage } = useInfiniteQuery(queryOptions);
```

### Validação de Schemas em Tempo de Execução (Zod / Standard Schema)

O `bfast-client` suporta a especificação unificada **Standard Schema** (`~standard` de Zod 3.24+, Valibot 1.0+, ArkType) e validadores clássicos:

```typescript
import { BFastDecoder, bfastFetch, BFastValidationError } from 'bfast-client';
import { z } from 'zod';

const MetricsSchema = z.object({
    cpu: z.number(),
    memory: z.number(),
    nodes: z.array(z.string()),
});

// 1. No bfastFetch:
try {
    const metrics = await bfastFetch('/api/metrics', { schema: MetricsSchema });
} catch (err) {
    if (err instanceof BFastValidationError) {
        console.error('Erros no schema:', err.issues);
    }
}

// 2. No BFastDecoder síncrono:
const metrics = BFastDecoder.decode(buffer, { schema: MetricsSchema });

// 3. Em streams (valida cada frame recebido):
const streamDecoder = new BFastStreamDecoder({ schema: MetricsSchema });
```

### React Hook Personalizado

```typescript
import { useState, useEffect } from 'react';
import { bfastFetch } from 'bfast-client';

export function useBFastData<T>(url: string) {
    const [data, setData] = useState<T | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<Error | null>(null);

    useEffect(() => {
        let active = true;
        bfastFetch<T>(url)
            .then(res => { if (active) setData(res); })
            .catch(err => { if (active) setError(err); })
            .finally(() => { if (active) setLoading(false); });
        return () => { active = false; };
    }, [url]);

    return { data, loading, error };
}
```

### Axios Interceptor

```typescript
import axios from 'axios';
import { BFastDecoder } from 'bfast-client';

axios.interceptors.response.use(response => {
    if (response.headers['content-type'] === 'application/x-bfast') {
        response.data = BFastDecoder.decode(response.data);
    }
    return response;
});
```

---

## Tratamento de Erros

```typescript
import { BFastDecoder, BFastError } from 'bfast-client';

try {
    const data = BFastDecoder.decode(buffer);
} catch (error) {
    if (error instanceof BFastError) {
        console.error('Erro de decodificação B-FAST:', error.message);
    }
}
```

---

## Compatibilidade

- **Navegadores:** Chrome 60+, Firefox 55+, Safari 12+, Edge 79+
- **Runtimes:** Node.js 14+, Bun, Deno, Cloudflare Workers
- **Formatos:** Dual Module (ESM nativo + CommonJS)
