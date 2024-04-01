const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(JSON.stringify(response));
// console.log(JSON.stringify(await response.json()));

for (const [key, value] of response.headers.entries()) {
    console.log(key, value);
}

// console.log(JSON.stringify());
// console.log(JSON.stringify(await response.json()));

// console.log(JSON.stringify(response));

// console.log(new Headers(response.headers));
// Kedo.writeFileSync("todos.json", JSON.stringify(response));

// const headers = new Headers({
//     "Content-Type": 'application/json',
//     "X-Custom-Header": 'custom value'
// });

// const readable = new ReadableStream({
//     start(controller) {
//         controller.enqueue("Hello ");
//         controller.enqueue("World");
//         controller.close();
//     }
// });
// const headers = new Headers([
//     ["Content-Type", 'application/json'],
//     ["X-Custom-Header", 'custom value']
// ]);

// for (const [key, value] of headers.entries()) {
//     console.log(key, value);
// }

// console.log(headers.get("Content-Type"));
// console.log(headers.get("X-Custom-Header"));
// console.log(headers.has("Content-Type"));
// console.log(headers.keys());
// console.log(headers.values());
// console.log(headers.entries());
// console.log(headers.delete("Content-Type"));
// console.log(headers.has("Content-Type"));
// headers.set("Content-Type", "application/text");
// console.log(headers.get("Content-Type"));
// console.log(headers.append("Content-Type", "application/json"));
// console.log(headers.get("Content-Type"));