/**
 * Assertion error class with detailed error information
 */
export class AssertionError extends Error {
    readonly name = "AssertionError";
    readonly actual: unknown;
    readonly expected: unknown;
    readonly operator: string;
    readonly path: string;

    constructor(options: {
        message?: string;
        actual?: unknown;
        expected?: unknown;
        operator?: string;
        path?: string;
    }) {
        super(options.message);
        this.actual = options.actual;
        this.expected = options.expected;
        this.operator = options.operator || "";
        this.path = options.path || "";
        this.stack = new Error().stack;
    }

    toString(): string {
        return (
            `${this.name}: ${this.message}\n` +
            `  Path: ${this.path}\n` +
            `  Actual: ${this.actual}\n` +
            `  Expected: ${this.expected}\n` +
            `  Operator: ${this.operator}\n` +
            `${this.stack}`
        );
    }
}

/**
 * Type for error constructor or validator
 */
type ErrorConstructor = new (...args: any[]) => Error;
type ErrorValidator = (err: Error) => boolean;
type ErrorMatcher = ErrorConstructor | ErrorValidator | RegExp | string | Error;

/**
 * Type for async functions that may throw
 */
type AsyncFunction = () => Promise<any>;
type SyncFunction = () => any;

/**
 * Helper function for deep equality comparison
 */
function _deepEqualHelper(
    actual: unknown,
    expected: unknown,
    path = "",
    seen = new Set(),
): {
    ok: boolean;
    path?: { path: string; actual: unknown; expected: unknown };
} {
    if (Object.is(actual, expected)) return { ok: true };

    if (
        typeof actual !== "object" ||
        actual === null ||
        typeof expected !== "object" ||
        expected === null
    ) {
        return { ok: false, path: { path, actual, expected } };
    }

    if (Object.getPrototypeOf(actual) !== Object.getPrototypeOf(expected)) {
        return { ok: false, path: { path, actual, expected } };
    }

    if (seen.has(actual) || seen.has(expected)) {
        return { ok: true };
    }

    seen.add(actual);
    seen.add(expected);

    if (actual instanceof Date) {
        if (actual.getTime() !== (expected as Date).getTime()) {
            return { ok: false, path: { path, actual, expected } };
        }
    } else if (actual instanceof RegExp) {
        if (actual.toString() !== (expected as RegExp).toString()) {
            return { ok: false, path: { path, actual, expected } };
        }
    } else if (actual instanceof Set) {
        if (actual.size !== (expected as Set<unknown>).size) {
            return { ok: false, path: { path, actual, expected } };
        }
        for (let item of actual) {
            if (!(expected as Set<unknown>).has(item)) {
                return { ok: false, path: { path, actual, expected } };
            }
        }
    } else if (actual instanceof Map) {
        if (actual.size !== (expected as Map<unknown, unknown>).size) {
            return { ok: false, path: { path, actual, expected } };
        }
        for (let [key, value] of actual) {
            if (
                !(expected as Map<unknown, unknown>).has(key) ||
                !_deepEqualHelper(
                    value,
                    (expected as Map<unknown, unknown>).get(key),
                    `${path}.get(${key})`,
                    seen,
                ).ok
            ) {
                return { ok: false, path: { path, actual, expected } };
            }
        }
    } else {
        const keysA = Reflect.ownKeys(actual);
        const keysB = Reflect.ownKeys(expected);
        if (keysA.length !== keysB.length) {
            return { ok: false, path: { path, actual, expected } };
        }
        for (const key of keysA) {
            if (!keysB.includes(key)) {
                return {
                    ok: false,
                    path: {
                        path: `${path}.${String(key)}`,
                        actual: (actual as any)[key],
                        expected: (expected as any)[key],
                    },
                };
            }
            const result = _deepEqualHelper(
                (actual as any)[key],
                (expected as any)[key],
                `${path}.${String(key)}`,
                seen,
            );
            if (!result.ok) {
                return result;
            }
        }
    }

    return { ok: true };
}

/**
 * Asserts deep strict equality between actual and expected values
 */
export function deepStrictEqual<T>(
    actual: T,
    expected: T,
    message?: string,
): asserts actual is T {
    const result = _deepEqualHelper(actual, expected);
    if (!result.ok) {
        throw new AssertionError({
            message:
                message ||
                "Actual and expected values are not deeply strict equal",
            actual,
            expected,
            operator: "deepStrictEqual",
            path: "",
        });
    }
}

/**
 * Asserts that values are not deeply strict equal
 */
export function notDeepStrictEqual<T>(
    actual: T,
    expected: T,
    message?: string,
): void {
    const result = _deepEqualHelper(actual, expected);
    if (result.ok) {
        throw new AssertionError({
            message:
                message || "Actual and expected values are deeply strict equal",
            actual,
            expected,
            operator: "notDeepStrictEqual",
            path: "",
        });
    }
}

/**
 * Asserts loose equality (==) between actual and expected
 */
export function equal<T>(
    actual: unknown,
    expected: T,
    message?: string,
): asserts actual is T {
    if (actual != expected) {
        throw new AssertionError({
            message: message || "Actual and expected values are not equal",
            actual,
            expected,
            operator: "equal",
            path: "",
        });
    }
}

/**
 * Asserts loose inequality (!=) between actual and expected
 */
export function notEqual(
    actual: unknown,
    expected: unknown,
    message?: string,
): void {
    if (actual == expected) {
        throw new AssertionError({
            message: message || "Actual and expected values are equal",
            actual,
            expected,
            operator: "notEqual",
            path: "",
        });
    }
}

/**
 * Asserts strict equality (===) between actual and expected
 */
export function strictEqual<T>(
    actual: unknown,
    expected: T,
    message?: string,
): asserts actual is T {
    if (actual !== expected) {
        throw new AssertionError({
            message:
                message || "Actual and expected values are not strictly equal",
            actual,
            expected,
            operator: "strictEqual",
            path: "",
        });
    }
}

/**
 * Asserts strict inequality (!==) between actual and expected
 */
export function notStrictEqual(
    actual: unknown,
    expected: unknown,
    message?: string,
): void {
    if (actual === expected) {
        throw new AssertionError({
            message: message || "Actual and expected values are strictly equal",
            actual,
            expected,
            operator: "notStrictEqual",
            path: "",
        });
    }
}

/**
 * Asserts that a function does not throw an error
 */
export function doesNotThrow(
    fn: SyncFunction,
    error?: ErrorConstructor | string,
    message?: string,
): void {
    try {
        fn();
    } catch (e) {
        if (typeof error === "string") {
            message = error;
            error = undefined;
        }

        if (error === undefined) {
            throw new AssertionError({
                message: message || "Function threw an exception",
                actual: e,
                expected: undefined,
                operator: "doesNotThrow",
                path: "",
            });
        }

        if (e instanceof error) {
            throw new AssertionError({
                message:
                    message || "Function threw an exception of the wrong type",
                actual: e,
                expected: error,
                operator: "doesNotThrow",
                path: "",
            });
        }
    }
}

/**
 * Asserts that a function throws an error
 */
export function throws(
    fn: SyncFunction,
    error?: ErrorMatcher | string,
    message?: string,
): void {
    try {
        fn();
    } catch (e) {
        if (error === undefined) {
            return;
        }

        if (typeof error === "function" && error.prototype) {
            if (e instanceof error) {
                return;
            }
        } else if (typeof error === "function") {
            if ((error as ErrorValidator)(e as Error)) {
                return;
            }
        }

        if (
            (typeof error === "string" && (e as Error).message === error) ||
            (e as Error).name === error
        ) {
            return;
        }

        if (error instanceof RegExp && error.test((e as Error).message)) {
            return;
        }

        if (typeof error === "object" && error !== null) {
            let result = _deepEqualHelper(e, error);
            if (result.ok) {
                return;
            }
        }

        throw new AssertionError({
            message:
                message ||
                "Function did not throw an exception of the expected type",
            actual: e,
            expected: error,
            operator: "throws",
            path: "",
        });
    }

    throw new AssertionError({
        message: message || "Function did not throw an exception",
        actual: undefined,
        expected: error,
        operator: "throws",
        path: "",
    });
}

/**
 * Asserts that a string matches a regular expression
 */
export function match(actual: string, regexp: RegExp, message?: string): void {
    if (!regexp.test(actual)) {
        throw new AssertionError({
            message:
                message || "Actual value does not match the regular expression",
            actual,
            expected: regexp,
            operator: "match",
            path: "",
        });
    }
}

/**
 * Asserts that a string does not match a regular expression
 */
export function doesNotMatch(
    actual: string,
    regexp: RegExp,
    message?: string,
): void {
    if (regexp.test(actual)) {
        throw new AssertionError({
            message: message || "Actual value matches the regular expression",
            actual,
            expected: regexp,
            operator: "doesNotMatch",
            path: "",
        });
    }
}

/**
 * Asserts that a promise rejects
 */
export async function rejects(
    promise: Promise<any> | AsyncFunction,
    error?: ErrorMatcher | string,
    message?: string,
): Promise<void> {
    try {
        if (typeof promise === "function") {
            await promise();
        } else {
            await promise;
        }
    } catch (e) {
        if (error === undefined) {
            return;
        }

        if (typeof error === "function" && error.prototype) {
            if (e instanceof error) {
                return;
            }
        } else if (typeof error === "function") {
            if ((error as ErrorValidator)(e as Error)) {
                return;
            }
        }

        if (
            (typeof error === "string" && (e as Error).message === error) ||
            (e as Error).name === error
        ) {
            return;
        }

        if (error instanceof RegExp && error.test((e as Error).message)) {
            return;
        }

        if (typeof error === "object" && error !== null) {
            let result = _deepEqualHelper(e, error);
            if (result.ok) {
                return;
            }
        }

        throw new AssertionError({
            message:
                message ||
                "Promise did not reject with an exception of the expected type",
            actual: e,
            expected: error,
            operator: "rejects",
            path: "",
        });
    }

    throw new AssertionError({
        message: message || "Promise did not reject",
        actual: undefined,
        expected: error,
        operator: "rejects",
        path: "",
    });
}

/**
 * Asserts that a value is truthy
 */
export function ok(value: unknown, message?: string): asserts value {
    if (!value) {
        throw new AssertionError({
            message: message || "Value is not truthy",
            actual: value,
            expected: true,
            operator: "ok",
            path: "",
        });
    }
}

/**
 * Type guard for checking if a value is defined (not null or undefined)
 */
export function isDefined<T>(
    value: T | null | undefined,
    message?: string,
): asserts value is T {
    if (value === null || value === undefined) {
        throw new AssertionError({
            message: message || "Value is null or undefined",
            actual: value,
            expected: "defined value",
            operator: "isDefined",
            path: "",
        });
    }
}

/**
 * Type guard for checking if a value is of a specific type
 */
export function isType<T>(
    value: unknown,
    type: string,
    message?: string,
): asserts value is T {
    if (typeof value !== type) {
        throw new AssertionError({
            message: message || `Value is not of type ${type}`,
            actual: typeof value,
            expected: type,
            operator: "isType",
            path: "",
        });
    }
}

/**
 * Type guard for checking if a value is an instance of a specific class
 */
export function isInstanceOf<T>(
    value: unknown,
    constructor: new (...args: any[]) => T,
    message?: string,
): asserts value is T {
    if (!(value instanceof constructor)) {
        throw new AssertionError({
            message:
                message || `Value is not an instance of ${constructor.name}`,
            actual: value?.constructor?.name || typeof value,
            expected: constructor.name,
            operator: "isInstanceOf",
            path: "",
        });
    }
}

/**
 * Asserts that a value is an array
 */
export function isArray<T>(
    value: unknown,
    message?: string,
): asserts value is T[] {
    if (!Array.isArray(value)) {
        throw new AssertionError({
            message: message || "Value is not an array",
            actual: typeof value,
            expected: "array",
            operator: "isArray",
            path: "",
        });
    }
}

/**
 * Default export with all assertion functions
 */
export default {
    AssertionError,
    deepStrictEqual,
    notDeepStrictEqual,
    strictEqual,
    notStrictEqual,
    equal,
    notEqual,
    throws,
    doesNotThrow,
    doesNotMatch,
    rejects,
    match,
    ok,
    isDefined,
    isType,
    isInstanceOf,
    isArray,
} as const;
