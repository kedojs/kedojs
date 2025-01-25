// import { op_fetch_send } from "ext:core/ops";
import {
  errorReadableStream,
  readableStreamForRid
} from "ext:deno_web/06_streams.js";
console.log(typeof op_fetch_send, errorReadableStream, readableStreamForRid);

async function testServerListen() {
  // op_fetch_send("GET", "http://localhost:8000");
  // Test 1: Passing URLSearchParams as the body
  Deno.serve({
    hostname: "localhost",
    port: 8000,
  },
    (req) => {
      let timer;
      const body = new ReadableStream({
        async start(controller) {
          // timer = setInterval(() => {
          controller.enqueue("Hello, World!\n");
          controller.close();
          // }, 1);
        },
        cancel() {
          clearInterval(timer);
        },
      });
      return new Response(body.pipeThrough(new TextEncoderStream()), {
        headers: {
          "content-type": "text/plain; charset=utf-8",
        },
      });
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

// import { ReadableStream } from "@kedo/stream";

// async function testServerListen() {
//   // Test 1: Passing URLSearchParams as the body
//   Kedo.serve(
//     (_req) => {
//       // return new Response("Hello, world");
//       // stream
//       const body = new ReadableStream({
//         type: "bytes",
//         start(controller) {
//           // let hello_word_bytes = ;
//           // timer = setInterval(() => {
//           controller.enqueue(new TextEncoder().encode("Hello, World!\n"));
//           controller.close();
//           // }, 1);
//         },
//         cancel() { },
//       });

//       return new Response(body, {
//         headers: {
//           "content-type": "text/plain; charset=utf-8",
//         },
//       });
//     },
//     {
//       onListen({ port, hostname }) {
//         console.log(`Server started at ${hostname}:${port}`);
//       },
//     });
// }

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
