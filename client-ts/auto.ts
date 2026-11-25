import { BFastDecoder } from './index';

/**
 * Event detail emitted when a B-FAST response is intercepted and decoded.
 */
export interface BFastResponseEventDetail {
    url: string;
    status: number;
    contentLength?: number;
    originalLength?: number;
    durationMs: number;
    timestamp: number;
}

export type BFastResponseListener = (detail: BFastResponseEventDetail) => void;

const listeners = new Set<BFastResponseListener>();

/**
 * Register a listener for B-FAST response metrics.
 * Returns an unsubscribe function.
 */
export function onBFastResponse(callback: BFastResponseListener): () => void {
    listeners.add(callback);
    return () => {
        listeners.delete(callback);
    };
}

interface GlobalScope {
    fetch?: typeof fetch;
    [key: string]: any;
}

let originalFetchRef: typeof fetch | null = null;
let currentScope: GlobalScope | null = null;

function patchResponseIfBFast(response: Response, input: RequestInfo | URL): Response {
    const contentType = response.headers.get('content-type') || '';
    if (!contentType.includes('application/x-bfast') && !contentType.includes('application/vnd.bfast')) {
        return response;
    }

    // Mark response as B-FAST accelerated
    Object.defineProperty(response, 'isBFast', {
        value: true,
        writable: true,
        configurable: true,
        enumerable: true,
    });

    let cachedParsed: any = undefined;
    let hasParsed = false;
    const originalClone = response.clone.bind(response);

    response.json = async function <T = any>(): Promise<T> {
        if (hasParsed) {
            return cachedParsed as T;
        }

        const startTime = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const buffer = await response.arrayBuffer();
        cachedParsed = BFastDecoder.decode<T>(buffer);
        hasParsed = true;

        const endTime = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const durationMs = Math.round((endTime - startTime) * 100) / 100;

        const origHeader = response.headers.get('x-bfast-original-length');
        const originalLength = origHeader ? Number(origHeader) : undefined;
        const clHeader = response.headers.get('content-length');
        const contentLength = clHeader ? Number(clHeader) : buffer.byteLength;

        let url = '';
        try {
            if (response.url) {
                url = response.url;
            } else if (typeof input === 'string') {
                url = input;
            } else if (typeof URL !== 'undefined' && input instanceof URL) {
                url = input.href;
            } else if (typeof Request !== 'undefined' && input instanceof Request) {
                url = input.url;
            }
        } catch {
            url = String(input);
        }

        const detail: BFastResponseEventDetail = {
            url,
            status: response.status,
            contentLength,
            originalLength,
            durationMs,
            timestamp: Date.now(),
        };

        // Notify in-process listeners
        for (const listener of listeners) {
            try {
                listener(detail);
            } catch {
                // Ignore listener errors
            }
        }

        // Notify browser window for DevTools extension / Inspector
        if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
            try {
                window.dispatchEvent(new CustomEvent('bfast:response', { detail }));
            } catch {
                // Ignore CustomEvent errors
            }
        }

        return cachedParsed as T;
    };

    response.clone = function (): Response {
        const cloned = originalClone();
        return patchResponseIfBFast(cloned, input);
    };

    return response;
}

/**
 * Check if the transparent B-FAST fetch monkeypatch is currently active.
 */
export function isBFastAutoInstalled(): boolean {
    return originalFetchRef !== null;
}

/**
 * Install transparent B-FAST fetch acceleration.
 * Automatically adds B-FAST Accept headers and transparently decodes .json() when server responds with B-FAST.
 */
export function installBFastAuto(target?: GlobalScope): void {
    const scope: GlobalScope =
        target ||
        (typeof globalThis !== 'undefined'
            ? globalThis
            : typeof window !== 'undefined'
              ? window
              : typeof global !== 'undefined'
                ? (global as any)
                : null);

    if (!scope || typeof scope.fetch !== 'function') {
        return;
    }

    if (originalFetchRef !== null) {
        return; // Already installed
    }

    originalFetchRef = scope.fetch;
    currentScope = scope;

    const bfastAccept = 'application/x-bfast, application/json;q=0.9, */*;q=0.8';

    scope.fetch = async function (
        input: RequestInfo | URL,
        init?: RequestInit
    ): Promise<Response> {
        let headers: Headers;
        const isInputRequest = typeof Request !== 'undefined' && input instanceof Request;

        if (init?.headers) {
            headers = new Headers(init.headers);
        } else if (isInputRequest) {
            headers = new Headers((input as Request).headers);
        } else {
            headers = new Headers();
        }

        const accept = headers.get('Accept') || '';
        if (!accept) {
            headers.set('Accept', bfastAccept);
        } else if (!accept.includes('application/x-bfast')) {
            headers.set('Accept', `application/x-bfast, ${accept}`);
        }

        let response: Response;
        if (isInputRequest && !init) {
            const req = new Request(input as Request, { headers });
            response = await originalFetchRef!(req);
        } else {
            response = await originalFetchRef!(input, {
                ...init,
                headers,
            });
        }

        return patchResponseIfBFast(response, input);
    };
}

/**
 * Uninstall the B-FAST fetch monkeypatch, restoring the original fetch implementation.
 */
export function uninstallBFastAuto(): void {
    if (currentScope && originalFetchRef) {
        currentScope.fetch = originalFetchRef;
        originalFetchRef = null;
        currentScope = null;
    }
}

// Auto-install by default on import
installBFastAuto();
