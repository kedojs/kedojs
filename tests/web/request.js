import { ReadableStream } from "@kedo/stream";
import assert from "@kedo/assert";

async function testRequestWithURLSearchParamsAsBody() {
  // Test 12: Passing URLSearchParams as the body
  const searchParams = new URLSearchParams({ foo: "bar", baz: "qux" });
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: searchParams,
  });

  return await request.text().then((bodyText) => {
    assert.strictEqual(
      bodyText,
      searchParams.toString(),
      "Body should be URL encoded string from URLSearchParams",
    );
    assert.strictEqual(
      request.headers.get("Content-Type"),
      "application/x-www-form-urlencoded;charset=UTF-8",
      "Content-Type should be set to application/x-www-form-urlencoded when URLSearchParams is used as body",
    );
  });
}

async function testRequestWithURLSearchParamsAsBodyEdgeCases() {
  // Test 13: Empty URLSearchParams as body
  const searchParams = new URLSearchParams();
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: searchParams,
  });

  return await request.text().then((bodyText) => {
    assert.strictEqual(
      bodyText,
      "",
      "Body should be empty when URLSearchParams is empty",
    );
    assert.strictEqual(
      request.headers.get("Content-Type"),
      "application/x-www-form-urlencoded;charset=UTF-8",
      "Content-Type should be set even for empty URLSearchParams",
    );
  });
}

async function testRequestJsonMethod() {
  // Test 14: Request with JSON body
  const jsonBody = { foo: "bar", baz: 42 };
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: JSON.stringify(jsonBody),
    headers: { "Content-Type": "application/json" },
  });

  assert.strictEqual(request.headers.get("Content-Type"), "application/json");
  assert.strictEqual(
    request.bodyUsed,
    false,
    "The bodyUsed property should be false after calling the json method",
  );
  await request.json().then((parsedJson) => {
    assert.deepStrictEqual(
      parsedJson,
      jsonBody,
      "The JSON method should correctly parse the body",
    );
  });

  assert.strictEqual(
    request.bodyUsed,
    true,
    "The bodyUsed property should be false after calling the json method",
  );
  assert.rejects(
    async () => await request.json(),
    TypeError,
    "The JSON method should throw a TypeError when the body is not valid JSON",
  );
}

async function testRequestBytesMethod() {
  // Test 15: Request with bytes (Uint8Array) body
  const byteArray = new Uint8Array([1, 2, 3, 4, 5]);
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: byteArray,
  });

  return await request.bytes().then((result) => {
    assert.deepStrictEqual(
      result,
      byteArray,
      "The bytes method should return the correct Uint8Array",
    );
  });
}

async function testRequestArrayBufferMethod() {
  // Test 16: Request with ArrayBuffer body
  const buffer = new Uint8Array([1, 2, 3, 4, 5]);
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: buffer,
  });

  return await request.arrayBuffer().then((result) => {
    assert.strictEqual(
      result.byteLength,
      buffer.byteLength,
      "The ArrayBuffer method should return a buffer of correct length",
    );
    assert.deepStrictEqual(
      new Uint8Array(result),
      new Uint8Array(buffer),
      "The ArrayBuffer method should return the correct ArrayBuffer",
    );
  });
}

async function testRequestBodyProperty() {
  // Test 19: body property and its type
  const text = "Hello, world!";
  const request = new Request("https://example.com/path", {
    method: "POST",
    body: text,
  });

  assert.ok(
    request.body instanceof ReadableStream,
    "body should be a ReadableStream",
  );
  // consume the body stream
  for await (const chunk of request.body) {
    assert.strictEqual(
      new TextDecoder().decode(chunk),
      text,
      "The body stream should yield the correct data",
    );
  }
}

async function testRequestArgs() {
  const req = new Request("http://kevin/", {
    body: "my test body",
    method: "POST",
    headers: {
      "test-header": "value",
    },
  });

  assert.strictEqual("my test body", await req.text());
  assert.strictEqual("POST", req.method);
  assert.strictEqual("value", req.headers.get("test-header"));
  assert.strictEqual("http://kevin/", req.url);
}

function testUndefinedMethod() {
  const req = new Request("http://kevin/", {
    headers: {
      "test-header": "value",
    },
  });

  assert.strictEqual("GET", req.method);
}

function testRequestConstructor() {
  // Test 1: Basic properties
  const url = "https://example.com/";
  const method = "GET";
  const request = new Request(url, { method });

  assert.strictEqual(request.url, url, "URL should be set correctly");
  assert.strictEqual(request.method, method, "Method should be GET");
  assert.strictEqual(
    request.headers instanceof Headers,
    true,
    "Headers should be an instance of Headers",
  );
  assert.strictEqual(
    request.credentials,
    "same-origin",
    'Default credentials should be "same-origin"',
  );
}

function testRequestMethodNormalization() {
  // Test 2: Method normalization
  const request = new Request("https://example.com", { method: "get" });
  assert.strictEqual(
    request.method,
    "GET",
    "Method should be normalized to uppercase",
  );
}

async function testRequestBodyHandling() {
  // Test 3: Request with a body
  const body = JSON.stringify({ foo: "bar" });
  const headers = new Headers({ "Content-Type": "application/json" });
  const request = new Request("https://example.com", {
    method: "POST",
    body,
    headers,
  });

  return await request.text().then((text) => {
    console.log("text: ", text, "Body: ", body);
    assert.strictEqual(
      text,
      body,
      "Body should be correctly set and retrievable",
    );
    assert.strictEqual(
      request.headers.get("content-type"),
      "application/json",
      "Content-Type should be correctly set",
    );
  });
}

function testRequestErrorHandling() {
  // Test 5: Error Handling
  assert.throws(
    () => new Request("", { method: "POST" }),
    TypeError,
    "Should throw TypeError for empty URL",
  );
}

// Execute tests
async function runTests() {
  console.log("Running tests...");
  try {
    await testRequestArgs();
    console.log("testRequestArgs passed");

    testUndefinedMethod();
    console.log("testUndefinedMethod passed");

    await testRequestWithURLSearchParamsAsBody();
    console.log("testRequestWithURLSearchParamsAsBody passed");

    await testRequestWithURLSearchParamsAsBodyEdgeCases();
    console.log("testRequestWithURLSearchParamsAsBodyEdgeCases passed");

    testRequestConstructor();
    console.log("testRequestConstructor passed");

    await testRequestArrayBufferMethod();
    console.log("testRequestArrayBufferMethod passed");

    await testRequestBodyProperty();
    console.log("testRequestBodyProperty passed");

    await testRequestJsonMethod();
    console.log("testRequestJsonMethod passed");

    await testRequestBytesMethod();
    console.log("testRequestBytesMethod passed");

    testRequestMethodNormalization();
    console.log("testRequestMethodNormalization passed");

    await testRequestBodyHandling();
    console.log("testRequestBodyHandling passed");

    testRequestErrorHandling();
    console.log("testRequestErrorHandling passed");

    console.log("All tests passed");
  } catch (err) {
    console.error(err.message);
  }
}

await runTests();
