import { ReadableStream } from "@kedo/stream";

async function testServerListen() {
  // Test 1: Passing URLSearchParams as the body
  Kedo.serve(
    (_req) => {
      // return new Response("Hello, world");
      // stream
      const body = new ReadableStream({
        type: "bytes",
        start(controller) {
          // let hello_word_bytes = ;
          // timer = setInterval(() => {
          controller.enqueue(new TextEncoder().encode("Hello, World!\n"));
          controller.close();
          // }, 1);
        },
        cancel() { },
      });

      return new Response(body, {
        headers: {
          "content-type": "text/plain; charset=utf-8",
        },
      });
    },
    {
      onListen({ port, hostname }) {
        console.log(`Server started at ${hostname}:${port}`);
      },
    });
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
