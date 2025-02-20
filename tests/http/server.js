
import assert from "@kedo/assert";

async function testServerListenAbort() {
    const abortController = new AbortController();
    // Test 1: Passing URLSearchParams as the body
    Kedo.serve(
        (_req) => new Response("Hello, world"),
        {
            signal: abortController.signal,
            onListen({ port, hostname }) {
                console.log(`Server started at ${hostname}:${port}`);
            },
        },
    );

    return new Promise((resolve) => setTimeout(resolve, 500)).then(() => {
        abortController.abort();
    });
}

async function testServerCustomHostnameAndPort() {
    const abortController = new AbortController();
    let tHostname;
    let tPort;
    // Test 1: Passing URLSearchParams as the body
    Kedo.serve(
        (_req) => new Response("Hello, world"),
        {
            hostname: "localhost",
            port: 8001,
            signal: abortController.signal,
            onListen({ port, hostname }) {
                tHostname = hostname;
                tPort = port;
                console.log(`Server started at ${tHostname}:${tPort}`);
            },
        },
    );
    return new Promise((resolve) => setTimeout(resolve, 100)).then(() => {
        abortController.abort();
        assert.strictEqual(tHostname, "127.0.0.1");
        assert.strictEqual(tPort, '8001');
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
        console.error(err.message);
    }
}

await runTests();