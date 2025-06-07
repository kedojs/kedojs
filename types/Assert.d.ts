/**
 * TypeScript definitions for @kedo/assert module
 */

declare module "@kedo/assert" {
    /**
     * Assertion error class with detailed error information
     */
    export class AssertionError extends Error {
        readonly name: "AssertionError";
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
        });

        toString(): string;
    }

    /**
     * Type for error constructor or validator
     */
    type ErrorConstructor = new (...args: any[]) => Error;
    type ErrorValidator = (err: Error) => boolean;
    type ErrorMatcher =
        | ErrorConstructor
        | ErrorValidator
        | RegExp
        | string
        | Error;

    /**
     * Type for async functions that may throw
     */
    type AsyncFunction = () => Promise<any>;
    type SyncFunction = () => any;

    /**
     * Asserts deep strict equality between actual and expected values
     */
    export function deepStrictEqual<T>(
        actual: T,
        expected: T,
        message?: string,
    ): asserts actual is T;

    /**
     * Asserts that values are not deeply strict equal
     */
    export function notDeepStrictEqual<T>(
        actual: T,
        expected: T,
        message?: string,
    ): void;

    /**
     * Asserts loose equality (==) between actual and expected
     */
    export function equal<T>(
        actual: unknown,
        expected: T,
        message?: string,
    ): asserts actual is T;

    /**
     * Asserts loose inequality (!=) between actual and expected
     */
    export function notEqual(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;

    /**
     * Asserts strict equality (===) between actual and expected
     */
    export function strictEqual<T>(
        actual: unknown,
        expected: T,
        message?: string,
    ): asserts actual is T;

    /**
     * Asserts strict inequality (!==) between actual and expected
     */
    export function notStrictEqual(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;

    /**
     * Asserts that a function does not throw an error
     */
    export function doesNotThrow(
        fn: SyncFunction,
        error?: ErrorConstructor,
        message?: string,
    ): void;
    export function doesNotThrow(fn: SyncFunction, message?: string): void;

    /**
     * Asserts that a function throws an error
     */
    export function throws(
        fn: SyncFunction,
        error?: ErrorMatcher,
        message?: string,
    ): void;
    export function throws(fn: SyncFunction, message?: string): void;

    /**
     * Asserts that a string matches a regular expression
     */
    export function match(
        actual: string,
        regexp: RegExp,
        message?: string,
    ): void;

    /**
     * Asserts that a string does not match a regular expression
     */
    export function doesNotMatch(
        actual: string,
        regexp: RegExp,
        message?: string,
    ): void;

    /**
     * Asserts that a promise rejects
     */
    export function rejects(
        promise: Promise<any> | AsyncFunction,
        error?: ErrorMatcher,
        message?: string,
    ): Promise<void>;
    export function rejects(
        promise: Promise<any> | AsyncFunction,
        message?: string,
    ): Promise<void>;

    /**
     * Asserts that a value is truthy
     */
    export function ok(value: unknown, message?: string): asserts value;

    /**
     * Type guard for checking if a value is defined (not null or undefined)
     */
    export function isDefined<T>(
        value: T | null | undefined,
        message?: string,
    ): asserts value is T;

    /**
     * Type guard for checking if a value is of a specific type
     */
    export function isType<T>(
        value: unknown,
        type: string,
        message?: string,
    ): asserts value is T;

    /**
     * Type guard for checking if a value is an instance of a specific class
     */
    export function isInstanceOf<T>(
        value: unknown,
        constructor: new (...args: any[]) => T,
        message?: string,
    ): asserts value is T;

    /**
     * Asserts that a value is an array
     */
    export function isArray<T>(
        value: unknown,
        message?: string,
    ): asserts value is T[];

    /**
     * Default export with all assertion functions
     */
    const assert: {
        readonly AssertionError: typeof AssertionError;
        readonly deepStrictEqual: typeof deepStrictEqual;
        readonly notDeepStrictEqual: typeof notDeepStrictEqual;
        readonly strictEqual: typeof strictEqual;
        readonly notStrictEqual: typeof notStrictEqual;
        readonly equal: typeof equal;
        readonly notEqual: typeof notEqual;
        readonly throws: typeof throws;
        readonly doesNotThrow: typeof doesNotThrow;
        readonly doesNotMatch: typeof doesNotMatch;
        readonly rejects: typeof rejects;
        readonly match: typeof match;
        readonly ok: typeof ok;
        readonly isDefined: typeof isDefined;
        readonly isType: typeof isType;
        readonly isInstanceOf: typeof isInstanceOf;
        readonly isArray: typeof isArray;
    };

    export default assert;
}
