class AssertionError extends Error {
  constructor({ message, actual, expected, operator, path }) {
    super(message);
    this.name = "AssertionError";
    this.actual = actual;
    this.expected = expected;
    this.operator = operator;
    this.path = path;
    this.stack = new Error().stack;
  }

  toString() {
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

function _deepEqualHelper(actual, expected, path = "", seen = new Set()) {
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
    if (actual.getTime() !== expected.getTime()) {
      return { ok: false, path: { path, actual, expected } };
    }
  } else if (actual instanceof RegExp) {
    if (actual.toString() !== expected.toString()) {
      return { ok: false, path: { path, actual, expected } };
    }
  } else if (actual instanceof Set) {
    if (actual.size !== expected.size) {
      return { ok: false, path: { path, actual, expected } };
    }
    for (let item of actual) {
      if (!expected.has(item)) {
        return { ok: false, path: { path, actual, expected } };
      }
    }
  } else if (actual instanceof Map) {
    if (actual.size !== expected.size) {
      return { ok: false, path: { path, actual, expected } };
    }
    for (let [key, value] of actual) {
      if (
        !expected.has(key) ||
        !_deepEqualHelper(value, expected.get(key), `${path}.get(${key})`, seen)
          .ok
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
            actual: actual[key],
            expected: expected[key],
          },
        };
      }
      const result = _deepEqualHelper(
        actual[key],
        expected[key],
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

export function deepStrictEqual(actual, expected, message) {
  const result = _deepEqualHelper(actual, expected);
  if (!result.ok) {
    throw new AssertionError({
      message:
        message || "Actual and expected values are not deeply strict equal",
      actual,
      expected,
      operator: "deepStrictEqual",
      path: "",
    });
  }
}

export function notDeepStrictEqual(actual, expected, message) {
  const result = _deepEqualHelper(actual, expected);
  if (result.ok) {
    throw new AssertionError({
      message: message || "Actual and expected values are deeply strict equal",
      actual,
      expected,
      operator: "notDeepStrictEqual",
      path: "",
    });
  }
}

export function equal(actual, expected, message) {
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

export function notEqual(actual, expected, message) {
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

export function strictEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new AssertionError({
      message: message || "Actual and expected values are not strictly equal",
      actual,
      expected,
      operator: "strictEqual",
      path: "",
    });
  }
}

export function notStrictEqual(actual, expected, message) {
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

export function doesNotThrow(fn, error, message) {
  try {
    fn();
  } catch (e) {
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
        message: message || "Function threw an exception of the wrong type",
        actual: e,
        expected: error,
        operator: "doesNotThrow",
        path: "",
      });
    }
  }
}

export function throws(fn, error, message) {
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
    }

    if (
      (typeof error === "string" && e.message === error) ||
      e.name === error
    ) {
      return;
    }

    if (error instanceof RegExp && error.test(e.message)) {
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
        message || "Function did not throw an exception of the expected type",
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

export function match(actual, regexp, message) {
  if (!regexp.test(actual)) {
    throw new AssertionError({
      message: message || "Actual value does not match the regular expression",
      actual,
      expected,
      operator: "match",
      path: "",
    });
  }
}

export function doesNotMatch(actual, regexp, message) {
  if (regexp.test(actual)) {
    throw new AssertionError({
      message: message || "Actual value matches the regular expression",
      actual,
      expected,
      operator: "doesNotMatch",
      path: "",
    });
  }
}

export async function rejects(promise, error, message) {
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
      if (error(e)) {
        return;
      }
    }

    if (
      (typeof error === "string" && e.message === error) ||
      e.name === error
    ) {
      return;
    }

    if (error instanceof RegExp && error.test(e.message)) {
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
    });
  }

  throw new AssertionError({
    message: message || "Promise did not reject",
    actual: undefined,
    expected: error,
    operator: "rejects",
  });
}

export function ok(value, message) {
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
};
