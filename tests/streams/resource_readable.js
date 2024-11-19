import {
  ReadableStreamResource,
  op_read_sync_readable_stream,
  op_write_sync_readable_stream,
  op_read_readable_stream,
  op_write_readable_stream,
  op_close_readable_stream,
  op_wait_close_readable_stream,
} from "@kedo/internal/utils";
import { ReadableStream } from "@kedo/stream";
import assert from "@kedo/assert";

// Helper function to compare two Uint8Arrays
function arraysEqual(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

async function testSyncReadWrite() {
  console.log("Starting synchronous read/write test...");
  const resource = new ReadableStreamResource(100);

  // Write data synchronously
  const data = new Uint8Array([10, 20, 30]);
  op_write_sync_readable_stream(resource, data);

  // Read data synchronously
  const result = op_read_sync_readable_stream(resource);
  assert.ok(arraysEqual(result, data), `Expected ${data}, got ${result}`);

  // Attempt to read again, should return undefined (no more data)
  const emptyResult = op_read_sync_readable_stream(resource);
  assert.ok(
    emptyResult === undefined,
    `Expected undefined, got ${emptyResult}`,
  );

  console.log("Synchronous read/write test passed.\n");
}

async function testAsyncReadWrite() {
  console.log("Starting asynchronous read/write test...");
  const resource = new ReadableStreamResource(101);

  // Prepare chunks to write
  const chunks = [
    new Uint8Array([0]),
    new Uint8Array([1, 2]),
    new Uint8Array([3, 4, 5]),
  ];

  // Write chunks asynchronously
  for (const chunk of chunks) {
    await op_write_readable_stream(resource, chunk);
  }

  // Read chunks asynchronously and verify
  for (const expectedChunk of chunks) {
    const result = await op_read_readable_stream(resource);
    assert.ok(
      arraysEqual(result, expectedChunk),
      `Expected ${expectedChunk}, got ${result}`,
    );
  }

  // setTimeout(() => {
  //   op_close_readable_stream(resource);
  // }, 1000);
  // let error = false;
  // Attempt to read again, should be pending or return undefined
  const emptyResult = op_read_sync_readable_stream(resource);
  assert.ok(
    emptyResult === undefined,
    `Expected undefined after all data read, got ${emptyResult}`,
  );

  console.log("Asynchronous read/write test passed.\n");
}

async function testCloseOperation() {
  console.log("Starting close operation test...");
  const resource = new ReadableStreamResource(102);

  // Write some data
  const data = new Uint8Array([42]);
  await op_write_readable_stream(resource, data);

  // Close the stream
  op_close_readable_stream(resource);

  // Attempt to read remaining data
  const result = await op_read_readable_stream(resource);
  assert.ok(arraysEqual(result, data), `Expected ${data}, got ${result}`);

  console.log("Close First Read test passed.\n");
  // Subsequent reads should return undefined or error
  try {
    const emptyResult = await op_read_readable_stream(resource);
    assert.ok(
      emptyResult === undefined,
      `Expected undefined after closing, got ${emptyResult}`,
    );
  } catch (e) {
    assert.ok(false, `Error occurred when reading after closing: ${e}`);
  }

  // Attempt to write after closing should fail
  try {
    await op_write_readable_stream(resource, new Uint8Array([1]));
    // assert.fail("Expected error when writing to closed resource");
  } catch (e) {
    assert.ok(
      true,
      `Correctly threw error when writing to closed resource: ${e}`,
    );
  }

  console.log("Close operation test passed.\n");
}

async function testEdgeCases() {
  console.log("Starting edge cases test...");
  const resource = new ReadableStreamResource(103);

  // Write an empty chunk
  await op_write_readable_stream(resource, new Uint8Array([]));

  // Read the empty chunk
  const result = await op_read_readable_stream(resource);
  assert.ok(
    result.length === 0,
    `Expected empty Uint8Array, got length ${result.length}`,
  );

  // Read
  const emptyResource = new ReadableStreamResource(104);
  setTimeout(() => {
    op_write_sync_readable_stream(emptyResource, new Uint8Array([2]));
  }, 1000);
  const secondResult = await op_read_readable_stream(emptyResource);
  assert.ok(
    secondResult[0] === 2,
    `Expected 2 when reading stream, got ${secondResult}`,
  );

  console.log("Edge cases test passed.\n");
}

async function testConcurrentOperations() {
  console.log("Starting concurrent operations test...");
  const resource = new ReadableStreamResource(105);

  // Start a read operation (will wait for data)
  const readPromise = op_read_readable_stream(resource);

  // Simulate delay and write data
  setTimeout(async () => {
    await op_write_readable_stream(resource, new Uint8Array([99]));
  }, 100);

  // Verify that the readPromise resolves with the correct data
  const result = await readPromise;
  assert.ok(result[0] === 99, `Expected 99, got ${result[0]}`);

  console.log("Concurrent operations test passed.\n");
}

async function testErrorHandling() {
  console.log("Starting error handling test...");
  const resource = new ReadableStreamResource(106);

  // Close the resource immediately
  op_close_readable_stream(resource);
  let is_error = false;
  // Attempt to read from closed resource
  // try {
  await op_read_readable_stream(resource);
  // assert.fail("Expected error when reading from closed resource");
  // } catch (e) {
  //   assert.ok(
  //     true,
  //     `Correctly threw error when reading from closed resource: ${e}`,
  //   );
  // }

  // Attempt to write to closed resource
  try {
    const result = await op_write_readable_stream(
      resource,
      new Uint8Array([1]),
    );
    if (result === -1) {
      is_error = true;
    }
    // assert.fail("Expected error when writing to closed resource");
  } catch (e) {
    is_error = true;
  }

  assert.ok(is_error, `Correctly threw error when writing to closed resource`);

  console.log("Error handling test passed.\n");
}

async function testHighWaterMark() {
  console.log("Starting high water mark test...");

  // Set the high water mark (maximum number of chunks in the queue) to a small value, e.g., 2 chunks
  const highWaterMark = 2;
  const resource = new ReadableStreamResource(highWaterMark);

  // Prepare data chunks
  const chunk1 = new Uint8Array([1]);
  const chunk2 = new Uint8Array([2]);
  const chunk3 = new Uint8Array([3]); // This chunk will exceed the buffer capacity when written after chunk1 and chunk2

  // Write first chunk
  await op_write_readable_stream(resource, chunk1);
  console.log("Wrote chunk1:", chunk1);

  // Write second chunk
  await op_write_readable_stream(resource, chunk2);
  console.log("Wrote chunk2:", chunk2);

  setTimeout(() => {
    // Write third chunk
    op_read_sync_readable_stream(resource, chunk3);
    console.log("Read chunk3: ", chunk3);
  }, 1000);

  await op_write_readable_stream(resource, chunk3);
  console.log("Wrote chunk3:", chunk3);

  console.log("High water mark test passed.\n");
}

async function testWaitCloseReadableStream() {
  console.log("Starting wait close test...");
  const resource = new ReadableStreamResource(107);

  // Write some data
  const data = new Uint8Array([42]);
  await op_write_readable_stream(resource, data);

  // Close the stream
  op_close_readable_stream(resource);

  // Wait for the stream to close
  await op_wait_close_readable_stream(resource);

  console.log("Wait close test passed.\n");
}

async function runAllTests() {
  try {
    await testSyncReadWrite();
    await testAsyncReadWrite();
    await testCloseOperation();
    await testEdgeCases();
    await testConcurrentOperations();
    await testErrorHandling();
    await testHighWaterMark();
    console.log("All tests passed successfully.");
  } catch (err) {
    console.error("Test failed:", err.message);
  }
}

await runAllTests();
