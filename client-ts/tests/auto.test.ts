import { test } from 'node:test';
import * as assert from 'node:assert';
import { BFastEncoder } from '../index';
import {
    installBFastAuto,
    uninstallBFastAuto,
    isBFastAutoInstalled,
    onBFastResponse,
    BFastResponseEventDetail,
} from '../auto';

test('bfast-client/auto: installation lifecycle', () => {
    // Should be installed by default when auto is loaded
    assert.strictEqual(isBFastAutoInstalled(), true);

    uninstallBFastAuto();
    assert.strictEqual(isBFastAutoInstalled(), false);

    installBFastAuto();
    assert.strictEqual(isBFastAutoInstalled(), true);
});

test('bfast-client/auto: transparently intercepts and decodes B-FAST binary response', async () => {
    const originalFetch = globalThis.fetch;
    const testPayload = { id: 42, username: 'marcelo', roles: ['admin', 'developer'] };
    const encodedBFast = BFastEncoder.encode(testPayload, { compress: true });

    let capturedHeaders: Headers | undefined;
    const mockFetch: typeof fetch = async (_input, init) => {
        capturedHeaders = new Headers(init?.headers);
        return new Response(encodedBFast as unknown as BodyInit, {
            status: 200,
            headers: {
                'Content-Type': 'application/x-bfast',
                'Content-Length': String(encodedBFast.byteLength),
                'X-BFast-Original-Length': '128',
            },
        });
    };

    // Install mock
    uninstallBFastAuto();
    globalThis.fetch = mockFetch;
    installBFastAuto();

    let capturedEvent: BFastResponseEventDetail | undefined;
    const unsubscribe = onBFastResponse((detail) => {
        capturedEvent = detail;
    });

    try {
        // Standard user code using fetch
        const response = await fetch('/api/test');

        // Check header injection
        assert.ok(capturedHeaders?.get('Accept')?.includes('application/x-bfast'));

        // Check BFast property
        assert.strictEqual((response as any).isBFast, true);

        // Standard user code calling .json()
        const data = await response.json();
        assert.deepStrictEqual(data, testPayload);

        // Multiple calls to .json() should return cached object
        const dataAgain = await response.json();
        assert.strictEqual(data, dataAgain);

        // Check telemetry event
        assert.ok(capturedEvent !== undefined);
        assert.strictEqual(capturedEvent?.url, '/api/test');
        assert.strictEqual(capturedEvent?.status, 200);
        assert.strictEqual(capturedEvent?.contentLength, encodedBFast.byteLength);
        assert.strictEqual(capturedEvent?.originalLength, 128);
        assert.ok(typeof capturedEvent?.durationMs === 'number');
    } finally {
        unsubscribe();
        uninstallBFastAuto();
        globalThis.fetch = originalFetch;
    }
});

test('bfast-client/auto: leaves non-BFast (standard JSON) responses untouched', async () => {
    const originalFetch = globalThis.fetch;
    const jsonPayload = { message: 'hello from standard json' };

    const mockFetch: typeof fetch = async () => {
        return new Response(JSON.stringify(jsonPayload), {
            status: 200,
            headers: {
                'Content-Type': 'application/json',
            },
        });
    };

    uninstallBFastAuto();
    globalThis.fetch = mockFetch;
    installBFastAuto();

    try {
        const response = await fetch('https://api.example.com/data');
        assert.strictEqual((response as any).isBFast, undefined);

        const data = await response.json();
        assert.deepStrictEqual(data, jsonPayload);
    } finally {
        uninstallBFastAuto();
        globalThis.fetch = originalFetch;
    }
});

test('bfast-client/auto: cloned response preserves B-FAST .json() decoding', async () => {
    const originalFetch = globalThis.fetch;
    const testPayload = { items: [1, 2, 3] };
    const encodedBFast = BFastEncoder.encode(testPayload, { compress: false });

    const mockFetch: typeof fetch = async () => {
        return new Response(encodedBFast as unknown as BodyInit, {
            status: 200,
            headers: {
                'Content-Type': 'application/x-bfast',
            },
        });
    };

    uninstallBFastAuto();
    globalThis.fetch = mockFetch;
    installBFastAuto();

    try {
        const response = await fetch('/api/clone-test');
        const cloned = response.clone();

        const dataOriginal = await response.json();
        const dataCloned = await cloned.json();

        assert.deepStrictEqual(dataOriginal, testPayload);
        assert.deepStrictEqual(dataCloned, testPayload);
    } finally {
        uninstallBFastAuto();
        globalThis.fetch = originalFetch;
    }
});

test('bfast-client/auto: preserves existing Accept headers', async () => {
    const originalFetch = globalThis.fetch;
    let sentAccept = '';

    const mockFetch: typeof fetch = async (_input, init) => {
        const h = new Headers(init?.headers);
        sentAccept = h.get('Accept') || '';
        return new Response(JSON.stringify({ ok: true }), {
            headers: { 'Content-Type': 'application/json' },
        });
    };

    uninstallBFastAuto();
    globalThis.fetch = mockFetch;
    installBFastAuto();

    try {
        await fetch('/api/custom-accept', {
            headers: {
                Accept: 'application/json, text/plain',
            },
        });

        assert.ok(sentAccept.includes('application/x-bfast'));
        assert.ok(sentAccept.includes('application/json, text/plain'));
    } finally {
        uninstallBFastAuto();
        globalThis.fetch = originalFetch;
    }
});
