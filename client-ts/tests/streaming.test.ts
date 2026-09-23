import test from 'node:test';
import assert from 'node:assert';
import {
    BFastDecoder,
    BFastEncoder,
    bfastFetch,
    BFastStreamDecoder,
    BFastStreamEncoder,
    decodeStream,
    decodeReadableStream,
    decodeMcpResource,
    STREAM_VERSION,
    FRAME_DATA,
    FRAME_EOS,
    initWasmLz4Sync,
    isWasmEnabled,
} from '../index';

// Helper to create a dummy valid B-FAST payload
function createSimpleBfastPayload(message: string): Uint8Array {
    // Magic 'BF' (2B) + stringTableCount 1 (2B LE) + [len 1B][string] + tag 0x50 [len 4B][msg]
    const encoder = new TextEncoder();
    const strBytes = encoder.encode(message);
    const buf = new Uint8Array(6 + 1 + 3 + 1 + 4 + strBytes.length);
    const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);

    // Magic 'BF'
    buf[0] = 0x42;
    buf[1] = 0x46;
    buf[2] = 0x00; // flags
    buf[3] = 0x01; // version
    view.setUint16(4, 1, true); // 1 string in table

    // String table entry: "msg"
    buf[6] = 3;
    buf[7] = 0x6d; // 'm'
    buf[8] = 0x73; // 's'
    buf[9] = 0x67; // 'g'

    // Value: Raw String (0x50)
    buf[10] = 0x50;
    view.setUint32(11, strBytes.length, true);
    buf.set(strBytes, 15);

    return buf;
}

test('BFastStreamEncoder static methods', () => {
    const handshake = BFastStreamEncoder.getHandshake();
    assert.strictEqual(handshake.length, 4);
    assert.strictEqual(handshake[0], 0x42);
    assert.strictEqual(handshake[1], 0x53);
    assert.strictEqual(handshake[2], STREAM_VERSION);

    const eos = BFastStreamEncoder.getEosFrame();
    assert.strictEqual(eos.length, 6);
    assert.strictEqual(eos[4], FRAME_EOS);

    const dummyPacket = new Uint8Array([0x42, 0x46, 0x00, 0x01, 0x00, 0x00]);
    const frame = BFastStreamEncoder.encodeFrame(dummyPacket);
    assert.strictEqual(frame.length, 6 + dummyPacket.length);
    const view = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
    assert.strictEqual(view.getUint32(0, true), dummyPacket.length);
    assert.strictEqual(frame[4], FRAME_DATA);
});

test('BFastStreamDecoder basic feed and handshake', () => {
    const decoder = new BFastStreamDecoder();
    const dummyPacket = createSimpleBfastPayload('hello world');
    const frame = BFastStreamEncoder.encodeFrame(dummyPacket);
    const handshake = BFastStreamEncoder.getHandshake();

    // Combine handshake + frame
    const streamData = new Uint8Array(handshake.length + frame.length);
    streamData.set(handshake, 0);
    streamData.set(frame, handshake.length);

    const items = decoder.feed(streamData);
    assert.strictEqual(items.length, 1);
    assert.strictEqual(items[0], 'hello world');
    assert.strictEqual(decoder.handshakeReceived, true);
    assert.strictEqual(decoder.isEos, false);
    assert.strictEqual(decoder.pendingBytes, 0);
});

test('BFastStreamDecoder raw frames without handshake', () => {
    const decoder = new BFastStreamDecoder();
    const dummyPacket = createSimpleBfastPayload('raw frame');
    const frame = BFastStreamEncoder.encodeFrame(dummyPacket);

    const items = decoder.feed(frame);
    assert.strictEqual(items.length, 1);
    assert.strictEqual(items[0], 'raw frame');
    assert.strictEqual(decoder.handshakeReceived, true);
});

test('BFastStreamDecoder with explicit expectHandshake', () => {
    const dummyPacket = createSimpleBfastPayload('explicit handshake test');
    const frame = BFastStreamEncoder.encodeFrame(dummyPacket);

    // expectHandshake = true: should throw if no handshake
    const strictDecoder = new BFastStreamDecoder(undefined, true);
    assert.throws(
        () => strictDecoder.feed(frame),
        /Expected stream handshake 'BS' not found/
    );

    // expectHandshake = false: directly decodes raw frames
    const noHandshakeDecoder = new BFastStreamDecoder(undefined, false);
    assert.strictEqual(noHandshakeDecoder.handshakeReceived, true);
    const items = noHandshakeDecoder.feed(frame);
    assert.strictEqual(items.length, 1);
    assert.strictEqual(items[0], 'explicit handshake test');
});

test('BFastStreamDecoder unsupported stream version error', () => {
    const decoder = new BFastStreamDecoder();
    const badHandshake = new Uint8Array([0x42, 0x53, 0x99, 0x00]); // version 0x99

    assert.throws(
        () => decoder.feed(badHandshake),
        /Unsupported stream version: 153/
    );
});

test('BFastStreamDecoder extreme byte-by-byte fragmentation', () => {
    const decoder = new BFastStreamDecoder();
    const messages = ['chunk_1', 'chunk_2', 'chunk_3', 'chunk_4'];

    const handshake = BFastStreamEncoder.getHandshake();
    const frames = messages.map(msg => BFastStreamEncoder.encodeFrame(createSimpleBfastPayload(msg)));
    const eos = BFastStreamEncoder.getEosFrame();

    const totalLen = handshake.length + frames.reduce((a, b) => a + b.length, 0) + eos.length;
    const stream = new Uint8Array(totalLen);
    let offset = 0;
    stream.set(handshake, offset);
    offset += handshake.length;
    for (const f of frames) {
        stream.set(f, offset);
        offset += f.length;
    }
    stream.set(eos, offset);

    const results: any[] = [];
    // Feed 1 single byte at a time
    for (let i = 0; i < stream.length; i++) {
        const chunk = stream.subarray(i, i + 1);
        const decoded = decoder.feed(chunk);
        results.push(...decoded);
    }

    assert.deepStrictEqual(results, messages);
    assert.strictEqual(decoder.isEos, true);
    assert.strictEqual(decoder.pendingBytes, 0);
});

test('BFastStreamDecoder maxFrameSize security protection', () => {
    const decoder = new BFastStreamDecoder(128); // max 128 bytes
    const maliciousHeader = new Uint8Array(6);
    const view = new DataView(maliciousHeader.buffer);
    view.setUint32(0, 1024, true); // claims 1024 bytes
    maliciousHeader[4] = FRAME_DATA;

    assert.throws(
        () => decoder.feed(maliciousHeader),
        /exceeds maximum allowed 128/
    );
    assert.strictEqual(decoder.pendingBytes, 0);
});

test('BFastStreamDecoder unknown frame type error', () => {
    const decoder = new BFastStreamDecoder();
    const badFrame = new Uint8Array([0x02, 0x00, 0x00, 0x00, 0x77, 0x00, 0xaa, 0xbb]);

    assert.throws(
        () => decoder.feed(badFrame),
        /Unknown frame type: 0x77/
    );
});

test('BFastStreamDecoder clear and reset', () => {
    const decoder = new BFastStreamDecoder();
    assert.deepStrictEqual(decoder.feed(new Uint8Array(0)), []);

    decoder.feed(BFastStreamEncoder.getHandshake());
    assert.strictEqual(decoder.handshakeReceived, true);

    decoder.clear();
    assert.strictEqual(decoder.handshakeReceived, false);
    assert.strictEqual(decoder.pendingBytes, 0);

    decoder.reset();
    assert.strictEqual(decoder.isEos, false);
});

test('decodeReadableStream async iteration', async () => {
    const messages = ['stream_item_A', 'stream_item_B'];
    const frames = messages.map(m => BFastStreamEncoder.encodeFrame(createSimpleBfastPayload(m)));
    const chunks = [
        BFastStreamEncoder.getHandshake(),
        frames[0],
        frames[1],
        BFastStreamEncoder.getEosFrame(),
    ];

    let chunkIdx = 0;
    const readable = new ReadableStream<Uint8Array>({
        pull(controller) {
            if (chunkIdx < chunks.length) {
                controller.enqueue(chunks[chunkIdx++]);
            } else {
                controller.close();
            }
        }
    });

    const results: any[] = [];
    for await (const item of decodeReadableStream(readable)) {
        results.push(item);
    }

    assert.deepStrictEqual(results, messages);
});

test('decodeStream with async iterable', async () => {
    const messages = ['async_1', 'async_2'];
    const frames = messages.map(m => BFastStreamEncoder.encodeFrame(createSimpleBfastPayload(m)));
    const chunks = [
        BFastStreamEncoder.getHandshake(),
        frames[0],
        frames[1],
        BFastStreamEncoder.getEosFrame(),
    ];

    async function* asyncGen() {
        for (const c of chunks) {
            yield c;
        }
    }

    const results: any[] = [];
    for await (const item of decodeStream(asyncGen())) {
        results.push(item);
    }

    assert.deepStrictEqual(results, messages);

    // Invalid stream argument
    await assert.rejects(async () => {
        for await (const _ of decodeStream(12345 as any)) {
            // should throw
        }
    }, /Input must be a ReadableStream or an async iterable/);
});

test('BFastDecoder with DecodeOptions and typedArrays', () => {
    // Create a payload with tag 0x90 (f64 numpy array): 2 elements [1.5, 2.5]
    const buf = new Uint8Array(6 + 1 + 4 + 16);
    const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
    // Header
    buf[0] = 0x42;
    buf[1] = 0x46;
    buf[2] = 0x00;
    buf[3] = 0x01;
    view.setUint16(4, 0, true); // stringTableCount = 0

    // Tag 0x90
    buf[6] = 0x90;
    view.setUint32(7, 2, true); // 2 elements
    view.setFloat64(11, 1.5, true);
    view.setFloat64(19, 2.5, true);

    // Default: returns Array of numbers
    const regular = BFastDecoder.decode(buf);
    assert.ok(Array.isArray(regular));
    assert.deepStrictEqual(regular, [1.5, 2.5]);

    // With typedArrays: true -> returns Float64Array
    const typed = BFastDecoder.decode<Float64Array>(buf, { typedArrays: true });
    assert.ok(typed instanceof Float64Array);
    assert.strictEqual(typed.length, 2);
    assert.strictEqual(typed[0], 1.5);
    assert.strictEqual(typed[1], 2.5);
});

test('BFastEncoder round-trip with complex nested object', () => {
    const original = {
        id: 42,
        name: 'Alice',
        score: 95.5,
        active: true,
        tags: ['binary', 'fast'],
        created: new Date('2026-09-22T10:00:00.000Z'),
        matrix: new Float64Array([10.5, 20.5]),
    };

    const encoded = BFastEncoder.encode(original);
    assert.ok(encoded instanceof Uint8Array);
    assert.ok(encoded.length > 0);

    const decoded = BFastDecoder.decode(encoded, { typedArrays: true });
    assert.strictEqual(decoded.id, 42);
    assert.strictEqual(decoded.name, 'Alice');
    assert.strictEqual(decoded.score, 95.5);
    assert.strictEqual(decoded.active, true);
    assert.deepStrictEqual(decoded.tags, ['binary', 'fast']);
    assert.ok(decoded.created instanceof Date);
    assert.strictEqual(decoded.created.toISOString(), '2026-09-22T10:00:00.000Z');
    assert.ok(decoded.matrix instanceof Float64Array);
    assert.strictEqual(decoded.matrix[0], 10.5);
    assert.strictEqual(decoded.matrix[1], 20.5);
});

test('BFastEncoder with compress option (auto-activates WebAssembly LZ4)', () => {
    const largeData = {
        users: Array.from({ length: 200 }, (_, i) => ({
            id: i,
            name: `User ${i}`,
            role: 'developer',
            status: 'active',
        })),
    };

    const uncompressed = BFastEncoder.encode(largeData, { compress: false });
    const compressed = BFastEncoder.encode(largeData, { compress: true });

    assert.ok(compressed.length < uncompressed.length);

    // Decoding a compressed payload auto-initializes WebAssembly LZ4
    const decoded = BFastDecoder.decode(compressed);
    assert.strictEqual(isWasmEnabled(), true);
    assert.strictEqual(decoded.users.length, 200);
    assert.strictEqual(decoded.users[0].name, 'User 0');
    assert.strictEqual(decoded.users[199].name, 'User 199');
});

test('BFastStreamEncoder.encodeFrame with JS objects', () => {
    const obj = { message: 'streaming item', count: 123 };
    const frame = BFastStreamEncoder.encodeFrame(obj);

    const decoder = new BFastStreamDecoder(undefined, false);
    const items = decoder.feed(frame);

    assert.strictEqual(items.length, 1);
    assert.strictEqual(items[0].message, 'streaming item');
    assert.strictEqual(items[0].count, 123);
});

test('WebAssembly LZ4 acceleration', () => {
    const success = initWasmLz4Sync();
    assert.strictEqual(success, true);
    assert.strictEqual(isWasmEnabled(), true);

    const largeData = {
        items: Array.from({ length: 500 }, (_, i) => ({
            id: i,
            text: `Repeated sample string for compression testing ${i}`,
        })),
    };

    const compressed = BFastEncoder.encode(largeData, { compress: true });
    // Decode will now use WASM decompressor
    const decoded = BFastDecoder.decode(compressed);
    assert.strictEqual(decoded.items.length, 500);
    assert.strictEqual(decoded.items[499].id, 499);
});

test('bfastFetch helper with mock fetch', async () => {
    const originalFetch = globalThis.fetch;
    const testPayload = { id: 100, status: 'ok' };
    const encodedResponse = BFastEncoder.encode(testPayload);

    let capturedHeaders: Headers | undefined;
    let capturedBody: any;

    (globalThis as any).fetch = async (_input: any, init?: any) => {
        capturedHeaders = init?.headers;
        capturedBody = init?.body;
        return {
            ok: true,
            status: 200,
            statusText: 'OK',
            arrayBuffer: async () => encodedResponse.buffer,
        };
    };

    try {
        const result = await bfastFetch<{ id: number; status: string }>('https://api.example.com/data', {
            method: 'POST',
            body: { send: 'data' },
        });

        assert.strictEqual(result.id, 100);
        assert.strictEqual(result.status, 'ok');
        assert.ok(capturedHeaders?.get('Accept')?.includes('application/x-bfast'));
        assert.strictEqual(capturedHeaders?.get('Content-Type'), 'application/x-bfast');
        assert.ok(capturedBody instanceof Uint8Array);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('decodeMcpResource with MCP tool result and base64 blob', () => {
    const original = { id: 777, label: 'mcp-test', tags: ['fast', 'mcp'] };
    const encoded = BFastEncoder.encode(original, { compress: true });
    const b64 = Buffer.from(encoded).toString('base64');

    // 1. Full MCP CallToolResult object
    const mcpResult = {
        content: [
            { type: 'text', text: '[B-FAST binary payload in embedded resource]' },
            {
                type: 'resource',
                resource: {
                    uri: 'bfast://tools/query/output',
                    mimeType: 'application/x-bfast',
                    blob: b64,
                },
            },
        ],
    };

    const decoded = decodeMcpResource(mcpResult);
    assert.strictEqual(decoded.id, 777);
    assert.strictEqual(decoded.label, 'mcp-test');
    assert.deepStrictEqual(decoded.tags, ['fast', 'mcp']);

    // 2. Direct array of content items
    const fromArray = decodeMcpResource(mcpResult.content);
    assert.deepStrictEqual(fromArray, original);

    // 3. EmbeddedResource directly
    const fromResource = decodeMcpResource(mcpResult.content[1]);
    assert.deepStrictEqual(fromResource, original);

    // 4. Base64 string directly
    const fromB64 = decodeMcpResource(b64);
    assert.deepStrictEqual(fromB64, original);
});




