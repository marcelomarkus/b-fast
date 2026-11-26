import test from 'node:test';
import assert from 'node:assert';
import { BFastEncoder } from '../index';
import {
    decodeInWorker,
    BFastWorkerPool,
    bfastFetchWorker,
    terminateSharedWorkerPool,
} from '../worker';

test('bfast-client/worker: decode basic object in worker thread', async () => {
    const payload = {
        name: 'Mesh3D',
        vertexCount: 3,
        vertices: [0.0, 1.0, 2.0],
        active: true,
    };

    const encoded = BFastEncoder.encode(payload, { compress: true });
    const decoded = await decodeInWorker(encoded);

    assert.deepStrictEqual(decoded, payload);
});

test('bfast-client/worker: decode large dataset with 0-copy ArrayBuffer transfer', async () => {
    const points: number[] = [];
    for (let i = 0; i < 10000; i++) {
        points.push(i * 0.5);
    }
    const payload = {
        modelId: 'stanford-bunny-simulation',
        points,
    };

    const encoded = BFastEncoder.encode(payload, { compress: true });
    const buffer = encoded.buffer.slice(encoded.byteOffset, encoded.byteOffset + encoded.byteLength);

    const decoded = await decodeInWorker(buffer as ArrayBuffer);
    assert.strictEqual(decoded.modelId, 'stanford-bunny-simulation');
    assert.strictEqual(decoded.points.length, 10000);
    assert.strictEqual(decoded.points[10], 5.0);
});

test('bfast-client/worker: custom BFastWorkerPool sizing and lifecycle', async () => {
    const pool = new BFastWorkerPool({ size: 2 });
    const payload1 = { id: 1, type: 'geometry' };
    const payload2 = { id: 2, type: 'normals' };

    const enc1 = BFastEncoder.encode(payload1);
    const enc2 = BFastEncoder.encode(payload2);

    const [res1, res2] = await Promise.all([
        pool.decode(enc1),
        pool.decode(enc2),
    ]);

    assert.deepStrictEqual(res1, payload1);
    assert.deepStrictEqual(res2, payload2);

    pool.terminate();

    await assert.rejects(async () => {
        await pool.decode(enc1);
    }, /terminated/);
});

test('bfast-client/worker: bfastFetchWorker handles both B-FAST and JSON transparently', async () => {
    const bfastPayload = { mesh: 'cube', vertices: 8 };
    const bfastBinary = BFastEncoder.encode(bfastPayload, { compress: true });

    // Mock fetch for B-FAST binary
    const mockBfastFetch: typeof fetch = async (_input: RequestInfo | URL) => {
        return new Response(bfastBinary as any, {
            status: 200,
            headers: { 'Content-Type': 'application/x-bfast' },
        });
    };

    // Replace global fetch temporarily
    const originalFetch = globalThis.fetch;
    try {
        globalThis.fetch = mockBfastFetch;
        const bfastResult = await bfastFetchWorker('/api/3d-mesh');
        assert.deepStrictEqual(bfastResult, bfastPayload);

        // Mock fetch for standard JSON
        const jsonPayload = { status: 'fallback', format: 'json' };
        globalThis.fetch = async () => {
            return new Response(JSON.stringify(jsonPayload), {
                status: 200,
                headers: { 'Content-Type': 'application/json' },
            });
        };

        const jsonResult = await bfastFetchWorker('/api/legacy-endpoint');
        assert.deepStrictEqual(jsonResult, jsonPayload);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('bfast-client/worker: error handling for corrupted buffer', async () => {
    const corruptBuffer = new Uint8Array([0x00, 0x01, 0x02, 0x03, 0x04]);

    await assert.rejects(async () => {
        await decodeInWorker(corruptBuffer);
    });
});

test('cleanup: terminate global shared worker pool', () => {
    terminateSharedWorkerPool();
});
