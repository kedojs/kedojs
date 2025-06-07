import { ReadableStream } from "@kedo/stream";

const encoder = new TextEncoder();

// async function readFiles() {
//   const random = `${Math.random()}-${Math.random()}-${new Date().getTime()}`;
//   const content = await Kedo.readFile(
//     "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
//   );
//   await Kedo.writeFile(
//     `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-kedo-${random}.txt`,
//     content,
//   );
//   await Kedo.remove(
//     `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-kedo-${random}.txt`,
//     false,
//   );
// }

async function testServerListen() {
    // Test 1: Passing URLSearchParams as the body
    Kedo.serve(
        async (_req) => {
            // return new Response("Hello, world");
            // stream
            // const body = "Hello, World!\n";
            // console.log(encoder.encode("Hello, World! 1\n").byteLength, " K:T ", typeof encoder.encode("Hello, World! 1\n"));
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
                        controller.enqueue(
                            encoder.encode(`Hello, World! ${i}\n`.repeat(5)),
                        );
                    }

                    controller.close();
                },
            });
            // await readFiles();
            return new Response(body, {
                headers: { "content-type": "application/octet-stream" },
            });
        },
        {
            onListen({ port, hostname }) {
                console.log(`Server started at ${hostname}:${port}`);
            },
        },
    );
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
