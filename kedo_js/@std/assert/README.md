# @kedo/assert

A comprehensive assertion library for KedoJS with full TypeScript support, providing type-safe assertions and type guards.

## Features

- **Type-safe assertions**: All assertion functions provide proper TypeScript type guards
- **Comprehensive error information**: Detailed error messages with actual vs expected values
- **Deep equality checking**: Support for objects, arrays, Maps, Sets, Dates, and RegExp
- **Async support**: Promise rejection assertions
- **Custom type guards**: Additional utilities for type checking
- **Full TypeScript support**: Complete type definitions and IntelliSense support

## Installation

```javascript
import assert from "@kedo/assert";
// or
import { strictEqual, deepStrictEqual, ok } from "@kedo/assert";
```

## Basic Usage

### Equality Assertions

```typescript
import assert from "@kedo/assert";

// Strict equality (===)
assert.strictEqual(42, 42);
assert.strictEqual("hello", "hello");

// Loose equality (==)
assert.equal("42", 42); // passes
assert.equal(null, undefined); // passes

// Deep equality for objects
assert.deepStrictEqual({ a: 1, b: 2 }, { a: 1, b: 2 });
assert.deepStrictEqual([1, 2, 3], [1, 2, 3]);

// Negation assertions
assert.notStrictEqual(1, "1");
assert.notDeepStrictEqual({ a: 1 }, { a: 2 });
```

### Type Guards and Type Safety

The assert library provides TypeScript type guards that help narrow types:

```typescript
function processValue(value: unknown): void {
    assert.strictEqual(value, "hello");
    // TypeScript now knows value is string
    console.log(value.toUpperCase()); // No type error
}

function handleUser(data: unknown): void {
    assert.isType<string>(data, "string");
    // TypeScript now knows data is string
    console.log(data.length);
}

function processArray(items: unknown): void {
    assert.isArray<number>(items);
    // TypeScript now knows items is number[]
    const sum = items.reduce((a, b) => a + b, 0);
}
```

### Truthiness and Existence Checks

```typescript
// Truthiness assertion
assert.ok(true);
assert.ok("non-empty string");
assert.ok(42);
assert.ok([]); // Empty array is truthy

// Check if value is defined (not null or undefined)
function processData(data: string | null | undefined): void {
    assert.isDefined(data);
    // TypeScript now knows data is string
    console.log(data.length);
}
```

### Instance and Type Checking

```typescript
class User {
    constructor(public name: string) {}
}

function handleUser(value: unknown): void {
    assert.isInstanceOf(value, User);
    // TypeScript now knows value is User
    console.log(value.name);
}

// Type checking
function processString(value: unknown): void {
    assert.isType<string>(value, "string");
    // TypeScript now knows value is string
    console.log(value.charAt(0));
}
```

### Error Handling

```typescript
// Assert that a function throws an error
assert.throws(() => {
    throw new TypeError("Invalid type");
}, TypeError);

// With error message pattern
assert.throws(() => {
    throw new Error("File not found");
}, /not found/);

// With custom error validator
assert.throws(() => {
    throw new Error("Custom error");
}, (err: Error) => err.message.includes("Custom"));

// Assert that a function doesn't throw
assert.doesNotThrow(() => {
    const result = JSON.parse('{"valid": "json"}');
});
```

### Async/Promise Handling

```typescript
// Assert that a promise rejects
await assert.rejects(async () => {
    throw new Error("Async error");
});

// With specific error type
await assert.rejects(
    Promise.reject(new TypeError("Type error")),
    TypeError
);

// With error pattern
await assert.rejects(
    fetch("invalid-url"),
    /network/i
);
```

### String Pattern Matching

```typescript
// Assert string matches pattern
assert.match("hello123", /\d+/);
assert.match("user@example.com", /^[\w.]+@[\w.]+$/);

// Assert string doesn't match pattern
assert.doesNotMatch("hello", /\d+/);
assert.doesNotMatch("plaintext", /[<>]/);
```

### Complex Object Assertions

```typescript
interface ApiResponse {
    data: User[];
    meta: {
        total: number;
        page: number;
    };
}

const response1: ApiResponse = {
    data: [{ name: "Alice" }, { name: "Bob" }],
    meta: { total: 2, page: 1 }
};

const response2: ApiResponse = {
    data: [{ name: "Alice" }, { name: "Bob" }],
    meta: { total: 2, page: 1 }
};

assert.deepStrictEqual(response1, response2);
```

### Working with Collections

```typescript
// Maps
const map1 = new Map([["key1", "value1"], ["key2", "value2"]]);
const map2 = new Map([["key1", "value1"], ["key2", "value2"]]);
assert.deepStrictEqual(map1, map2);

// Sets
const set1 = new Set([1, 2, 3]);
const set2 = new Set([3, 2, 1]); // Order doesn't matter for Sets
assert.deepStrictEqual(set1, set2);

// Dates
const date1 = new Date("2023-01-01T00:00:00Z");
const date2 = new Date("2023-01-01T00:00:00Z");
assert.deepStrictEqual(date1, date2);
```

## Error Information

When assertions fail, you get detailed error information:

```typescript
try {
    assert.strictEqual(42, "42");
} catch (error) {
    console.log(error instanceof assert.AssertionError); // true
    console.log(error.actual); // 42
    console.log(error.expected); // "42"
    console.log(error.operator); // "strictEqual"
}
```

## API Reference

### Core Assertions

- `strictEqual<T>(actual: unknown, expected: T, message?: string): asserts actual is T`
- `notStrictEqual(actual: unknown, expected: unknown, message?: string): void`
- `equal<T>(actual: unknown, expected: T, message?: string): asserts actual is T`
- `notEqual(actual: unknown, expected: unknown, message?: string): void`
- `deepStrictEqual<T>(actual: T, expected: T, message?: string): asserts actual is T`
- `notDeepStrictEqual<T>(actual: T, expected: T, message?: string): void`

### Truthiness and Existence

- `ok(value: unknown, message?: string): asserts value`
- `isDefined<T>(value: T | null | undefined, message?: string): asserts value is T`

### Type Guards

- `isType<T>(value: unknown, type: string, message?: string): asserts value is T`
- `isInstanceOf<T>(value: unknown, constructor: new (...args: any[]) => T, message?: string): asserts value is T`
- `isArray<T>(value: unknown, message?: string): asserts value is T[]`

### Error Handling

- `throws(fn: () => any, error?: ErrorMatcher, message?: string): void`
- `doesNotThrow(fn: () => any, error?: ErrorConstructor, message?: string): void`
- `rejects(promise: Promise<any> | (() => Promise<any>), error?: ErrorMatcher, message?: string): Promise<void>`

### Pattern Matching

- `match(actual: string, regexp: RegExp, message?: string): void`
- `doesNotMatch(actual: string, regexp: RegExp, message?: string): void`

## TypeScript Integration

The library provides full TypeScript support with:

- Type assertions that narrow types
- Generic type parameters for better inference
- Proper error types
- IntelliSense support
- Compile-time type checking

## Best Practices

1. **Use specific assertions**: Prefer `strictEqual` over `equal` for better type safety
2. **Leverage type guards**: Use `isDefined`, `isType`, etc. to narrow types
3. **Provide meaningful messages**: Add custom error messages for better debugging
4. **Use async assertions**: Prefer `rejects` for testing promise rejections
5. **Test error conditions**: Use `throws` to ensure proper error handling