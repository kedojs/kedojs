import {
  ReadableStreamResource,
  op_read_sync_readable_stream,
  op_write_sync_readable_stream,
  op_read_readable_stream,
  op_write_readable_stream,
  op_close_readable_stream,
  op_wait_close_readable_stream,
} from "@kedo/internal/utils";
import { ReadableStream, readableStreamResource } from "@kedo/stream";
import assert from "@kedo/assert";

async function testReadableStreamResource() {
  console.log("Starting ReadableStreamResource test...");

  // Create a ReadableStream with some data
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new Uint8Array([1, 2, 3]));
      controller.enqueue(new Uint8Array([4, 5, 6]));
      controller.close();
    },
  });

  // Convert the ReadableStream to a ReadableStreamResource
  const resource = readableStreamResource(stream);

  // Read data from the resource and verify
  let result = await op_read_readable_stream(resource);
  assert.ok(
    result[0] === 1 && result[1] === 2 && result[2] === 3,
    `Expected [1, 2, 3], got ${result}`,
  );

  result = await op_read_readable_stream(resource);
  assert.ok(
    result[0] === 4 && result[1] === 5 && result[2] === 6,
    `Expected [4, 5, 6], got ${result}`,
  );

  // Attempt to read again, should return undefined (no more data)
  result = await op_read_readable_stream(resource);
  assert.ok(result === undefined, `Expected undefined, got ${result}`);

  console.log("ReadableStreamResource test passed.\n");
}

await testReadableStreamResource();
console.log("All tests passed.");
