import { test } from 'node:test';
import * as assert from 'node:assert';
import {
    BFastEncoder,
    BFastDecoder,
    BFastStreamEncoder,
    BFastStreamDecoder,
    BFastValidationError,
    validateWithSchema,
    bfastFetch,
    bfastQueryOptions,
    bfastInfiniteQueryOptions,
    SchemaValidator,
} from '../index';

interface User {
    id: number;
    name: string;
    email: string;
}

// 1. Mock Standard Schema (~standard protocol, supported by Zod 3.24+, Valibot, ArkType)
function createStandardUserSchema(): SchemaValidator<User> {
    return {
        '~standard': {
            version: 1,
            vendor: 'test-standard-schema',
            validate: (value: unknown) => {
                if (
                    typeof value === 'object' &&
                    value !== null &&
                    'id' in value &&
                    typeof (value as any).id === 'number' &&
                    'name' in value &&
                    typeof (value as any).name === 'string' &&
                    'email' in value &&
                    typeof (value as any).email === 'string'
                ) {
                    return { value: value as User };
                }
                return {
                    issues: [
                        { message: 'Invalid User: id must be number, name and email must be strings' },
                    ],
                };
            },
        },
    };
}

// 2. Mock Zod-like schema with safeParse
function createZodUserSchema(): SchemaValidator<User> {
    return {
        safeParse: (data: unknown) => {
            if (
                typeof data === 'object' &&
                data !== null &&
                'id' in data &&
                typeof (data as any).id === 'number' &&
                'name' in data &&
                typeof (data as any).name === 'string'
            ) {
                return { success: true, data: data as User };
            }
            return {
                success: false,
                error: {
                    issues: [{ message: 'Field validation error in User schema' }],
                },
            };
        },
    };
}

test('validateWithSchema with Standard Schema (~standard)', () => {
    const schema = createStandardUserSchema();
    const validUser = { id: 1, name: 'Alice', email: 'alice@test.com' };
    const invalidUser = { id: 'not-a-number', name: 'Alice' };

    const validated = validateWithSchema(schema, validUser);
    assert.deepStrictEqual(validated, validUser);

    assert.throws(
        () => validateWithSchema(schema, invalidUser),
        (err: any) => {
            assert.ok(err instanceof BFastValidationError);
            assert.ok(err.message.includes('Schema validation failed'));
            assert.strictEqual(err.issues.length, 1);
            return true;
        }
    );
});

test('validateWithSchema with Zod-like safeParse schema', () => {
    const schema = createZodUserSchema();
    const valid = { id: 42, name: 'Bob', email: 'bob@test.com' };
    const invalid = { id: 42 };

    const validated = validateWithSchema(schema, valid);
    assert.deepStrictEqual(validated, valid);

    assert.throws(
        () => validateWithSchema(schema, invalid),
        (err: any) => {
            assert.ok(err instanceof BFastValidationError);
            assert.ok(err.message.includes('Field validation error'));
            return true;
        }
    );
});

test('validateWithSchema with custom validator function', () => {
    const fnSchema: SchemaValidator<{ positive: number }> = (val: any) => {
        if (typeof val?.positive !== 'number' || val.positive <= 0) {
            throw new Error('Must be positive number');
        }
        return val;
    };

    assert.deepStrictEqual(validateWithSchema(fnSchema, { positive: 10 }), { positive: 10 });
    assert.throws(() => validateWithSchema(fnSchema, { positive: -5 }));
});

test('BFastDecoder.decode with schema validation', () => {
    const schema = createZodUserSchema();
    const validUser: User = { id: 100, name: 'Charlie', email: 'charlie@test.com' };
    const encoded = BFastEncoder.encode(validUser, { compress: false });

    // Success case
    const decoded = BFastDecoder.decode<User>(encoded, { schema });
    assert.deepStrictEqual(decoded, validUser);

    // Failure case
    const badData = { id: 'invalid-id', name: 'Charlie' };
    const badEncoded = BFastEncoder.encode(badData, { compress: false });
    assert.throws(
        () => BFastDecoder.decode<User>(badEncoded, { schema }),
        (err: any) => {
            assert.ok(err instanceof BFastValidationError);
            return true;
        }
    );
});

test('BFastStreamDecoder with schema validation for each frame', () => {
    const schema = createZodUserSchema();
    const decoder = new BFastStreamDecoder<User>({
        schema,
        expectHandshake: false,
    });

    const validFrame = BFastStreamEncoder.encodeFrame({ id: 1, name: 'Alice', email: 'alice@test.com' });
    const frames = decoder.feed<User>(validFrame);
    assert.strictEqual(frames.length, 1);
    assert.strictEqual(frames[0].name, 'Alice');

    // Invalid frame throws BFastValidationError
    const invalidFrame = BFastStreamEncoder.encodeFrame({ id: 'wrong', name: 123 });
    assert.throws(
        () => decoder.feed<User>(invalidFrame),
        (err: any) => {
            assert.ok(err instanceof BFastValidationError);
            return true;
        }
    );
});

test('bfastFetch with schema validation', async () => {
    const user: User = { id: 7, name: 'Grace', email: 'grace@example.com' };
    const payload = BFastEncoder.encode(user);

    // Mock fetch
    const originalFetch = globalThis.fetch;
    try {
        (globalThis as any).fetch = async (_input: RequestInfo | URL, _init?: RequestInit) => {
            return {
                ok: true,
                status: 200,
                statusText: 'OK',
                headers: new Headers({ 'Content-Type': 'application/x-bfast' }),
                arrayBuffer: async () => payload.buffer,
            } as any;
        };

        const schema = createStandardUserSchema();
        const result = await bfastFetch<User>('https://example.com/api/user', {
            schema,
        });
        assert.deepStrictEqual(result, user);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('bfastQueryOptions creates TanStack Query options and executes queryFn', async () => {
    const user: User = { id: 99, name: 'TanStack User', email: 'tanstack@example.com' };
    const payload = BFastEncoder.encode(user);

    const originalFetch = globalThis.fetch;
    try {
        let capturedSignal: AbortSignal | null | undefined;
        let capturedUrl: string | undefined;

        (globalThis as any).fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
            capturedUrl = String(input);
            capturedSignal = init?.signal;
            return {
                ok: true,
                status: 200,
                statusText: 'OK',
                headers: new Headers({ 'Content-Type': 'application/x-bfast' }),
                arrayBuffer: async () => payload.buffer,
            } as any;
        };

        const schema = createStandardUserSchema();
        const options = bfastQueryOptions<User>({
            queryKey: ['users', 99],
            url: 'https://api.example.com/users/99',
            schema,
            staleTime: 5000,
        });

        assert.deepStrictEqual(options.queryKey, ['users', 99]);
        assert.strictEqual(options.staleTime, 5000);
        assert.strictEqual(typeof options.queryFn, 'function');

        // Test running queryFn
        const controller = new AbortController();
        const queryResult = await options.queryFn({ signal: controller.signal });
        assert.deepStrictEqual(queryResult, user);
        assert.strictEqual(capturedUrl, 'https://api.example.com/users/99');
        assert.strictEqual(capturedSignal, controller.signal);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('bfastInfiniteQueryOptions creates TanStack Infinite Query options', async () => {
    const pageData = [{ id: 1, name: 'Item 1' }, { id: 2, name: 'Item 2' }];
    const payload = BFastEncoder.encode(pageData);

    const originalFetch = globalThis.fetch;
    try {
        let requestedUrl: string | undefined;

        (globalThis as any).fetch = async (input: RequestInfo | URL) => {
            requestedUrl = String(input);
            return {
                ok: true,
                status: 200,
                statusText: 'OK',
                headers: new Headers({ 'Content-Type': 'application/x-bfast' }),
                arrayBuffer: async () => payload.buffer,
            } as any;
        };

        const options = bfastInfiniteQueryOptions({
            queryKey: ['infinite-items'],
            initialPageParam: 1,
            getUrl: (page: number) => `https://api.example.com/items?page=${page}`,
            getNextPageParam: (lastPage: any[], _allPages: any[][], lastParam: number) => {
                return lastPage.length > 0 ? lastParam + 1 : undefined;
            },
        });

        assert.deepStrictEqual(options.queryKey, ['infinite-items']);
        assert.strictEqual(options.initialPageParam, 1);
        assert.strictEqual(options.getNextPageParam(pageData, [pageData], 1), 2);

        // Execute queryFn
        const result = await options.queryFn({ pageParam: 2 });
        assert.deepStrictEqual(result, pageData);
        assert.strictEqual(requestedUrl, 'https://api.example.com/items?page=2');
    } finally {
        globalThis.fetch = originalFetch;
    }
});
