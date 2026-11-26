# 🌐 Frontend TypeScript - Consuming & Producing B-FAST

The [`bfast-client`](https://www.npmjs.com/package/bfast-client) package is the ultra-fast binary serialization, streaming, and decoding library for JavaScript and TypeScript.

It supports **Dual-Module (ESM & CommonJS)**, **Browsers**, **Node.js**, **Bun**, **Deno**, and **Cloudflare Workers** with **automatic, zero-config adaptive compression**.

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
    compress: true, // optional compression
});
```

---

## 🧵 Web Worker Off-Main-Thread Decoding (`bfast-client/worker`) 🚀

For high-frequency or large payloads (3D models, point clouds, canvas animations, geospatial datasets, real-time telemetry tables), decoding data on the browser's main UI thread can cause noticeable frame drops and UI freezes ("jank").

The `bfast-client/worker` subpath decodes B-FAST binary payloads entirely off the main thread in background Web Workers, keeping your UI thread completely free and responsive without micro-stutters.

### Drop-in Worker Fetch (`bfastFetchWorker`)

```typescript
import { bfastFetchWorker } from 'bfast-client/worker';

// Fetches and decodes entirely inside a background Web Worker
const mesh = await bfastFetchWorker<{ positions: Float32Array }>('/api/3d-mesh', {
    typedArrays: true,
});

// UI thread never stutters, 0.00ms main thread blocking!
```

### Decode Existing Buffers in Worker (`decodeInWorker`)

```typescript
import { decodeInWorker } from 'bfast-client/worker';

// Offloads decoding entirely to a background worker
const decoded = await decodeInWorker(binaryBuffer);
```

### Scalable Worker Pool (`BFastWorkerPool`)

For heavy concurrent applications (e.g. multi-chart dashboards, GIS tile loaders, or simulation engines), you can create and manage dedicated worker pools:

```typescript
import { BFastWorkerPool } from 'bfast-client/worker';

// Initialize a pool with 4 concurrent background workers
const pool = new BFastWorkerPool({ size: 4 });

// Decode across available pool workers with automatic load balancing
const results = await Promise.all(
    buffers.map(buf => pool.decode(buf))
);

// Terminate pool workers when no longer needed
pool.terminate();
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

// With compression enabled
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Binary Decoding (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodes ArrayBuffer or Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Direct typed array extraction (Float64Array)
const numbers = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## ⚡ Transparent Compression (Zero Config)

`bfast-client` handles compression and decompression automatically and transparently with zero setup:

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

## 🔍 DevTools Extension: B-FAST Inspector

When debugging in the browser, binary network payloads can normally appear as unreadable binary text in your browser's Network tab. 

The official **[B-FAST Inspector](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** extension brings the same seamless inspection experience you have with JSON:

- **Automatic Interception:** Captures any network response with `Content-Type: application/x-bfast`.
- **Interactive JSON Tree:** View parsed responses as expandable, color-coded trees.
- **Timing & Bandwidth Breakdown:** See exact savings and decode latencies in real time.
- **Proof Card Generator:** Create benchmark snapshots directly from your DevTools panel.

Available on the **[Chrome Web Store](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** (compatible with Google Chrome, Microsoft Edge, Brave, and Opera).

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
