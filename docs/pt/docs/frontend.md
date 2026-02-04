# 🌐 Frontend - Integração TypeScript

Guia completo para integrar B-FAST em aplicações frontend modernas.

## 📦 Instalação

```bash
npm install bfast-client
```

## 🛠️ Configuração Básica

### Importação

```typescript
import { BFastDecoder } from 'bfast-client';
```

### Decodificação Simples

```typescript
async function fetchData() {
    const response = await fetch('/api/data');
    const buffer = await response.arrayBuffer();
    
    const data = BFastDecoder.decode(buffer);
    return data;
}
```

## 🚀 Integração com Frameworks

### React

```tsx
import React, { useState, useEffect } from 'react';
import { BFastDecoder } from 'bfast-client';

interface User {
    id: number;
    name: string;
    email: string;
}

const UserList: React.FC = () => {
    const [users, setUsers] = useState<User[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const loadUsers = async () => {
            try {
                const response = await fetch('/api/users');
                const buffer = await response.arrayBuffer();
                const userData = BFastDecoder.decode(buffer) as User[];
                setUsers(userData);
            } catch (error) {
                console.error('Erro ao carregar usuários:', error);
            } finally {
                setLoading(false);
            }
        };

        loadUsers();
    }, []);

    if (loading) return <div>Carregando...</div>;

    return (
        <ul>
            {users.map(user => (
                <li key={user.id}>{user.name} - {user.email}</li>
            ))}
        </ul>
    );
};
```

### Vue.js

```vue
<template>
  <div>
    <div v-if="loading">Carregando...</div>
    <ul v-else>
      <li v-for="user in users" :key="user.id">
        {{ user.name }} - {{ user.email }}
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { BFastDecoder } from 'bfast-client';

interface User {
  id: number;
  name: string;
  email: string;
}

const users = ref<User[]>([]);
const loading = ref(true);

const loadUsers = async () => {
  try {
    const response = await fetch('/api/users');
    const buffer = await response.arrayBuffer();
    const userData = BFastDecoder.decode(buffer) as User[];
    users.value = userData;
  } catch (error) {
    console.error('Erro ao carregar usuários:', error);
  } finally {
    loading.value = false;
  }
};

onMounted(loadUsers);
</script>
```

### Angular

```typescript
import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { BFastDecoder } from 'bfast-client';
import { Observable, from } from 'rxjs';

export interface User {
  id: number;
  name: string;
  email: string;
}

@Injectable({
  providedIn: 'root'
})
export class UserService {
  constructor(private http: HttpClient) {}

  getUsers(): Observable<User[]> {
    return from(this.loadUsers());
  }

  private async loadUsers(): Promise<User[]> {
    const response = await fetch('/api/users');
    const buffer = await response.arrayBuffer();
    return BFastDecoder.decode(buffer) as User[];
  }
}
```

## 🔧 Configurações Avançadas

### Tratamento de Erros

```typescript
async function safeDecode(buffer: ArrayBuffer) {
    try {
        return BFastDecoder.decode(buffer);
    } catch (error) {
        console.error('Erro na decodificação B-FAST:', error);
        
        // Fallback para JSON se necessário
        const text = new TextDecoder().decode(buffer);
        return JSON.parse(text);
    }
}
```

### Verificação de Tipo de Conteúdo

```typescript
async function fetchWithBFast(url: string) {
    const response = await fetch(url);
    const contentType = response.headers.get('content-type');
    
    if (contentType === 'application/x-bfast') {
        const buffer = await response.arrayBuffer();
        return BFastDecoder.decode(buffer);
    } else {
        return await response.json();
    }
}
```

### Cache com Service Worker

```typescript
// service-worker.ts
self.addEventListener('fetch', (event) => {
    if (event.request.url.includes('/api/')) {
        event.respondWith(handleBFastRequest(event.request));
    }
});

async function handleBFastRequest(request: Request) {
    const cache = await caches.open('bfast-cache');
    const cachedResponse = await cache.match(request);
    
    if (cachedResponse) {
        return cachedResponse;
    }
    
    const response = await fetch(request);
    
    // Cache apenas respostas B-FAST
    if (response.headers.get('content-type') === 'application/x-bfast') {
        cache.put(request, response.clone());
    }
    
    return response;
}
```

## 📊 Monitoramento de Performance

### Métricas de Transferência

```typescript
async function fetchWithMetrics(url: string) {
    const startTime = performance.now();
    
    const response = await fetch(url);
    const buffer = await response.arrayBuffer();
    
    const transferTime = performance.now() - startTime;
    const payloadSize = buffer.byteLength;
    
    console.log(`Transfer: ${transferTime.toFixed(2)}ms, Size: ${payloadSize} bytes`);
    
    const decodeStart = performance.now();
    const data = BFastDecoder.decode(buffer);
    const decodeTime = performance.now() - decodeStart;
    
    console.log(`Decode: ${decodeTime.toFixed(2)}ms`);
    
    return data;
}
```

### Comparação com JSON

```typescript
async function compareFormats(url: string) {
    // B-FAST
    const bfastStart = performance.now();
    const bfastResponse = await fetch(url);
    const bfastBuffer = await bfastResponse.arrayBuffer();
    const bfastData = BFastDecoder.decode(bfastBuffer);
    const bfastTime = performance.now() - bfastStart;
    
    // JSON (para comparação)
    const jsonStart = performance.now();
    const jsonResponse = await fetch(url.replace('/api/', '/api/json/'));
    const jsonData = await jsonResponse.json();
    const jsonTime = performance.now() - jsonStart;
    
    console.log(`B-FAST: ${bfastTime.toFixed(2)}ms (${bfastBuffer.byteLength} bytes)`);
    console.log(`JSON: ${jsonTime.toFixed(2)}ms (${JSON.stringify(jsonData).length} bytes)`);
    console.log(`Speedup: ${(jsonTime / bfastTime).toFixed(1)}x`);
    
    return bfastData;
}
```

## 🎯 Casos de Uso Específicos

### Dashboards em Tempo Real

```typescript
class RealtimeDashboard {
    private ws: WebSocket;
    
    constructor(wsUrl: string) {
        this.ws = new WebSocket(wsUrl);
        this.ws.binaryType = 'arraybuffer';
        
        this.ws.onmessage = (event) => {
            if (event.data instanceof ArrayBuffer) {
                const data = BFastDecoder.decode(event.data);
                this.updateDashboard(data);
            }
        };
    }
    
    private updateDashboard(data: any) {
        // Atualizar componentes do dashboard
        console.log('Dashboard atualizado:', data);
    }
}
```

### Aplicações Mobile (React Native)

```typescript
import { BFastDecoder } from 'bfast-client';

// Otimizado para conexões móveis lentas
async function fetchMobileOptimized(url: string) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 10000); // 10s timeout
    
    try {
        const response = await fetch(url, {
            signal: controller.signal,
            headers: {
                'Accept': 'application/x-bfast', // Preferir B-FAST
                'Accept-Encoding': 'gzip, deflate' // Compressão adicional
            }
        });
        
        clearTimeout(timeoutId);
        
        const buffer = await response.arrayBuffer();
        return BFastDecoder.decode(buffer);
    } catch (error) {
        clearTimeout(timeoutId);
        throw error;
    }
}
```

## 🔍 Debugging

### Inspeção de Dados

```typescript
function debugBFast(buffer: ArrayBuffer) {
    console.log('Buffer size:', buffer.byteLength);
    console.log('First 16 bytes:', new Uint8Array(buffer.slice(0, 16)));
    
    try {
        const data = BFastDecoder.decode(buffer);
        console.log('Decoded successfully:', data);
        return data;
    } catch (error) {
        console.error('Decode failed:', error);
        return null;
    }
}
```

## 📚 Próximos Passos

- [Performance](performance.md) - Análise técnica detalhada
- [Solução de Problemas](troubleshooting.md) - Guia de troubleshooting
- [Início](index.md) - Voltar ao início
