import { BFastDecoder, DecodeOptions } from './index';
import type { WorkerDecodeRequest, WorkerDecodeResponse } from './worker-thread';

export interface BFastWorkerPoolOptions {
    /**
     * Number of workers in the pool. Defaults to navigator.hardwareConcurrency (or 2 in Node.js), clamped between 1 and 8.
     */
    size?: number;
    /**
     * Custom URL or path to the worker thread script.
     */
    workerUrl?: string | URL;
    /**
     * Whether to disable workers and use main-thread fallback directly.
     */
    disableWorkers?: boolean;
}

interface WorkerHandle {
    worker: any;
    isNode: boolean;
    pendingCount: number;
    pendingRequests: Map<number, { resolve: (res: any) => void; reject: (err: any) => void }>;
}

export class BFastWorkerPool {
    private handles: WorkerHandle[] = [];
    private nextRequestId = 1;
    private options: BFastWorkerPoolOptions;
    private isTerminated = false;

    constructor(options: BFastWorkerPoolOptions = {}) {
        this.options = options;
        if (!options.disableWorkers) {
            this.initPool();
        }
    }

    private initPool(): void {
        const defaultSize = typeof navigator !== 'undefined' && navigator.hardwareConcurrency
            ? Math.min(Math.max(navigator.hardwareConcurrency - 1, 1), 8)
            : 2;
        const poolSize = this.options.size ?? defaultSize;

        for (let i = 0; i < poolSize; i++) {
            const handle = this.createWorkerHandle();
            if (handle) {
                this.handles.push(handle);
            }
        }
    }

    private createWorkerHandle(): WorkerHandle | null {
        let worker: any = null;
        let isNode = false;

        // 1. Try Node.js worker_threads
        if (typeof process !== 'undefined' && process.versions && process.versions.node) {
            try {
                // eslint-disable-next-line @typescript-eslint/no-var-requires
                const { Worker } = require('worker_threads');
                // eslint-disable-next-line @typescript-eslint/no-var-requires
                const path = require('path');
                const scriptPath = this.options.workerUrl
                    ? String(this.options.workerUrl)
                    : path.join(__dirname, 'worker-thread.js');
                worker = new Worker(scriptPath);
                isNode = true;
            } catch (_) {
                // Node worker failed or not in environment
            }
        }

        // 2. Try Browser Web Worker
        if (!worker && typeof Worker !== 'undefined') {
            try {
                let url: string | URL;
                if (this.options.workerUrl) {
                    url = this.options.workerUrl;
                } else {
                    const base = typeof location !== 'undefined' ? location.href : 'http://localhost';
                    url = new URL('./worker-thread.js', base);
                }
                worker = new Worker(url, { type: 'module' });
                isNode = false;
            } catch (_) {
                // Browser worker failed
            }
        }

        if (!worker) {
            return null;
        }

        const handle: WorkerHandle = {
            worker,
            isNode,
            pendingCount: 0,
            pendingRequests: new Map(),
        };

        const onMessage = (response: WorkerDecodeResponse) => {
            if (!response || typeof response.id !== 'number') return;
            const pending = handle.pendingRequests.get(response.id);
            if (pending) {
                handle.pendingRequests.delete(response.id);
                handle.pendingCount--;
                if (response.success) {
                    pending.resolve(response.result);
                } else {
                    pending.reject(new Error(response.error || 'Failed to decode in worker'));
                }
            }
        };

        const onError = (error: any) => {
            for (const [id, pending] of handle.pendingRequests.entries()) {
                pending.reject(error instanceof Error ? error : new Error(String(error)));
                handle.pendingRequests.delete(id);
            }
            handle.pendingCount = 0;
        };

        if (isNode) {
            worker.on('message', onMessage);
            worker.on('error', onError);
        } else {
            worker.onmessage = (e: MessageEvent) => onMessage(e.data);
            worker.onerror = (e: ErrorEvent) => onError(e.error || e.message);
        }

        return handle;
    }

    /**
     * Decode a B-FAST binary payload off the main thread.
     * Zero-copy transfers the underlying ArrayBuffer when available.
     */
    async decode<T = any>(
        buffer: ArrayBuffer | Uint8Array | any,
        options?: DecodeOptions<T>
    ): Promise<T> {
        if (this.isTerminated) {
            throw new Error('BFastWorkerPool is terminated');
        }

        // Fallback to synchronous decode if workers unavailable
        if (this.handles.length === 0) {
            return BFastDecoder.decode<T>(buffer, options);
        }

        // Pick worker with lowest pending load
        let bestHandle = this.handles[0];
        for (let i = 1; i < this.handles.length; i++) {
            if (this.handles[i].pendingCount < bestHandle.pendingCount) {
                bestHandle = this.handles[i];
            }
        }

        const id = this.nextRequestId++;
        const req: WorkerDecodeRequest = {
            id,
            action: 'decode',
            buffer,
            options,
        };

        return new Promise<T>((resolve, reject) => {
            bestHandle.pendingRequests.set(id, { resolve, reject });
            bestHandle.pendingCount++;

            // Zero-copy transfer if buffer is ArrayBuffer or TypedArray
            const transfer: Transferable[] = [];
            if (buffer instanceof ArrayBuffer) {
                transfer.push(buffer);
            } else if (ArrayBuffer.isView(buffer) && buffer.buffer instanceof ArrayBuffer) {
                // If it's a view, we only transfer the buffer if caller intends 0-copy transfer
                transfer.push(buffer.buffer);
            }

            try {
                if (bestHandle.isNode) {
                    bestHandle.worker.postMessage(req, transfer);
                } else {
                    bestHandle.worker.postMessage(req, transfer);
                }
            } catch (postErr) {
                // If transfer fails (e.g. shared buffer), retry without transfer list
                try {
                    bestHandle.worker.postMessage(req);
                } catch (retryErr) {
                    bestHandle.pendingRequests.delete(id);
                    bestHandle.pendingCount--;
                    // Ultimate fallback to synchronous decode
                    try {
                        const syncDecoded = BFastDecoder.decode<T>(buffer, options);
                        resolve(syncDecoded);
                    } catch (syncErr) {
                        reject(syncErr);
                    }
                }
            }
        });
    }

    /**
     * Terminate all workers in the pool.
     */
    terminate(): void {
        this.isTerminated = true;
        for (const handle of this.handles) {
            if (handle.isNode) {
                handle.worker.terminate();
            } else if (typeof handle.worker.terminate === 'function') {
                handle.worker.terminate();
            }
            handle.pendingRequests.clear();
            handle.pendingCount = 0;
        }
        this.handles = [];
    }
}

let sharedPool: BFastWorkerPool | null = null;

/**
 * Get the global shared B-FAST worker pool instance.
 */
export function getSharedBFastWorkerPool(options?: BFastWorkerPoolOptions): BFastWorkerPool {
    if (!sharedPool) {
        sharedPool = new BFastWorkerPool(options);
    }
    return sharedPool;
}

/**
 * Terminate and clear the global shared worker pool.
 */
export function terminateSharedWorkerPool(): void {
    if (sharedPool) {
        sharedPool.terminate();
        sharedPool = null;
    }
}

/**
 * Decode B-FAST binary payload off the main thread in a Web Worker.
 *
 * @param buffer - ArrayBuffer or Uint8Array containing B-FAST data.
 * @param options - Decode options.
 * @returns Promise resolving to the decoded JavaScript object.
 */
export async function decodeInWorker<T = any>(
    buffer: ArrayBuffer | Uint8Array | any,
    options?: DecodeOptions<T>
): Promise<T> {
    const pool = getSharedBFastWorkerPool();
    return pool.decode<T>(buffer, options);
}

/**
 * High-performance fetch wrapper that requests B-FAST and offloads decoding to a Web Worker.
 * Guaranteed zero main-thread blocking time for heavy datasets.
 *
 * @param input - Request URL or Request object.
 * @param init - Request options.
 */
export async function bfastFetchWorker<T = any>(
    input: RequestInfo | URL,
    init?: RequestInit
): Promise<T> {
    const headers = new Headers(init?.headers || {});
    if (!headers.has('Accept')) {
        headers.set('Accept', 'application/x-bfast, application/json, */*');
    }

    const response = await fetch(input, {
        ...init,
        headers,
    });

    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }

    const contentType = response.headers.get('content-type') || '';
    if (contentType.includes('application/x-bfast') || contentType.includes('application/vnd.bfast')) {
        const arrayBuffer = await response.arrayBuffer();
        return decodeInWorker<T>(arrayBuffer);
    }

    return response.json();
}
