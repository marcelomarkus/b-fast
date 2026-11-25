# bfast-client

<p align="left">
  <a href="https://www.npmjs.com/package/bfast-client"><img src="https://img.shields.io/npm/v/bfast-client?color=crimson&logo=npm&logoColor=white" alt="npm version"></a>
  <a href="https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp"><img src="https://img.shields.io/badge/Chrome%20Web%20Store-B--FAST%20Inspector-blue?logo=googlechrome&logoColor=white" alt="Chrome Web Store"></a>
</p>

Ultra-fast binary serializer, streaming frame engine, and decoder for the ⚡**B-FAST** format in TypeScript & JavaScript.

Supports **Node.js**, **Browsers**, **Cloudflare Workers**, **Deno**, and **Bun** with **Dual Module (ESM & CJS)** and **transparent zero-config compression**.

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
    compress: true, // optional compression
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

// Or with compression enabled
const compressedBytes = BFastEncoder.encode(payload, { compress: true });
```

### Binary Decoding (`BFastDecoder`)

```typescript
import { BFastDecoder } from 'bfast-client';

// Decodes ArrayBuffer or Uint8Array
const data = BFastDecoder.decode<User>(buffer);

// Direct TypedArrays for numerical arrays (Float64Array, etc.)
const fastNumeric = BFastDecoder.decode(buffer, { typedArrays: true });
```

---

## ⚡ Transparent Compression (Zero-Config)

`bfast-client` handles compression and decompression automatically:

- **Zero Configuration:** No bundler plugins (Vite, Webpack, Next.js), separate asset files, or native build steps required.
- **Automatic Detection:** Any compressed payload (`compress: true`) is detected and unpacked on the fly.
- **Universal:** Works out of the box across Browsers, Node.js, Bun, Deno, and Edge Workers.

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

## Columnar Data Access: `BFastTable`

```typescript
import { BFastTable } from 'bfast-client';

const table = BFastTable.from(buffer);

// Direct column views
const prices = table.getFloat64Array('price');
const ids = table.getInt32Array('id');
const actives = table.getBooleanArray('active');

// Fast aggregations ('sum', 'avg', 'min', 'max', 'count')
const totalPrice = table.aggregate('price', 'sum');
const avgPrice = table.aggregate('price', 'avg');

// Vectorized filter (returns matching row indices)
const filteredIndices = table.filter('price', '>', 50.0);

// On-demand string access
const name = table.getString('name', 0);

// Release WebAssembly memory when done
table.free();
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
- **Compression**: Automatic compression & decompression

---

## 🔍 DevTools Extension: B-FAST Inspector

Debug binary payloads in your browser as easily as JSON. The official **[B-FAST Inspector](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)** extension adds a dedicated panel to your browser DevTools:

- Intercepts `application/x-bfast` network responses automatically.
- Displays interactive, expandable data trees.
- Live measurements for decode time and bandwidth savings.

Install from the **[Chrome Web Store](https://chromewebstore.google.com/detail/b-fast-inspector/nbjopcagmflkmgfppkeijcpbkobkgehp)**.

---

## License

MIT
