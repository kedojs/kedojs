// import assert from "node:assert";
// import { ReadableStream } from "node:stream/web";
import assert from "@kedo/assert";
import { ReadableStream } from "@kedo/stream";

// Test: Basic ReadableStream Construction
const readableStream = new ReadableStream({
  start(controller) {
    controller.enqueue("test data");
    controller.close();
  },
});

assert.ok(readableStream, "ReadableStream should be created successfully");
assert.ok(
  typeof readableStream.getReader === "function",
  "ReadableStream should have a getReader method",
);

// Test: ReadableStreamDefaultReader
const reader = readableStream.getReader();

let result;
result = await reader.read();
console.log("Test: ", result);
assert.deepStrictEqual(
  result,
  { value: "test data", done: false },
  "Reader should read the correct data",
);
const result2 = await reader.read();
console.log("Test 2: ", result2);
assert.deepStrictEqual(
  result2,
  { value: undefined, done: true },
  "Reader should indicate the stream is closed",
);
reader.releaseLock();

const byobReadableStream = new ReadableStream({
  start(controller) {
    const buffer = new ArrayBuffer(4);
    const view = new Uint8Array(buffer);
    view[0] = 1;
    view[1] = 2;
    view[2] = 3;
    view[3] = 5;
    controller.enqueue(view);
    controller.close();
  },
  type: "bytes",
});

// Test: ReadableStreamBYOBReader
const byobReader = byobReadableStream.getReader({ mode: "byob" });
const buffer = new ArrayBuffer(4);
const view = new Uint8Array(buffer);

await byobReader
  .read(view)
  .then((result) => {
    console.log("Test 3: ", result, view);
    assert.deepStrictEqual(
      result.value[0],
      1,
      "BYOB Reader should read the correct data",
    );
    // return byobReader.read(view);
  })
  .finally(() => {
    byobReader.releaseLock();
  });

// |----------------------------------------|
// | Test: ReadableStream with String Data  |
// |----------------------------------------|
const stringStream = new ReadableStream({
  start(controller) {
    controller.enqueue("Hello");
    controller.enqueue("World");
    controller.close();
  },
});

const stringReader = stringStream.getReader();

result = await stringReader.read();
console.log("Test 6: ", result);
assert.deepStrictEqual(
  result,
  { value: "Hello", done: false },
  "Should read the first string data",
);

result = await stringReader.read();
console.log("Test 7: ", result);
assert.deepStrictEqual(
  result,
  { value: "World", done: false },
  "Should read the second string data",
);

result = await stringReader.read();
console.log("Test 8: ", result);
assert.deepStrictEqual(
  result,
  { value: undefined, done: true },
  "Should indicate the stream is closed",
);

console.log("Test 9: ");
stringReader.releaseLock();

// |----------------------------------|
// | Test: ReadableStream with Errors |
// |----------------------------------|
const errorStream = new ReadableStream({
  start(controller) {
    controller.enqueue("Initial data");
    setTimeout(() => controller.error(new Error("Stream error")), 10);
  },
});

const errorReader = errorStream.getReader();
result = await errorReader.read();
console.log("Test 10: ", result);
assert.deepStrictEqual(
  result,
  { value: "Initial data", done: false },
  "Should read the initial data before error",
);

await new Promise((resolve) => setTimeout(resolve, 11));
await assert.rejects(
  () => errorReader.read(),
  (err) => {
    assert.strictEqual(
      err.message,
      "Stream error",
      "Should reject with the error message",
    );
    return true;
  },
);

console.log("Test 12: ");
errorReader.releaseLock();

// |----------------------------------| (10 tests, 16 assertions
// | Test: ReadableStream with Delays |
// |----------------------------------|
const delayStream = new ReadableStream({
  start(controller) {
    setTimeout(() => controller.enqueue("Delayed data 1"), 100);
    setTimeout(() => controller.enqueue("Delayed data 2"), 200);
    setTimeout(() => controller.close(), 300);
  },
});

const delayReader = delayStream.getReader();

result = await delayReader.read();
console.log("Test 13: ", result);
assert.deepStrictEqual(
  result,
  { value: "Delayed data 1", done: false },
  "Should read the first delayed data",
);

result = await delayReader.read();
console.log("Test 14: ", result);
assert.deepStrictEqual(
  result,
  { value: "Delayed data 2", done: false },
  "Should read the second delayed data",
);

result = await delayReader.read();
console.log("Test 15: ", result);
assert.deepStrictEqual(
  result,
  { value: undefined, done: true },
  "Should indicate the stream is closed after delays",
);

console.log("Test 16: ");
delayReader.releaseLock();

// |---------------------------------|
// | Test: ReadableStream with Pull  |
// |---------------------------------|
const pullStream = new ReadableStream({
  start(controller) {
    this.counter = 0;
  },
  pull(controller) {
    if (this.counter < 3) {
      controller.enqueue(`Pull data ${this.counter}`);
      this.counter++;
    } else {
      controller.close();
    }
  },
});

const pullReader = pullStream.getReader();

result = await pullReader.read();
console.log("Test 17: ", result);
assert.deepStrictEqual(
  result,
  { value: "Pull data 0", done: false },
  "Should read the first pull data",
);

result = await pullReader.read();
console.log("Test 18: ", result);
assert.deepStrictEqual(
  result,
  { value: "Pull data 1", done: false },
  "Should read the second pull data",
);

result = await pullReader.read();
console.log("Test 19: ", result);
assert.deepStrictEqual(
  result,
  { value: "Pull data 2", done: false },
  "Should read the third pull data",
);

result = await pullReader.read();
console.log("Test 20: ", result);
assert.deepStrictEqual(
  result,
  { value: undefined, done: true },
  "Should indicate the stream is closed after pull",
);

console.log("Test 21: ");
pullReader.releaseLock();

// |-----------------------------------|
// | Test: ReadableStream with Cancel  |
// |-----------------------------------|
//
let isCancelled = false;
const cancelStream = new ReadableStream({
  start(controller) {
    controller.enqueue("Initial data");
  },
  cancel(reason) {
    isCancelled = true;
    console.log(`Stream canceled due to: ${reason}`);
  },
});

const cancelReader = cancelStream.getReader();
result = await cancelReader.read();
assert.deepStrictEqual(
  result,
  { value: "Initial data", done: false },
  "Should read the initial data before cancel",
);
result = await cancelReader.cancel("User requested cancel");
console.log("Test 22: ");
assert.strictEqual(isCancelled, true, "Should call the cancel method");
result = await cancelReader.read();
console.log("Test 22: ");
assert.deepStrictEqual(
  result,
  { value: undefined, done: true },
  "Should read the initial data before cancel",
);
console.log("Test 22L: ", result, isCancelled);
cancelReader.releaseLock();

// |-------------------------------------------------|
// | Test: ReadableStream with autoAllocateChunkSize |
// |-------------------------------------------------|
const autoAllocateStream = new ReadableStream({
  start(controller) {
    this.counter = 0;
  },
  pull(controller) {
    if (this.counter < 3) {
      const chunk = new Uint8Array(
        controller.byobRequest.view.buffer.byteLength,
      );
      chunk[0] = this.counter;
      controller.byobRequest.respondWithNewView(chunk);
      this.counter++;
    } else {
      controller.close();
    }
  },
  type: "bytes",
  autoAllocateChunkSize: 1,
});

const autoAllocateReader = autoAllocateStream.getReader({ mode: "byob" });
const viewTwo = new Uint8Array(new ArrayBuffer(1));

console.log("Data: ", viewTwo, viewTwo.byteLength);
result = await autoAllocateReader.read(viewTwo);
console.log("Data: ", viewTwo, result, viewTwo.byteLength, {
  value: result.value[0],
  done: false,
});
assert.deepStrictEqual(
  { value: result.value[0], done: false },
  { value: 0, done: false },
  "Should read the first allocated chunk",
);
console.log("Data2: ", viewTwo, viewTwo.byteLength);
result = await autoAllocateReader.read(new Uint8Array(new ArrayBuffer(1)));
assert.deepStrictEqual(
  { value: result.value[0], done: false },
  { value: 1, done: false },
  "Should read the second allocated chunk",
);

result = await autoAllocateReader.read(new Uint8Array(new ArrayBuffer(1)));
assert.deepStrictEqual(
  { value: result.value[0], done: false },
  { value: 2, done: false },
  "Should read the third allocated chunk",
);

autoAllocateReader.releaseLock();

// |-------------------------------------------------|
// | Test: ReadableStream with Backpressure Handling |
// |-------------------------------------------------|
const backpressureStream = new ReadableStream({
  start(controller) {
    let counter = 0;
    const produceData = () => {
      if (counter < 5) {
        controller.enqueue(`Data ${counter}`);
        counter++;
        setTimeout(produceData, 100); // Simulate slow producer
      } else {
        controller.close();
      }
    };
    produceData();
  },
});

const backpressureReader = backpressureStream.getReader();
let readCount = 0;

while (true) {
  const result = await backpressureReader.read();
  if (result.done) break;

  readCount++;
  assert.ok(
    result.value.startsWith("Data"),
    `Read data should start with 'Data', got ${result.value}`,
  );

  if (readCount < 3) {
    continue;
  } else {
    await new Promise((resolve) => setTimeout(resolve, 500)); // Simulate backpressure
  }
}

console.log("Result ReadCount: %d", readCount);
assert.strictEqual(readCount, 5, "Should have read all 5 pieces of data");
backpressureReader.releaseLock();

// |--------------------------------------|
// | Test: ReadableStream with Large Data |
// |--------------------------------------|
const largeDataStream = new ReadableStream({
  start(controller) {
    for (let i = 0; i < 1000; i++) {
      controller.enqueue(`Chunk ${i}`);
    }
    controller.close();
  },
});

const largeDataReader = largeDataStream.getReader();
let largeDataCount = 0;

while (true) {
  const result = await largeDataReader.read();
  if (result.done) break;

  largeDataCount++;
  assert.ok(
    result.value.startsWith("Chunk"),
    `Read data should start with 'Chunk', got ${result.value}`,
  );
}

console.log("Result LargeDataCount: %d", largeDataCount);
assert.strictEqual(largeDataCount, 1000, "Should have read all 1000 chunks");
largeDataReader.releaseLock();

const testReadable = new ReadableStream();
const testReadableReader = testReadable.getReader();

// |--------------------------------------|
// | Create ReadableStream from iterator  |
// |--------------------------------------|
const asyncIterator = (async function* () {
  yield 2;
  yield 4;
  yield 6;
})();

const myReadableStream = ReadableStream.from(asyncIterator);

let iteratorCount = 0;
for await (const value of myReadableStream) {
  iteratorCount++;
  assert.strictEqual(
    value,
    iteratorCount * 2,
    `Value should be ${iteratorCount}, got ${value}`,
  );
}

assert.strictEqual(iteratorCount, 3, "Should have read all 3 values");

console.log("ReadableStream tests passed");
