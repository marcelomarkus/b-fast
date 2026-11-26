import { BFastDecoder, DecodeOptions } from './index';

export interface WorkerDecodeRequest {
    id: number;
    action: 'decode';
    buffer: ArrayBuffer | Uint8Array;
    options?: DecodeOptions;
}

export interface WorkerDecodeResponse {
    id: number;
    success: boolean;
    result?: any;
    error?: string;
    durationMs?: number;
}

/**
 * Handle decode execution inside the worker context.
 */
export function processDecodeMessage(msg: WorkerDecodeRequest): { response: WorkerDecodeResponse; transfer: Transferable[] } {
    const start = performance.now();
    try {
        const decoded = BFastDecoder.decode(msg.buffer, msg.options);
        const durationMs = performance.now() - start;
        const transfer: Transferable[] = [];

        // Collect transferables if decoded payload is or contains typed arrays / ArrayBuffers
        if (decoded) {
            if (ArrayBuffer.isView(decoded) && decoded.buffer instanceof ArrayBuffer) {
                transfer.push(decoded.buffer);
            } else if (decoded instanceof ArrayBuffer) {
                transfer.push(decoded);
            } else if (typeof decoded === 'object') {
                for (const key of Object.keys(decoded)) {
                    const val = (decoded as any)[key];
                    if (val && ArrayBuffer.isView(val) && val.buffer instanceof ArrayBuffer) {
                        transfer.push(val.buffer);
                    }
                }
            }
        }

        return {
            response: {
                id: msg.id,
                success: true,
                result: decoded,
                durationMs,
            },
            transfer,
        };
    } catch (err: any) {
        return {
            response: {
                id: msg.id,
                success: false,
                error: err?.message || String(err),
                durationMs: performance.now() - start,
            },
            transfer: [],
        };
    }
}

// 1. Node.js Worker Thread Initialization
if (typeof process !== 'undefined' && process.versions && process.versions.node) {
    try {
        // eslint-disable-next-line @typescript-eslint/no-var-requires
        const { parentPort } = require('worker_threads');
        if (parentPort) {
            parentPort.on('message', (msg: WorkerDecodeRequest) => {
                if (msg && msg.action === 'decode') {
                    const { response } = processDecodeMessage(msg);
                    parentPort.postMessage(response);
                }
            });
        }
    } catch (_) {
        // Ignored if not running in node worker_threads
    }
}

// 2. Browser Web Worker Initialization
if (
    typeof self !== 'undefined' &&
    typeof (self as any).postMessage === 'function' &&
    typeof window === 'undefined'
) {
    self.onmessage = (event: MessageEvent<WorkerDecodeRequest>) => {
        const msg = event.data;
        if (msg && msg.action === 'decode') {
            const { response, transfer } = processDecodeMessage(msg);
            (self as any).postMessage(response, transfer);
        }
    };
}
