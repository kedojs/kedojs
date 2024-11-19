import {
    strictEqual
} from "@kedo/assert"; // Replace with the correct path to your assertion library

// Define the base URL for testing
const BASE_URL = "https://httpbin.org"; // Use a public echo server for testing

// test gzip encoding 
async function testFetchWithGzipEncoding() {
    // Test 1: Basic GET request
    const response = await fetch(`${BASE_URL}/gzip`);
    strictEqual(response.status, 200, "Status should be 200 for GET request");
    const data = await response.json();

    strictEqual(data.gzipped, true, "URL in response should match the request URL");
    strictEqual(data.headers["Accept-Encoding"], "gzip, deflate, zstd, br", "Accept-Encoding header should be gzip");
    strictEqual(response.headers.get("Content-Encoding"), "gzip", "Content-Encoding header should be gzip");
}

async function testFetchWithDeflateEncoding() {
    // Test 2: Basic GET request
    const response = await fetch(`${BASE_URL}/deflate`);
    strictEqual(response.status, 200, "Status should be 200 for GET request");
    const data = await response.json();

    strictEqual(data.deflated, true, "URL in response should match the request URL");
    strictEqual(data.headers["Accept-Encoding"], "gzip, deflate, zstd, br", "Accept-Encoding header should be deflate");
    strictEqual(response.headers.get("Content-Encoding"), "deflate", "Content-Encoding header should be deflate");
}

async function testFetchWithZstdEncoding() {
    // Test 3: Basic GET request
    const response = await fetch(`${BASE_URL}/zstd`);
    strictEqual(response.status, 200, "Status should be 200 for GET request");
    const data = await response.json();

    strictEqual(data.zstd, true, "URL in response should match the request URL");
    strictEqual(data.headers["Accept-Encoding"], "gzip, deflate, zstd, br", "Accept-Encoding header should be zstd");
    strictEqual(response.headers.get("Content-Encoding"), "zstd", "Content-Encoding header should be zstd");
}

async function testFetchWithBrEncoding() {
    // Test 4: Basic GET request
    const response = await fetch(`${BASE_URL}/brotli`);
    strictEqual(response.status, 200, "Status should be 200 for GET request");
    const data = await response.json();

    strictEqual(data.brotli, true, "URL in response should match the request URL");
    strictEqual(data.headers["Accept-Encoding"], "gzip, deflate, zstd, br", "Accept-Encoding header should be br");
    strictEqual(response.headers.get("Content-Encoding"), "br", "Content-Encoding header should be br");
}

// Run All Tests
async function runFetchTests() {
    console.log("Running Fetch API tests...");

    try {
        await testFetchWithGzipEncoding();
        console.log("testFetchWithGzipEncoding passed");

        await testFetchWithBrEncoding();
        console.log("testFetchWithBrEncoding passed");

        await testFetchWithDeflateEncoding();
        console.log("testFetchWithDeflateEncoding passed");

        console.log("All Fetch API tests passed");
    } catch (err) {
        console.error("Test failed: ", err.message);
    }
}

await runFetchTests();