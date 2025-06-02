import assert from "@kedo/assert";

assert.deepStrictEqual({ a: 1 }, { a: 1 });
assert.throws(() => assert.deepStrictEqual({ a: 1 }, { a: 2 }));

// Test for simple objects with same key-value pairs
assert.deepStrictEqual({ a: 1, b: 2 }, { a: 1, b: 2 });

// Test for nested objects with same structure and values
assert.deepStrictEqual({ a: { b: 2 } }, { a: { b: 2 } });

// Test for arrays with same elements
assert.deepStrictEqual([1, 2, 3], [1, 2, 3]);

// Test for arrays with nested objects
assert.deepStrictEqual([{ a: 1 }, { b: 2 }], [{ a: 1 }, { b: 2 }]);

// Test for different objects (should throw)
assert.throws(() => assert.deepStrictEqual({ a: 1, b: 2 }, { a: 1, b: 3 }));

// Test for different arrays (should throw)
assert.throws(() => assert.deepStrictEqual([1, 2, 3], [1, 2, 4]));

// Test for objects with different key order (should pass)
assert.deepStrictEqual({ a: 1, b: 2 }, { b: 2, a: 1 });

// Test for objects with additional keys (should throw)
assert.throws(() => assert.deepStrictEqual({ a: 1 }, { a: 1, b: 2 }));

// Test for objects with different types (should throw)
assert.throws(() => assert.deepStrictEqual({ a: 1 }, { a: "1" }));

// Test for same values with different types (should throw)
assert.throws(() => assert.deepStrictEqual(1, "1"));

// Test for null values (should pass)
assert.deepStrictEqual(null, null);

// Test for undefined values (should pass)
assert.deepStrictEqual(undefined, undefined);

// Test for null and undefined (should throw)
assert.throws(() => assert.deepStrictEqual(null, undefined));

// Test for same values (should pass)
assert.deepStrictEqual(123, 123);
assert.deepStrictEqual("abc", "abc");

// Test for different values (should throw)
assert.throws(() => assert.deepStrictEqual(123, 456));
assert.throws(() => assert.deepStrictEqual("abc", "def"));

// Tests for deepStrictEqual
assert.deepStrictEqual({ a: 1 }, { a: 1 });
assert.deepStrictEqual([1, 2, 3], [1, 2, 3]);
assert.deepStrictEqual({ a: { b: 2 } }, { a: { b: 2 } });
assert.deepStrictEqual(new Set([1, 2]), new Set([1, 2]));
assert.deepStrictEqual(new Date("2020-01-01"), new Date("2020-01-01"));
assert.deepStrictEqual(
    new Map([
        [1, "a"],
        [2, "b"],
    ]),
    new Map([
        [1, "a"],
        [2, "b"],
    ]),
);

// Tests for notDeepStrictEqual
assert.notDeepStrictEqual({ a: 1 }, { a: 2 });
assert.notDeepStrictEqual([1, 2, 3], [1, 2, 4]);
assert.notDeepStrictEqual({ a: { b: 2 } }, { a: { b: 3 } });
assert.notDeepStrictEqual(new Date("2020-01-01"), new Date("2020-01-02"));
assert.notDeepStrictEqual(new Set([1, 2]), new Set([1, 3]));
assert.notDeepStrictEqual(
    new Map([
        [1, "a"],
        [2, "b"],
    ]),
    new Map([
        [1, "a"],
        [2, "c"],
    ]),
);

// Tests for equal
assert.equal(1, 1);
assert.equal("abc", "abc");
assert.equal(true, true);
assert.equal(null, null);
assert.equal(undefined, undefined);
assert.equal(0, "0"); // Non-strict comparison

// Tests for notEqual
assert.notEqual(1, 2);
assert.notEqual("abc", "def");
assert.notEqual(true, false);
assert.notEqual(0, 1);
assert.throws(() => assert.notEqual(null, undefined)); // Strict comparison

// Tests for strictEqual
assert.strictEqual(1, 1);
assert.strictEqual("abc", "abc");
assert.strictEqual(true, true);
assert.strictEqual(null, null);
assert.strictEqual(undefined, undefined);

// Tests for notStrictEqual
assert.notStrictEqual(1, "1"); // Strict comparison
assert.notStrictEqual("abc", "def");
assert.notStrictEqual(true, 1);
assert.notStrictEqual(null, undefined);
assert.notStrictEqual(0, false);

// Tests for throws
assert.throws(() => {
    throw new Error("error");
});
assert.throws(() => {
    throw new TypeError("error");
}, TypeError);
assert.throws(() => {
    throw new Error("expected error");
}, /expected/);
assert.throws(() => {
    throw new Error("expected error");
}, "expected error");
assert.throws(
    () => {
        throw new Error("Custom error");
    },
    (err) => err.message.includes("Custom"),
);

// Rejects
await assert.rejects(async () => {
    throw new Error("Async error");
});
await assert.rejects(Promise.reject(new TypeError("Type error")), TypeError);
await assert.rejects(async () => {
    throw new Error("Pattern test");
}, /Pattern/);

// Tests for doesNotThrow
assert.doesNotThrow(() => /* no error thrown */ {});
assert.doesNotThrow(() => {
    throw new Error("expected error");
}, TypeError);
assert.doesNotThrow(() => {
    throw new TypeError("Wrong type");
}, SyntaxError);

// Tests for match
assert.match("abc123", /\d{3}/);
assert.match("hello world", /world/);
assert.throws(() => assert.match("abc", /\d/));

// Tests for doesNotMatch
assert.doesNotMatch("abc123", /\d{4}/);
assert.doesNotMatch("hello world", /planet/);

// Tests for ok
assert.ok(true);
assert.ok(1);
assert.ok("non-empty string");
assert.ok([]); // Empty array is truthy
assert.ok({}); // Empty object is truthy

// Utils
assert.isInstanceOf(
    new assert.AssertionError({ message: "Error" }),
    assert.AssertionError,
);
assert.isInstanceOf(new TypeError("Error"), TypeError);
assert.isDefined({});
assert.isType({}, "object");
assert.isArray([]);
assert.throws(() => {
    assert.isInstanceOf(new TypeError("error"), SyntaxError);
}, assert.AssertionError);

// nested object
const data1 = {
    users: [
        { id: 1, name: "Alice" },
        { id: 2, name: "Bob", email: "bob@test.com" },
    ],
    metadata: {
        total: 2,
        page: 1,
    },
    settings: {
        theme: "dark",
        notifications: true,
    },
};

const data2 = {
    users: [
        { id: 1, name: "Alice" },
        { id: 2, name: "Bob", email: "bob@test.com" },
    ],
    metadata: {
        total: 2,
        page: 1,
    },
    settings: {
        theme: "dark",
        notifications: true,
    },
};

assert.deepStrictEqual(data1, data2);
