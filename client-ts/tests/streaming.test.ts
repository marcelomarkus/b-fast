import test from 'node:test';
import assert from 'node:assert';
import {
    BFastStreamDecoder,
    BFastStreamEncoder,
    decodeStream,
    decodeReadableStream,
    STREAM_VERSION,
    FRAME_DATA,
    FRAME_EOS,
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
