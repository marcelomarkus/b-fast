# @b-fast/client

Client ultra-fast binary decoder for ⚡B-FAST format in TypeScript/JavaScript.

## Installation

```bash
npm install @b-fast/client
```

## Usage

### Basic Decoding

```typescript
import { BFastDecoder } from '@b-fast/client';

// Fetch B-FAST data from API
const response = await fetch('/api/data');
const buffer = await response.arrayBuffer();

// Decode (automatically handles LZ4 decompression)
const data = BFastDecoder.decode(buffer);
console.log(data);
```

### Error Handling

```typescript
import { BFastDecoder, BFastError } from '@b-fast/client';

try {
    const data = BFastDecoder.decode(buffer);
    console.log(data);
} catch (error) {
    if (error instanceof BFastError) {
        console.error('B-FAST decode error:', error.message);
    } else {
        console.error('Unexpected error:', error);
    }
}
```

### React Hook

```typescript
import { useState, useEffect } from 'react';
import { BFastDecoder } from '@b-fast/client';

function useBFastData<T>(url: string) {
    const [data, setData] = useState<T | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        async function fetchData() {
            try {
                const response = await fetch(url);
                const buffer = await response.arrayBuffer();
                const decoded = BFastDecoder.decode(buffer);
                setData(decoded);
            } catch (err) {
                setError(err instanceof Error ? err.message : 'Unknown error');
            } finally {
                setLoading(false);
            }
        }
        fetchData();
    }, [url]);

    return { data, loading, error };
}
```

### Axios Integration

```typescript
import axios from 'axios';
import { BFastDecoder } from '@b-fast/client';

// Configure axios to handle B-FAST responses
axios.interceptors.response.use(response => {
    if (response.headers['content-type'] === 'application/x-bfast') {
        response.data = BFastDecoder.decode(response.data);
    }
    return response;
});

// Use normally
const { data } = await axios.get('/api/users');
console.log(data); // Already decoded
```

## Supported Types

- **Primitives**: `null`, `boolean`, `number`, `string`
- **Collections**: `Array`, `Object`
- **Special Types**: `UUID`, `Date`, NumPy arrays
- **Compression**: Automatic LZ4 decompression detection

## Performance

B-FAST provides significant performance improvements over JSON:

- **15x faster** than standard JSON
- **3x faster** than orjson
- **80% smaller** payload size
- **Zero-copy** NumPy array handling

## Browser Compatibility

| Browser | Version |
|---------|---------|
| Chrome  | 60+     |
| Firefox | 55+     |
| Safari  | 12+     |
| Edge    | 79+     |
| Node.js | 14+     |

## API Reference

### `BFastDecoder.decode(buffer)`

Decodes B-FAST binary data to JavaScript objects.

**Parameters:**
- `buffer` - `ArrayBuffer` or `Uint8Array` containing B-FAST data

**Returns:**
- Decoded JavaScript object

**Throws:**
- `BFastError` - If decoding fails

### `BFastError`

Custom error class for B-FAST decoding errors.

## License

MIT
