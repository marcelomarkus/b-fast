# Frontend TypeScript - Consumindo B-FAST

## Instalação
```bash
npm install bfast-client
```

## Uso Básico

### Decodificação Simples
```typescript
import { BFastDecoder } from 'bfast-client';

async function fetchData() {
    const response = await fetch('/api/users');
    const buffer = await response.arrayBuffer();
    
    // Decodifica automaticamente (com descompressão LZ4 se necessário)
    const users = BFastDecoder.decode(buffer);
    console.log(users);
}
```

### Streaming HTTP em Tempo Real (Streamable HTTP) 🌊

Consuma streams contínuos enviados por `BFastStreamingResponse` ou servidores MCP usando `readBFastStream`:

```typescript
import { readBFastStream } from 'bfast-client';

async function streamUsers() {
    const response = await fetch('/api/stream-users', {
        headers: { 'Accept': 'application/x-bfast-stream' }
    });

    if (!response.body) return;

    // Consome frames binários de forma assíncrona conforme chegam pela rede
    for await (const user of readBFastStream(response.body)) {
        console.log('Usuário recebido em tempo real:', user);
    }
}
```

#### Decodificador de Stream de Baixo Nível (`BFastStreamDecoder`)

Para WebSockets, conexões TCP ou controle manual de buffers:

```typescript
import { BFastStreamDecoder } from 'bfast-client';

const decoder = new BFastStreamDecoder({ maxFrameSize: 16 * 1024 * 1024 });

// Recebendo chunks binários (Uint8Array)
function onChunkReceived(chunk: Uint8Array) {
    const items = decoder.feed(chunk);
    for (const item of items) {
        console.log('Item decodificado:', item);
    }
}
```

### React Hook Personalizado
```typescript
import { useState, useEffect } from 'react';
import { BFastDecoder } from 'bfast-client';

function useBFastData<T>(url: string) {
    const [data, setData] = useState<T | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        async function fetchData() {
            try {
                const response = await fetch(url);
                if (!response.ok) throw new Error('Network error');
                
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

// Uso do hook
function UserList() {
    const { data: users, loading, error } = useBFastData<User[]>('/api/users');
    
    if (loading) return <div>Carregando...</div>;
    if (error) return <div>Erro: {error}</div>;
    
    return (
        <ul>
            {users?.map(user => (
                <li key={user.id}>{user.name}</li>
            ))}
        </ul>
    );
}
```

### Axios Interceptor
```typescript
import axios from 'axios';
import { BFastDecoder } from 'bfast-client';

// Configurar interceptor para respostas B-FAST
axios.interceptors.response.use(response => {
    if (response.headers['content-type'] === 'application/x-bfast') {
        response.data = BFastDecoder.decode(response.data);
    }
    return response;
});

// Uso normal do axios
const users = await axios.get('/api/users');
console.log(users.data); // Já decodificado automaticamente
```

### Tratamento de Erros
```typescript
import { BFastDecoder, BFastError } from 'bfast-client';

async function safeDecodeData(buffer: ArrayBuffer) {
    try {
        return BFastDecoder.decode(buffer);
    } catch (error) {
        if (error instanceof BFastError) {
            console.error('Erro de decodificação B-FAST:', error.message);
            // Fallback para JSON se necessário
            const text = new TextDecoder().decode(buffer);
            return JSON.parse(text);
        }
        throw error;
    }
}
```

## Compatibilidade
- **Navegadores:** Chrome 60+, Firefox 55+, Safari 12+, Edge 79+
- **Node.js:** 14.0+
- **Frameworks:** React, Vue, Angular, Svelte

## Próximos Passos
- [Otimização](performance.md) - Dicas para máxima performance
- [Início Rápido](getting_started.md) - Configuração do backend
