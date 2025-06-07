import assert from "@kedo/assert";
import { ReadableStream, readableStreamResource } from "@kedo:int/std/stream";
import {
    op_acquire_unbounded_stream_reader,
    op_read_unbounded_stream,
} from "@kedo:op/web";

function asyncOp(fn, ...args) {
    return new Promise((resolve, reject) => {
        fn(...args, (err, result) => {
            if (err) {
                reject(err);
            } else {
                resolve(result);
            }
        });
    });
}

async function testReadableStreamResource() {
    console.log("Starting ReadableStreamResource test...");

    // Create a ReadableStream with some data
    const stream = new ReadableStream({
        type: "bytes",
        start(controller) {
            controller.enqueue(new Uint8Array([6, 5, 6]));
            controller.enqueue(new Uint8Array([7, 7, 7]));
            controller.close();
        },
    });

    // Convert the ReadableStream to a ReadableStreamResource
    const resource = readableStreamResource(stream);
    const reader = op_acquire_unbounded_stream_reader(resource);

    // Read data from the resource and verify
    let result = await asyncOp(op_read_unbounded_stream, reader);
    assert.ok(
        result[0] === 6 && result[1] === 5 && result[2] === 6,
        `Expected [1, 2, 3], got ${result}`,
    );

    result = await asyncOp(op_read_unbounded_stream, reader);
    assert.ok(
        result[0] === 7 && result[1] === 7 && result[2] === 7,
        `Expected [4, 5, 6], got ${result}`,
    );

    // Attempt to read again, should return undefined (no more data)
    result = await asyncOp(op_read_unbounded_stream, reader);
    assert.ok(result === -1, `Expected undefined, got ${result}`);

    console.log("ReadableStreamResource test passed.\n");
}

// benchmark
async function testReadableStreamResourceBenchmark() {
    console.log("Starting ReadableStreamResource benchmark...");

    // Create a ReadableStream with some data
    const stream = new ReadableStream(
        {
            type: "bytes",
            start(controller) {
                for (let i = 0; i < 1000; i++) {
                    controller.enqueue(new Uint8Array([9, 8, 2]));
                }
                controller.close();
            },
        },
        { highWaterMark: 1001 },
    );

    // Convert the ReadableStream to a ReadableStreamResource
    const resource = readableStreamResource(stream);
    const reader = op_acquire_unbounded_stream_reader(resource);

    // Read data from the resource and verify
    let result;
    let start = Date.now();
    console.log("Starting benchmark...");
    while ((result = await asyncOp(op_read_unbounded_stream, reader))) {
        // Do nothing
    }
    let end = Date.now();
    console.log(`Time taken: ${end - start}ms`);

    console.log("ReadableStreamResource benchmark passed.\n");
}

// mutiples ReadableStreamResource
async function testMultipleReadableStreamResource() {
    console.log("Starting MultipleReadableStreamResource test...");

    const promises = [];
    const streamFn = async (num) => {
        // Create a ReadableStream with some data
        const stream = new ReadableStream({
            type: "bytes",
            start(controller) {
                controller.enqueue(new Uint8Array([1, 2, 3]));
                controller.enqueue(new Uint8Array([4, 5, 6]));
                controller.close();
            },
        });

        const resource = readableStreamResource(stream);
        const reader = op_acquire_unbounded_stream_reader(resource);

        let result = await asyncOp(op_read_unbounded_stream, reader);
        assert.ok(
            result[0] === 1 && result[1] === 2 && result[2] === 3,
            `Expected [1, 2, 3], got ${result}`,
        );

        result = await asyncOp(op_read_unbounded_stream, reader);
        assert.ok(
            result[0] === 4 && result[1] === 5 && result[2] === 6,
            `Expected [4, 5, 6], got ${result}`,
        );

        // Attempt to read again, should return undefined (no more data)
        result = await asyncOp(op_read_unbounded_stream, reader);
        assert.ok(result === undefined, `Expected undefined, got ${result}`);
    };

    for (let i = 0; i < 4000; i++) {
        promises.push(streamFn(i));
    }

    await Promise.all(promises);
    console.log("MultipleReadableStreamResource test passed.\n");
}

async function runAllTests() {
    await testReadableStreamResource();
    await testReadableStreamResourceBenchmark();
    await testMultipleReadableStreamResource();
}

await runAllTests();
console.log("All tests passed.");
