import { deepStrictEqual, ok, rejects, strictEqual } from "@kedo/assert"; // Replace with the correct path to your assertion library
import { ReadableStream } from "@kedo/stream";

// Define the base URL for testing
const BASE_URL = "https://httpbin.org"; // Use a public echo server for testing
const BASE_HTTP_URL = "http://httpbin.org"; // Use a public echo server for testing

// Test Suite for the Fetch API
async function testFetchBasicGetRequest() {
  // Test 1: Basic GET request
  const response = await fetch(`${BASE_URL}/get`, {
    headers: { "User-Agent": "curl/8.7.1" },
  });
  strictEqual(response.status, 200, "Status should be 200 for GET request");
  ok(
    response.headers instanceof Headers,
    "Headers should be an instance of Headers",
  );

  const data = await response.json();
  strictEqual(
    data.url,
    `${BASE_URL}/get`,
    "URL in response should match the request URL",
  );
}

async function testHpttRequest() {
  // Test 1: Basic GET request
  const response = await fetch(`${BASE_HTTP_URL}/get`, {
    headers: { "User-Agent": "curl/8.7.1" },
  });
  strictEqual(response.status, 200, "Status should be 200 for GET request");
  const data = await response.json();
  strictEqual(
    data.url,
    `${BASE_HTTP_URL}/get`,
    "URL in response should match the request URL",
  );
}

async function testFetchPostRequestWithBody() {
  // Test 2: POST request with JSON body
  const payload = { foo: "bar" };
  const response = await fetch(`${BASE_URL}/post`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  strictEqual(response.status, 200, "Status should be 200 for POST request");
  const data = await response.json();
  deepStrictEqual(
    data.json,
    payload,
    "Server should receive the correct JSON body",
  );
}

async function testFetchRequestHeaders() {
  // Test 3: Sending custom headers
  const response = await fetch(`${BASE_URL}/headers`, {
    headers: { "X-Custom-Header": "TestValue" },
  });

  strictEqual(
    response.status,
    200,
    "Status should be 200 when sending headers",
  );
  const data = await response.json();
  strictEqual(
    data.headers["X-Custom-Header"],
    "TestValue",
    "Server should receive the custom header",
  );
}

async function testFetchResponseHeaders() {
  // Test 4: Reading response headers
  const response = await fetch(
    `${BASE_URL}/response-headers?X-Test-Header=TestValue`,
  );
  strictEqual(
    response.status,
    200,
    "Status should be 200 when fetching response headers",
  );
  strictEqual(
    response.headers.get("X-Test-Header"),
    "TestValue",
    "Response should contain the custom header",
  );
}

async function testFetchNotFound() {
  // Test 6: Handling 404 Not Found
  const response = await fetch(`${BASE_URL}/status/404`);
  strictEqual(response.status, 404, "Status should be 404 for Not Found");
}

async function testFetchRedirectHandling() {
  // Test 7: Handling redirects
  const response = await fetch(`${BASE_URL}/redirect/1`);
  strictEqual(response.status, 200, "Status should be 200 after redirect");
  const data = await response.json();
  strictEqual(
    data.url,
    `${BASE_URL}/get`,
    "URL should be updated after redirect",
  );
}

async function testFetchNoRedirect() {
  // Test 8: Handling manual redirect mode
  const response = await fetch(`${BASE_URL}/redirect/1`, {
    redirect: "manual",
  });
  strictEqual(
    response.status,
    302,
    "Status should be 302 when redirects are manual",
  );
  strictEqual(
    response.headers.get("Location"),
    `/get`,
    "Location header should be set",
  );
}

async function testFetchRequestMethods() {
  // Test 11: Sending various HTTP methods
  const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"];
  for (const method of methods) {
    const response = await fetch(`${BASE_URL}/anything`, { method });
    strictEqual(
      response.status,
      200,
      `Status should be 200 for ${method} request`,
    );
    const data = await response.json();
    strictEqual(data.method, method, `Server should receive ${method} request`);
  }
}

async function testFetchInvalidMethod() {
  // Test 12: Using an invalid HTTP method
  await rejects(
    fetch(`${BASE_URL}/anything`, { method: "INVALID" }),
    TypeError,
    "Using an invalid HTTP method should throw a TypeError",
  );
}

async function testFetchAbortController() {
  // Test 9: Aborting a fetch request
  const controller = new AbortController();
  const signal = controller.signal;

  setTimeout(() => {
    controller.abort();
  }, 100); // Abort after 10ms
  await rejects(
    fetch(`${BASE_URL}/delay/5`, { signal }),
    DOMException,
    "Aborted fetch should throw a DOMException",
  );
}

async function testFetchRequestBodyUsed() {
  // Test 13: Reusing a Request body
  const body = new ReadableStream({
    start(controller) {
      controller.enqueue(new Uint8Array([1, 2, 3]));
      controller.close();
    },
  });
  const request = new Request(`${BASE_URL}/post`, { method: "POST", body });

  await fetch(request);
  await rejects(
    fetch(request),
    TypeError,
    "Reusing a consumed Request body should throw a TypeError",
  );
}

async function testFetchResponseBodyUsed() {
  // Test 14: Reusing a Response body
  const response = await fetch(`${BASE_URL}/stream/10`);
  const reader = response.body.getReader();
  await reader.read();

  await rejects(
    response.text(),
    TypeError,
    "Reusing a consumed Response body should throw a TypeError",
  );
}

async function testFetchResponseStreamJson() {
  const response = await fetch(`${BASE_URL}/stream/10`);
  const reader = response.body.getReader();

  let totalChunk = 0;
  let listItems = [];
  let decoder = new TextDecoder("utf-8");
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    const jsonContext = decoder.decode(value).trim().split("\n");
    jsonContext.forEach((item) => {
      totalChunk += 1;
      listItems.push(JSON.parse(item));
    });
  }

  strictEqual(totalChunk, 10, "Should have received 10 bytes from stream");
  strictEqual(
    listItems.length,
    10,
    "Should have received 10 bytes from stream",
  );
}

async function testFetchCredentials() {
  // Test 16: Fetch with credentials
  const response = await fetch(`${BASE_URL}/cookies`, {
    credentials: "include",
  });
  strictEqual(
    response.status,
    200,
    "Status should be 200 when sending credentials",
  );
}

async function testFetchCacheMode() {
  // Test 17: Fetch with cache mode
  const response = await fetch(`${BASE_URL}/cache`, { cache: "no-store" });
  strictEqual(
    response.status,
    200,
    "Status should be 200 when using cache mode",
  );
}

async function testFetchCustomRequest() {
  // Test 18: Using a custom Request object
  const request = new Request(`${BASE_URL}/get`, {
    method: "GET",
    headers: { "X-Custom-Header": "TestValue" },
  });
  const response = await fetch(request);
  strictEqual(
    response.status,
    200,
    "Status should be 200 when using custom Request",
  );
  const data = await response.json();
  strictEqual(
    data.headers["X-Custom-Header"],
    "TestValue",
    "Server should receive the custom header from Request object",
  );
}

//   async function testFetchBlobBody() {
//     // Test 19: Sending a Blob as the request body
//     const blob = new Blob(["Hello, world!"], { type: "text/plain" });
//     const response = await fetch(`${BASE_URL}/post`, {
//       method: "POST",
//       body: blob,
//     });

//     strictEqual(response.status, 200, "Status should be 200 when sending Blob as body");
//     const data = await response.json();
//     strictEqual(data.data, "Hello, world!", "Server should receive the Blob content");
//   }

//   async function testFetchFormDataBody() {
//     // Test 20: Sending FormData as the request body
//     const formData = new FormData();
//     formData.append("field1", "value1");
//     formData.append("field2", "value2");

//     const response = await fetch(`${BASE_URL}/post`, {
//       method: "POST",
//       body: formData,
//     });

//     strictEqual(response.status, 200, "Status should be 200 when sending FormData as body");
//     const data = await response.json();
//     deepStrictEqual(
//       data.form,
//       { field1: "value1", field2: "value2" },
//       "Server should receive the FormData content"
//     );
//   }

async function testFetchStreamResponse() {
  // Test 21: Handling streamed responses
  const response = await fetch(`${BASE_URL}/stream-bytes/1024`);
  strictEqual(
    response.status,
    200,
    "Status should be 200 for streamed response",
  );

  const reader = response.body.getReader();
  let totalBytes = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    totalBytes += value.length;
  }
  strictEqual(totalBytes, 1024, "Should have received 1024 bytes from stream");
}

async function testFetchMedia() {
  // Test 22: Fetching media content
  const response = await fetch(`${BASE_URL}/image/png`);
  strictEqual(response.status, 200, "Status should be 200 for image request");

  strictEqual(
    response.headers.get("Content-Type"),
    "image/png",
    "Content-Type should be image/png",
  );
  const array = await response.arrayBuffer();
  ok(
    array instanceof ArrayBuffer,
    "Response should be parsed as an ArrayBuffer",
  );
}

async function testFetchTimeout() {
  await rejects(
    fetch(`${BASE_URL}/delay/2`, { signal: AbortSignal.timeout(1000) }),
    DOMException,
    "Fetch should be aborted due to timeout",
  );
}

async function testFetchInvalidURL() {
  // Test 23: Fetch with an invalid URL
  await rejects(
    fetch("ht!tp://invalid-url"),
    TypeError,
    "Fetching an invalid URL should throw a TypeError",
  );
}

async function testFetchWithCredentialsOmit() {
  // Test 24: Fetch with credentials omitted
  const response = await fetch(`${BASE_URL}/cookies`, { credentials: "omit" });
  strictEqual(
    response.status,
    200,
    "Status should be 200 when credentials are omitted",
  );
}

async function testFetchTextResponse() {
  // Test 25: Fetching a plain text response
  const response = await fetch(`${BASE_URL}/robots.txt`);
  strictEqual(response.status, 200, "Status should be 200 for text response");
  const text = await response.text();
  ok(text.includes("User-agent"), "Response should contain robots.txt content");
}

async function testFetchJSONResponse() {
  // Test 26: Fetching and parsing JSON response
  const response = await fetch(`${BASE_URL}/json`);
  strictEqual(response.status, 200, "Status should be 200 for JSON response");
  const data = await response.json();
  ok(data.slideshow, "Response JSON should contain 'slideshow' property");
  strictEqual(
    response.headers.get("Content-Type"),
    "application/json",
    "Content-Type should be JSON",
  );
}

async function testFetchHEADRequest() {
  // Test 27: Making a HEAD request
  const response = await fetch(`${BASE_URL}/get`, { method: "HEAD" });
  strictEqual(response.status, 200, "Status should be 200 for HEAD request");
  const text = await response.text();
  strictEqual(text, "", "HEAD request should not have a response body");
}

async function testFetchDeleteRequest() {
  // Test 28: Making a DELETE request
  const response = await fetch(`${BASE_URL}/delete`, { method: "DELETE" });
  strictEqual(response.status, 200, "Status should be 200 for DELETE request");
  const _ = await response.json();
}

async function testFetchPutRequest() {
  // Test 29: Making a PUT request with a body
  const payload = { foo: "bar" };
  const response = await fetch(`${BASE_URL}/put`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  strictEqual(response.status, 200, "Status should be 200 for PUT request");
  const data = await response.json();
  deepStrictEqual(
    data.json,
    payload,
    "Server should receive the correct JSON body",
  );
}

async function testFetchPatchRequest() {
  // Test 30: Making a PATCH request with a body
  const payload = { foo: "bar" };
  const response = await fetch(`${BASE_URL}/patch`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  strictEqual(response.status, 200, "Status should be 200 for PATCH request");
  const data = await response.json();
  deepStrictEqual(
    data.json,
    payload,
    "Server should receive the correct JSON body",
  );
}

// Run All Tests
async function runFetchTests() {
  console.log("Running Fetch API tests...");

  try {
    await testFetchBasicGetRequest();
    console.log("testFetchBasicGetRequest passed");

    await testHpttRequest();
    console.log("testHpttRequest passed");

    await testFetchPostRequestWithBody();
    console.log("testFetchPostRequestWithBody passed");

    await testFetchRequestHeaders();
    console.log("testFetchRequestHeaders passed");

    await testFetchResponseHeaders();
    console.log("testFetchResponseHeaders passed");

    await testFetchNotFound();
    console.log("testFetchNotFound passed");

    await testFetchRedirectHandling();
    console.log("testFetchRedirectHandling passed");

    await testFetchNoRedirect();
    console.log("testFetchNoRedirect passed");

    await testFetchMedia();
    console.log("testFetchMedia passed");

    await testFetchResponseStreamJson();
    console.log("testFetchResponseStreamJson passed");

    // await testFetchRequestMethods();
    // console.log("testFetchRequestMethods passed");

    await testFetchInvalidMethod();
    console.log("testFetchInvalidMethod passed");

    await testFetchAbortController();
    console.log("testFetchAbortController passed");

    await testFetchRequestBodyUsed();
    console.log("testFetchRequestBodyUsed passed");

    await testFetchResponseBodyUsed();
    console.log("testFetchResponseBodyUsed passed");

    // await testFetchCredentials();
    // console.log("testFetchCredentials passed");

    // await testFetchCacheMode();
    // console.log("testFetchCacheMode passed");

    await testFetchCustomRequest();
    console.log("testFetchCustomRequest passed");

    //   await testFetchBlobBody();
    //   console.log("testFetchBlobBody passed");

    //   await testFetchFormDataBody();
    //   console.log("testFetchFormDataBody passed");

    await testFetchStreamResponse();
    console.log("testFetchStreamResponse passed");

    await testFetchTimeout();
    console.log("testFetchTimeout passed");

    await testFetchInvalidURL();
    console.log("testFetchInvalidURL passed");

    // await testFetchWithCredentialsOmit();
    // console.log("testFetchWithCredentialsOmit passed");

    await testFetchTextResponse();
    console.log("testFetchTextResponse passed");

    await testFetchJSONResponse();
    console.log("testFetchJSONResponse passed");

    await testFetchHEADRequest();
    console.log("testFetchHEADRequest passed");

    await testFetchDeleteRequest();
    console.log("testFetchDeleteRequest passed");

    await testFetchPutRequest();
    console.log("testFetchPutRequest passed");

    await testFetchPatchRequest();
    console.log("testFetchPatchRequest passed");

    console.log("All Fetch API tests passed");
  } catch (err) {
    console.error("Test failed: ", err.message);
  }
}

await runFetchTests();
