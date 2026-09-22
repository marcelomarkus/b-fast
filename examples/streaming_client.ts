/**
 * Exemplo de Cliente TypeScript consumindo B-FAST Streamable HTTP no Navegador ou Node.js.
 * 
 * Requer:
 *     npm install bfast-client
 */

import { decodeStream, decodeReadableStream } from 'bfast-client';

// ------------------------------------------------------------------
// 1. Uso no Navegador com Fetch nativo (ReadableStream)
// ------------------------------------------------------------------
async function consumeInBrowser() {
    const response = await fetch('http://127.0.0.1:8000/telemetry/stream', {
        headers: {
            'Accept': 'application/x-bfast-stream'
        }
    });

    if (!response.body) {
        throw new Error('ReadableStream não suportado pelo navegador');
    }

    console.log('📡 Conectado ao stream B-FAST no browser...');

    // Itera diretamente sobre o body do fetch de forma progressiva
    for await (const reading of decodeReadableStream(response.body)) {
        console.log('Leitura em tempo real:', reading);
        // Exemplo: atualizar gráfico na interface do usuário
        // updateChart(reading.sensor_id, reading.temperature);
    }

    console.log('🏁 Stream finalizado com sucesso!');
}

// ------------------------------------------------------------------
// 2. Uso Universal (Node.js 18+ ou Browser) com decodeStream
// ------------------------------------------------------------------
async function consumeUniversal() {
    const response = await fetch('http://127.0.0.1:8000/database/export');

    console.log('📦 Baixando grandes lotes de dados em stream...');

    let totalRecords = 0;
    for await (const batch of decodeStream(response.body)) {
        totalRecords += Array.isArray(batch) ? batch.length : 1;
        console.log(`Recebido lote com ${batch.length} itens. Total acumulado: ${totalRecords}`);
    }
}
