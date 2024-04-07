const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(response.body.locked);

let body = Uint8Array.from([]);
for await (const chunk of response.body) {
    body += chunk;
}

console.log(body);
console.log(response.body.locked);