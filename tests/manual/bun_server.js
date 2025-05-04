import { serve } from "bun";
import { readFile, rm, writeFile } from "node:fs/promises";

const port = 3000;
const encoder = new TextEncoder();

async function readFiles() {
    const random = `${Math.random()}-${Math.random()}-${new Date().getTime()}`;
    const content = await readFile(
        "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
        { encoding: "utf8" },
    );

    await writeFile(
        `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-bun-${random}.txt`,
        content,
    );

    await rm(
        `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-bun-${random}.txt`,
    );
}

serve({
    port,
    async fetch(request) {
        // stream
        // const body = "Hello, World!\n";
        // await readFiles();
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

        return new Response(body, {
            headers: { "Content-Type": "application/octet-stream" },
        });
    },
});

console.log(`Server running at http://localhost:${port}`);