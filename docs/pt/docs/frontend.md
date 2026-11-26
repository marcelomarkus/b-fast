# 🌐 Frontend TypeScript - Consumindo e Produzindo B-FAST

O pacote [`bfast-client`](https://www.npmjs.com/package/bfast-client) é a biblioteca ultra-rápida de serialização, streaming e decodificação binária para JavaScript e TypeScript.

Suporta **Dual-Module (ESM e CommonJS)**, **Browsers**, **Node.js**, **Bun**, **Deno** e **Cloudflare Workers** com **compressão transparente e zero-config**.

---

## 📦 Instalação

```bash
npm install bfast-client
```

---

## 🚀 Aceleração Transparente Zero-Config (`bfast-client/auto`) ⭐

A maneira mais rápida e não-invasiva de adotar o B-FAST em um frontend existente.

Em vez de reescrever suas chamadas de API para usar `bfastFetch`, basta adicionar `import 'bfast-client/auto'` no arquivo de entrada principal do seu projeto (`main.ts`, `index.tsx`, `App.vue`, `_app.tsx`, etc.). **Todas as chamadas nativas de `fetch()` passarão a negociar e decodificar respostas B-FAST de forma 100% transparente**:

```typescript
// Adicione este import no ponto de entrada da aplicação (ex: main.ts / index.tsx)
import 'bfast-client/auto';

// Seu código existente permanece 100% inalterado!
const response = await fetch('/api/usuarios');
const usuarios = await response.json(); // Decodificado do binário B-FAST automaticamente sob o capô!
```

### Como Funciona por Baixo dos Panos
1. **Negociação de Cabeçalhos:** Adiciona automaticamente `application/x-bfast` ao cabeçalho `Accept` das requisições enviadas, preservando outros tipos configurados.
2. **Interceptação Seletiva:** Apenas intercepta respostas com `content-type: application/x-bfast` ou `application/vnd.bfast`. Endpoints que retornam JSON comum, HTML ou outros formatos continuam totalmente intactos.
3. **Decodificação Transparente:** Substitui o método `response.json()` para ler o buffer binário via `BFastDecoder.decode()` com aceleração WebAssembly. O resultado é memorizado em cache para chamadas subsequentes.
4. **Integração com DevTools:** Dispara eventos `bfast:response` na `window` informando a taxa de compressão e a latência de decodificação para a extensão **B-FAST Inspector**.

### Controle Programático & Métricas

Você também pode gerenciar o ciclo de vida da interceptação ou ouvir métricas de tráfego programaticamente:

```typescript
import { 
    installBFastAuto, 
    uninstallBFastAuto, 
    isBFastAutoInstalled, 
    onBFastResponse 
} from 'bfast-client/auto';

// Verificar se a aceleração automática está ativa
console.log(isBFastAutoInstalled()); // true

// Ouvir métricas de desempenho e compressão
const unsubscribe = onBFastResponse((metric) => {
    console.log(`[B-FAST] ${metric.url}: ${metric.contentLength} bytes (decodificado em ${metric.durationMs}ms)`);
    if (metric.originalLength) {
        const economia = Math.round((1 - metric.contentLength / metric.originalLength) * 100);
        console.log(`Economia de tráfego: -${economia}%`);
    }
});

// Remover o listener
unsubscribe();

// Restaurar o fetch original se necessário
uninstallBFastAuto();

// Reinstalar a qualquer momento
installBFastAuto();
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
    compress: true, // opcional: compressão adaptativa
});
```

---

## 🧵 Decodificação Off-Main-Thread com Web Workers (`bfast-client/worker`) 🚀

Para aplicações de alta frequência ou payloads volumosos (modelos 3D, nuvens de pontos, animações em canvas, mapas geoespaciais, tabelas de telemetria em tempo real), decodificar dados na thread principal (UI thread) do navegador causa quedas de taxa de quadros e travamentos perceptíveis ("jank").

O submódulo `bfast-client/worker` executa a decodificação binária do B-FAST completamente fora da main thread em Web Workers dedicados em segundo plano, mantendo a interface do usuário totalmente livre e responsiva sem micro-travamentos.

> 🎮 **Simulação Interativa ao Vivo:** Veja a biblioteca oficial `bfast-client` em ação com partículas 3D animadas em tempo real na nossa **[Demo 3D em Tempo Real](demo.md)**.

### Fetch com Worker Automático (`bfastFetchWorker`)

```typescript
import { bfastFetchWorker } from 'bfast-client/worker';

// Faz o download e decodifica integralmente em background no Web Worker
const malha = await bfastFetchWorker<{ positions: Float32Array }>('/api/malha-3d', {
    typedArrays: true,
});

// A interface do usuário não congela, 0.00ms de bloqueio no event loop!
```

### Decodificar Buffers Existentes em Worker (`decodeInWorker`)

```typescript
import { decodeInWorker } from 'bfast-client/worker';

// Descarrega o processamento integralmente para o worker em segundo plano
const dados = await decodeInWorker(binaryBuffer);
```

### Pool Escalável de Workers (`BFastWorkerPool`)

Para aplicações com alto volume concorrente de dados (ex: dashboards com dezenas de gráficos ou renderizadores de tiles geoespaciais), você pode gerenciar um pool distribuído de workers:

```typescript
import { BFastWorkerPool } from 'bfast-client/worker';

// Inicializa um pool com 4 workers dedicados em paralelo
const pool = new BFastWorkerPool({ size: 4 });

// Distribui a decodificação entre os workers disponíveis com balanceamento de carga
const resultados = await Promise.all(
    buffers.map(buf => pool.decode(buf))
);

// Finaliza os workers quando necessário
pool.terminate();
```

---

## 🔄 Serialização e Decodificação Bidirecional

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

// Com compressão ativada
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Decodificação Binária (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodifica ArrayBuffer ou Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Extração com arrays tipados (Float64Array)
const numbers = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## ⚡ Compressão Transparente (Zero Config)

O `bfast-client` gerencia compressão e descompressão de forma totalmente automática, transparente e com zero configuração:

- **Zero Configuração:** Sem necessidade de plugins de bundler (Vite, Webpack, Next.js), arquivos externos para servir ou dependências nativas.
- **Detecção Automática:** Ao decodificar qualquer payload gerado com compressão (`compress: true`), o cliente detecta e descomprime os dados instantaneamente.
- **Universal:** Funciona de imediato em navegadores, Node.js, Bun, Deno e Cloudflare Workers.

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

## 🧩 Integração com Frameworks

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

O `bfast-client` suporta a especificação unificada **Standard Schema** (`~standard` de Zod 3.24+, Valibot 1.0+, ArkType 2.0+) e validadores clássicos (`safeParse`/`parse`):

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

## 🔍 Extensão DevTools: B-FAST Inspector

Ao depurar no navegador, payloads binários costumam aparecer como texto bruto ilegível no Network tab do DevTools.

A extensão oficial **[B-FAST Inspector](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** restaura a mesma experiência transparente de depuração do JSON:

- **Interceptação Automática:** Captura qualquer resposta com `Content-Type: application/x-bfast`.
- **Árvore Interativa:** Explore os dados decodificados em uma árvore visual expansível e colorida.
- **Métricas em Tempo Real:** Acompanhe a economia de banda e tempo de decode por requisição.
- **Gerador de Cartão de Performance:** Crie snapshots de benchmark prontos para compartilhamento.

Disponível na **[Chrome Web Store](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** (compatível com Google Chrome, Microsoft Edge, Brave e Opera).

---

## 🛡️ Tratamento de Erros

```typescript
import { BFastDecoder, BFastError, BFastValidationError } from 'bfast-client';

try {
    const data = BFastDecoder.decode(buffer, { schema: UserSchema });
} catch (error) {
    if (error instanceof BFastValidationError) {
        console.error('Falha de validação no payload:', error.issues);
    } else if (error instanceof BFastError) {
        console.error('Erro de decodificação binária B-FAST:', error.message);
    }
}
```

---

## 🌐 Compatibilidade

- **Navegadores:** Chrome 60+, Firefox 55+, Safari 12+, Edge 79+
- **Runtimes:** Node.js 14+, Bun, Deno, Cloudflare Workers
- **Formatos:** Dual Module (ESM nativo + CommonJS)
