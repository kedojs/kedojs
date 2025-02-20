
const encoder = new TextEncoder();
async function testServerListen() {
    // Test 1: Passing URLSearchParams as the body
    Deno.serve({
        hostname: "0.0.0.0",
        port: 8000,
    },
        async (req) => {
            // const body = "Hello, World!\n";
            const body = new ReadableStream({
                type: "bytes",
                start(controller) {
                    controller.enqueue(encoder.encode("Hello, World! 1\n"));
                    controller.enqueue(encoder.encode("Hello, World! 2\n"));
                },
                async pull(controller) {
                    controller.enqueue(encoder.encode("Hello, World! 4\n"));
                    // enqueue more data more then 64kb
                    for (let i = 0; i < 160; i++) {
                        controller.enqueue(encoder.encode(`Hello, World! ${i}\n`.repeat(5)));
                    }

                    controller.close();
                },
                cancel() { },
            });
            // body.pipeThrough(new TextEncoderStream())
            let response = new Response(body, {
                headers: { "Content-Type": "application/octet-stream" },
            });
            // response.text();
            return response;
        });
    // Deno.serve(
    //   {
    //     onListen({ port, hostname }) {
    //       console.log(`Server started at ${hostname}:${port}`);
    //     },
    //   },
    //   (_req) => new Response("Hello, world"),
    // );
}

// Execute tests
async function runTests() {
    console.log("Running tests...");
    try {
        await testServerListen();
        console.log("testRequestArgs passed");

        console.log("All tests passed");
    } catch (err) {
        console.error(err.message);
    }
}

await runTests();
