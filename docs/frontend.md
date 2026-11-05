# 🌐 Frontend TypeScript - Consuming & Producing B-FAST

The [`bfast-client`](https://www.npmjs.com/package/bfast-client) package is the ultra-fast binary serialization, streaming, and decoding library for JavaScript and TypeScript.

It supports **Dual-Module (ESM & CommonJS)**, **Browsers**, **Node.js**, **Bun**, **Deno**, and **Cloudflare Workers** with **automatic, zero-config LZ4 compression**.

---

## 📦 Installation

```bash
npm install bfast-client
```

---

## 🚀 Drop-in Transparent Acceleration (`bfast-client/auto`) ⭐

The fastest and most non-invasive way to adopt B-FAST across an existing frontend application.

Instead of rewriting your API calls to use `bfastFetch`, simply add `import 'bfast-client/auto'` at the root entry point of your project (`main.ts`, `index.tsx`, `App.vue`, `_app.tsx`, etc.). **All standard native `fetch()` calls will automatically negotiate and decode B-FAST payloads transparently**:

```typescript
// Add this single import at your application entrypoint (e.g. main.ts / index.tsx)
import 'bfast-client/auto';

// Existing code remains 100% unchanged!
const response = await fetch('/api/users');
const users = await response.json(); // Automatically & transparently decoded from B-FAST binary!
```

### How It Works Under the Hood
1. **Header Negotiation:** Automatically appends `application/x-bfast` to outgoing HTTP `Accept` headers without overriding existing custom headers.
2. **Selective Interception:** Only intercepts responses returning `content-type: application/x-bfast` or `application/vnd.bfast`. Regular JSON endpoints, images, HTML, and other assets are left completely untouched.
3. **Transparent Decoding:** Replaces `response.json()` to parse the binary buffer through `BFastDecoder.decode()` with WebAssembly acceleration. Parsed results are cached so repeated calls return the same reference.
4. **DevTools & Telemetry Integration:** Emits `bfast:response` browser events containing bandwidth savings metrics and payload timing for the **B-FAST Inspector DevTools Extension**.

### Programmatic Control & Metrics

You can also control the lifecycle or listen for response telemetry metrics programmatically:

```typescript
import { 
    installBFastAuto, 
    uninstallBFastAuto, 
    isBFastAutoInstalled, 
    onBFastResponse 
} from 'bfast-client/auto';

// Check if transparent fetch interception is active
console.log(isBFastAutoInstalled()); // true

// Listen to performance & compression metrics
const unsubscribe = onBFastResponse((metric) => {
    console.log(`[B-FAST] ${metric.url}: ${metric.contentLength} bytes (decoded in ${metric.durationMs}ms)`);
    if (metric.originalLength) {
        const savings = Math.round((1 - metric.contentLength / metric.originalLength) * 100);
        console.log(`Bandwidth savings: -${savings}%`);
    }
});

// Remove listener when needed
unsubscribe();

// Temporarily restore original fetch implementation
uninstallBFastAuto();

// Re-install anytime
installBFastAuto();
```

---

## ⚡ Quick Start: `bfastFetch`

The fastest and most ergonomic way to communicate with B-FAST endpoints:

```typescript
import { bfastFetch } from 'bfast-client';

interface User {
    id: number;
    name: string;
    role: string;
}

// GET: Automatically sets Accept headers and decodes the binary response
const users = await bfastFetch<User[]>('/api/users');
console.log(users[0].name);

// POST: Automatically serializes JS objects to B-FAST binary payload
const created = await bfastFetch<User>('/api/users', {
    method: 'POST',
    body: { name: 'Alice', role: 'admin' },
    compress: true, // optional LZ4 compression
});
```

---

## 🔄 Bidirectional Serialization & Decoding

### Binary Encoding (`BFastEncoder`)

Serializes native JavaScript objects, dates, arrays, and typed arrays directly into the binary wire format:

```typescript
import { BFastEncoder } from 'bfast-client';

const payload = {
    userId: 42,
    username: 'alice',
    tags: ['admin', 'dev'],
    createdAt: new Date(),
    matrix: new Float64Array([1.5, 2.5, 3.5]),
};

// Generates pure B-FAST Uint8Array binary
const bytes = BFastEncoder.encode(payload);

// With LZ4 compression enabled
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Binary Decoding (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodes ArrayBuffer or Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Zero-copy typed array extraction (Float64Array)
const numbers = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## ⚡ Transparent Compression (Zero Config)

`bfast-client` handles LZ4 compression and decompression automatically and transparently with zero setup:

- **Zero Configuration:** No bundler plugins (Vite, Webpack, Next.js), external assets to serve, or native build steps required.
- **Automatic Detection:** When decoding any compressed payload (`compress: true`), the client detects and decompresses data instantly.
- **Universal:** Works out of the box in browsers, Node.js, Bun, Deno, and Cloudflare Workers.

---

## 🌊 Real-Time Streaming (Bidirectional)

### Consuming Streams (`decodeReadableStream` and `decodeStream`)

Consume continuous data frames emitted by `BFastStreamingResponse` or MCP servers without blocking the user interface:

```typescript
import { decodeReadableStream, decodeStream } from 'bfast-client';

// In browsers with Fetch API ReadableStream
const response = await fetch('/api/stream-users');
for await (const user of decodeReadableStream<User>(response.body!)) {
    console.log('Real-time user received:', user);
}

// Universal (ReadableStream or Node.js async iterators)
for await (const item of decodeStream(response.body!)) {
    console.log('Stream item received:', item);
}
```

### Encoding Stream Frames (`BFastStreamEncoder`)

```typescript
import { BFastStreamEncoder } from 'bfast-client';

// Stream handshake
const handshake = BFastStreamEncoder.getHandshake();

// Encode JavaScript objects into binary stream frames
const frame = BFastStreamEncoder.encodeFrame({ sensor: 'A', value: 42 });

// End of Stream (EOS) marker frame
const eos = BFastStreamEncoder.getEosFrame();
```

---

## 🧩 Framework Integrations

### TanStack Query (React Query, Vue Query, Svelte, Solid)

The recommended way to integrate B-FAST with TanStack Query is using `bfastQueryOptions`:

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
            schema: UserSchema, // Runtime validation & strict typing
            staleTime: 10_000,
        })
    );

    if (isLoading) return <span>Loading...</span>;
    return <h1>{user?.name}</h1>;
}
```

#### Infinite Query (Pagination & Infinite Scroll)

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

### Runtime Schema Validation (Zod / Standard Schema)

`bfast-client` universally supports the **Standard Schema** specification (`~standard` for Zod 3.24+, Valibot 1.0+, ArkType 2.0+) and classic `safeParse`/`parse` validators:

```typescript
import { BFastDecoder, bfastFetch, BFastValidationError } from 'bfast-client';
import { z } from 'zod';

const MetricsSchema = z.object({
    cpu: z.number(),
    memory: z.number(),
    nodes: z.array(z.string()),
});

// 1. In bfastFetch:
try {
    const metrics = await bfastFetch('/api/metrics', { schema: MetricsSchema });
} catch (err) {
    if (err instanceof BFastValidationError) {
        console.error('Schema validation issues:', err.issues);
    }
}

// 2. In synchronous BFastDecoder:
const metrics = BFastDecoder.decode(buffer, { schema: MetricsSchema });

// 3. In streams (validates every incoming frame):
const streamDecoder = new BFastStreamDecoder({ schema: MetricsSchema });
```

### Custom React Hook

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

## 🛡️ Error Handling

```typescript
import { BFastDecoder, BFastError, BFastValidationError } from 'bfast-client';

try {
    const data = BFastDecoder.decode(buffer, { schema: UserSchema });
} catch (error) {
    if (error instanceof BFastValidationError) {
        console.error('Payload validation failure:', error.issues);
    } else if (error instanceof BFastError) {
        console.error('B-FAST binary decoding error:', error.message);
    }
}
```

---

## 🌐 Compatibility

- **Browsers:** Chrome 60+, Firefox 55+, Safari 12+, Edge 79+
- **Runtimes:** Node.js 14+, Bun, Deno, Cloudflare Workers
- **Module Formats:** Dual Module (native ESM + CommonJS)
