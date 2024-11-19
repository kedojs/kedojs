const encoder = new TextEncoder();
const fileContent = await Deno.open('tests/file-to-send.txt');

// const readableStream = new ReadableStream({
//     start(controller) {
//         controller.enqueue(fileContent);
//         controller.enqueue();
//         controller.close();
//     }
// });
const inputReader = fileContent.readable;

const response = await fetch('http://localhost:3000', {
    method: 'POST',
    body: inputReader,
    headers: {
        'Content-Type': 'application/octet-stream',
        'X-Test': 'test-value'
    }
});

console.log('Response status:', response.status);