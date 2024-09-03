import { deepStrictEqual, strictEqual, ok, throws } from "@kedo/assert"; // Replace with the correct path to your assertion library

// Test Suite for the Response Class
async function testResponseConstructor() {
  // Test 1: Basic properties
  const response = new Response("Hello, world!", {
    status: 200,
    statusText: "OK",
    headers: { "Content-Type": "text/plain" },
  });

  strictEqual(response.status, 200, "Status should be 200");
  strictEqual(response.statusText, "OK", 'Status text should be "OK"');
  strictEqual(response.type, "default", 'Response type should be "default"');
  ok(
    response.headers instanceof Headers,
    "Headers should be an instance of Headers",
  );
  strictEqual(response.url, "", "URL should be empty");
}

async function testResponseBodyMethods() {
  // Test 2: Response body methods (text, json, arrayBuffer, bytes)
  const responseText = new Response("Hello, world!", {
    headers: { "Content-Type": "text/plain" },
  });
  const text = await responseText.text();
  strictEqual(text, "Hello, world!", 'Text should be "Hello, world!"');

  const responseJson = new Response(JSON.stringify({ foo: "bar" }), {
    headers: { "Content-Type": "application/json" },
  });
  const json = await responseJson.json();
  deepStrictEqual(json, { foo: "bar" }, "JSON should be parsed correctly");

  const byteArray = new Uint8Array([1, 2, 3]);
  const responseBytes = new Response(byteArray);
  const bytes = await responseBytes.arrayBuffer();
  deepStrictEqual(
    new Uint8Array(bytes),
    byteArray,
    "Bytes should match the original Uint8Array",
  );

  const buffer = byteArray.buffer;
  const responseArrayBuffer = new Response(buffer);
  const arrayBuffer = await responseArrayBuffer.arrayBuffer();
  strictEqual(
    arrayBuffer.byteLength,
    buffer.byteLength,
    "ArrayBuffer should match the original ArrayBuffer",
  );
}

async function testResponseBodyUsed() {
  // Test 3: Body used state
  const response = new Response("Hello, world!");
  ok(!response.bodyUsed, "bodyUsed should be false before reading body");

  await response.text();
  ok(response.bodyUsed, "bodyUsed should be true after reading body");
}

async function testResponseRedirected() {
  // Test 4: Redirected state
  const response = new Response(null);
  ok(!response.redirected, "Response should not be redirected initially");

  // Simulate a redirected response
  const redirectResponse = Response.redirect("https://example.com", 302);
  strictEqual(
    redirectResponse.status,
    302,
    "Redirected response should have status 302",
  );
  strictEqual(
    redirectResponse.headers.get("Location"),
    "https://example.com/",
    "Redirected response should have Location header",
  );
}

async function testResponseCloneNotImplemented() {
  // Test 5: Clone method not implemented
  const response = new Response("Hello, world!");
  throws(() => response.clone(), Error, 'Clone should throw "Not implemented"');
}

async function testResponseStaticMethods() {
  // Test 6: Static method Response.json()
  const jsonResponse = Response.json({ foo: "bar" });
  const json = await jsonResponse.json();
  deepStrictEqual(
    json,
    { foo: "bar" },
    "Response.json() should create a JSON response",
  );

  // Test 7: Static method Response.error()
  const errorResponse = Response.error();
  strictEqual(errorResponse.status, 0, "Error response should have status 0");
  strictEqual(
    errorResponse.type,
    "error",
    'Error response should have type "error"',
  );

  // Test 8: Static method Response.redirect()
  const redirectUrl = "https://example.com/";
  const redirectResponse = Response.redirect(redirectUrl, 302);
  strictEqual(
    redirectResponse.status,
    302,
    "Redirect response should have status 302",
  );
  strictEqual(
    redirectResponse.headers.get("Location"),
    redirectUrl,
    "Redirect response should have Location header",
  );
}

async function testResponseStatusConstraints() {
  // Test 9: Status code constraints
  throws(
    () => new Response(null, { status: 600 }),
    RangeError,
    "Status code 600 should throw a RangeError",
  );
  throws(
    () => new Response(null, { status: 199 }),
    RangeError,
    "Status code 199 should throw a RangeError",
  );
}

async function testResponseStatusTextConstraints() {
  // Test 10: Status text constraints
  throws(
    () => new Response(null, { statusText: "Invalid\nText" }),
    TypeError,
    "Status text with invalid characters should throw a TypeError",
  );
}

async function testResponseNullBodyStatus() {
  // Test 11: Null body status constraint
  throws(
    () => new Response("Body", { status: 204 }),
    TypeError,
    "Status code 204 should not allow a body",
  );
}

// Run All Tests
async function runResponseTests() {
  console.log("Running Response tests...");

  try {
    await testResponseConstructor();
    console.log("testResponseConstructor passed");

    await testResponseBodyMethods();
    console.log("testResponseBodyMethods passed");

    await testResponseBodyUsed();
    console.log("testResponseBodyUsed passed");

    await testResponseRedirected();
    console.log("testResponseRedirected passed");

    await testResponseCloneNotImplemented();
    console.log("testResponseCloneNotImplemented passed");

    await testResponseStaticMethods();
    console.log("testResponseStaticMethods passed");

    await testResponseStatusConstraints();
    console.log("testResponseStatusConstraints passed");

    await testResponseStatusTextConstraints();
    console.log("testResponseStatusTextConstraints passed");

    await testResponseNullBodyStatus();
    console.log("testResponseNullBodyStatus passed");

    console.log("All Response tests passed");
  } catch (err) {
    console.error("Test failed:", err.message);
  }
}

runResponseTests();
