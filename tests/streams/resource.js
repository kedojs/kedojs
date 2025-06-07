import assert from "@kedo/assert";
import { ReadableStream } from "@kedo/stream";
import {
    ReadableStreamResource,
    op_acquire_stream_reader,
    op_close_stream_resource,
    op_read_readable_stream,
    op_write_readable_stream,
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

    const streamFn = async (_) => {
        // Convert the ReadableStream to a ReadableStreamResource
        const resource = new ReadableStreamResource(64);
        const reader = op_acquire_stream_reader(resource);

        await (async () => {
            await asyncOp(
                op_write_readable_stream,
                resource,
                new Uint8Array([6, 5, 6]),
            );
            await asyncOp(
                op_write_readable_stream,
                resource,
                new Uint8Array([6, 5, 6]),
            );
            op_close_stream_resource(resource);
        })();

        let count = 0;

        while (true) {
            const data = await asyncOp(op_read_readable_stream, reader);
            if (data === undefined || count === 2) {
                break;
            }

            count++;
            assert.ok(
                data[0] === 6 && data[1] === 5 && data[2] === 6,
                `Expected [6, 5, 6], got ${data}`,
            );
        }

        assert.ok(count === 2, `Expected 2, got ${count}`);
    };

    const promises = [];
    for (let i = 0; i < 4000; i++) {
        promises.push(streamFn());
    }

    await Promise.all(promises);
    console.log("ReadableStreamResource test passed.\n");
}

async function runAllTests() {
    await testReadableStreamResource();
}

await runAllTests();
console.log("All tests passed.");
