import assert from "@kedo/assert";

async function testServerListenAbort() {
    const abortController = new AbortController();
    // Test 1: Passing URLSearchParams as the body
    Kedo.serve((_req) => new Response("Hello, world"), {
        signal: abortController.signal,
        onListen({ port, hostname }) {
            console.log(`Server started at ${hostname}:${port}`);
        },
    });

    await new Promise((resolve) => setTimeout(resolve, 500)).then(() => {
        abortController.abort();
    });
}

async function testServerCustomHostnameAndPort() {
    const abortController = new AbortController();
    let tPort;
    // Test 1: Passing URLSearchParams as the body
    Kedo.serve((_req) => new Response("Hello, world"), {
        hostname: "localhost",
        port: 8081,
        signal: abortController.signal,
        onListen({ port, hostname }) {
            tPort = port;
            console.log(`Server started at ${hostname}:${port}`);
        },
    });
    await new Promise((resolve) => setTimeout(resolve, 100)).then(() => {
        abortController.abort();
        assert.strictEqual(tPort, "8081");
    });
}

// Execute tests
async function runTests() {
    console.log("Running tests...");
    try {
        await testServerListenAbort();
        console.log("testServerListenAbort passed");

        await testServerCustomHostnameAndPort();
        console.log("testServerCustomHostnameAndPort passed");

        console.log("All tests passed");
    } catch (err) {
        console.error("Error: ", err.message);
    }
}

await runTests();
