# bfast-client

Ultra-fast binary serializer, streaming frame engine, and decoder for the ⚡**B-FAST** format in TypeScript & JavaScript.

Supports **Node.js**, **Browsers**, **Cloudflare Workers**, **Deno**, and **Bun** with **Dual Module (ESM & CJS)** and **Automatic WebAssembly (Wasm) LZ4 Acceleration**.

---

## Installation

```bash
npm install bfast-client
```

---

## Quick Start: `bfastFetch`

The simplest and fastest way to consume or post B-FAST data:

```typescript
import { bfastFetch } from 'bfast-client';

interface User {
    id: number;
    name: string;
    role: string;
}

// GET request: sets Accept header and decodes binary response automatically
const users = await bfastFetch<User[]>('/api/users');
console.log(users[0].name);

// POST request: automatically serializes JS object to B-FAST binary payload
const created = await bfastFetch<User>('/api/users', {
    method: 'POST',
    body: { name: 'Alice', role: 'admin' },
    compress: true, // optional LZ4 compression
});
```

---

## Encoding & Decoding

### Binary Encoding (`BFastEncoder`)

Serialize native JavaScript objects, dates, arrays, and typed arrays directly to B-FAST binary:

```typescript
import { BFastEncoder } from 'bfast-client';

const payload = {
    userId: 42,
    username: 'alice',
    tags: ['admin', 'engineering'],
    createdAt: new Date(),
    matrix: new Float64Array([1.0, 2.5, 3.8]),
};

// Raw uncompressed B-FAST bytes (Uint8Array)
const bytes = BFastEncoder.encode(payload);

// Or with LZ4 compression enabled
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Binary Decoding (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodes ArrayBuffer or Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Zero-copy TypedArrays for numerical arrays (Float64Array, etc.)
const fastNumeric = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## 🚀 WebAssembly Acceleration (Zero-Config)

`bfast-client` comes with an embedded **10 KB WebAssembly LZ4 engine** compiled from Rust (`lz4_flex`).

- **Automatic Activation:** Activated lazily on the first compressed block encountered. No bundler plugins, no file loading, and no network requests required!
- **Transparent Fallback:** If `WebAssembly` is unavailable (e.g. strict CSP), it silently and transparently falls back to pure JavaScript `lz4js`.
- **Custom Decompressors:** You can inspect status or register custom decompressors:

```typescript
import { isWasmEnabled, initWasmLz4Sync, BFastDecoder } from 'bfast-client';

// Check if Wasm acceleration is currently active
console.log('Wasm LZ4 active:', isWasmEnabled());

// Optional: explicit manual initialization if desired
initWasmLz4Sync();

// Custom decompressor hook
BFastDecoder.setDecompressor((compressedBytes, uncompressedSize) => {
    // your custom decompression logic
    return decompressedBytes;
});
```

---

## ⚡ Real-Time Streaming (Bidirectional)

Stream data in real-time as chunks arrive over HTTP (`ReadableStream`), WebSockets, or Node.js streams.

### Consuming Streams

```typescript
import { decodeReadableStream, decodeStream } from 'bfast-client';

// Browser fetch with ReadableStream
const response = await fetch('/api/telemetry/stream');
for await (const metric of decodeReadableStream(response.body!)) {
    console.log('Live metric:', metric);
}

// Or universal decodeStream for both ReadableStream and Node.js async iterables
for await (const chunk of decodeStream(response.body!)) {
    console.log('Received:', chunk);
}
```

### Producing Stream Frames (`BFastStreamEncoder`)

```typescript
import { BFastStreamEncoder } from 'bfast-client';

// Handshake frame (optional / protocol-level)
const handshake = BFastStreamEncoder.getHandshake();

// Encode JavaScript objects directly into streaming frames
const frame1 = BFastStreamEncoder.encodeFrame({ sensorId: 1, temp: 22.4 });
const frame2 = BFastStreamEncoder.encodeFrame({ sensorId: 2, temp: 23.1 });

// End of Stream frame
const eos = BFastStreamEncoder.getEosFrame();
```

---

## Framework Integrations

### TanStack Query (React Query, Vue Query, Svelte, Solid)

Use `bfastQueryOptions` to wire binary B-FAST endpoints directly into TanStack Query with full type inference, signal forwarding, and optional runtime schema validation:

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

function UserProfile({ userId }: { userId: number }) {
    const { data: user, isLoading, error } = useQuery(
        bfastQueryOptions<User>({
            queryKey: ['user', userId],
            url: `/api/users/${userId}`,
            schema: UserSchema, // validates response at runtime
            staleTime: 10_000,
        })
    );

    if (isLoading) return <div>Loading...</div>;
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

### Runtime Schema Validation (Zod, Valibot, ArkType, Standard Schema)

`bfast-client` provides universal schema validation conforming to the **Standard Schema** (`~standard`) specification, as well as classic Zod schemas and validator functions:

```typescript
import { BFastDecoder, bfastFetch, BFastValidationError } from 'bfast-client';
import { z } from 'zod';

const MetricsSchema = z.object({
    cpu: z.number(),
    memory: z.number(),
    nodes: z.array(z.string()),
});

// 1. With bfastFetch
try {
    const metrics = await bfastFetch('/api/metrics', { schema: MetricsSchema });
} catch (err) {
    if (err instanceof BFastValidationError) {
        console.error('Validation issues:', err.issues);
    }
}

// 2. With BFastDecoder
const metrics = BFastDecoder.decode(binaryBuffer, { schema: MetricsSchema });

// 3. With BFastStreamDecoder (validates every chunk in real-time)
const streamDecoder = new BFastStreamDecoder({ schema: MetricsSchema });
```

### React Hook

```typescript
import { useState, useEffect } from 'react';
import { bfastFetch } from 'bfast-client';

export function useBFast<T>(url: string) {
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

## 🤖 Model Context Protocol (MCP)

Easily consume B-FAST binary tool results and resources from MCP / FastMCP servers:

```typescript
import { decodeMcpResource } from 'bfast-client';

// Call tool via MCP client (e.g. @modelcontextprotocol/sdk)
const result = await mcpClient.callTool({ name: 'query_records', arguments: {} });

// Decodes embedded B-FAST resource automatically into strongly-typed objects
const data = decodeMcpResource<User[]>(result);
console.log(data);
```

---

## Supported Types

- **Primitives**: `null`, `boolean`, `number` (small int, int32, int64, float64), `string`
- **Collections**: `Array`, `Object`, nested structures
- **Special Types**: `Date`, `UUID`, NumPy typed arrays (`Float64Array`)
- **String Table**: Automatic string interning for repetitive keys
- **Compression**: Automatic LZ4 compression & decompression

---

## License

MIT
