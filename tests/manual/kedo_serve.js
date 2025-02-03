// import { ReadableStream } from "@kedo/stream";

const encoder = new TextEncoder();

async function testServerListen() {
  // Test 1: Passing URLSearchParams as the body
  Kedo.serve(
    async (_req) => {
      // return new Response("Hello, world");
      // stream
      const body = "Hello, World!\n";
      // const body = new ReadableStream({
      //   type: "bytes",
      //   start(controller) {
      //     controller.enqueue(encoder.encode("Hello, World! 1\n"));
      //     controller.enqueue(encoder.encode("Hello, World! 2\n"));
      //   },
      //   async pull(controller) {
      //     controller.enqueue(encoder.encode("Hello, World! 4\n"));
      //     controller.close();
      //   },
      //   cancel() { },
      // });

      // const headers = _req.headers;
      // const url = _req.url;
      // const method = _req.method;
      // const data = await _req.text();
      // // console.log("data: ", data);
      // _req.signal;

      return new Response(body, {
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "server": "kedo/1.0",
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
