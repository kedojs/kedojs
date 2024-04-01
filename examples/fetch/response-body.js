const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

// console.log(JSON.stringify(response.body[Symbol.asyncIterator]));
// console.log(response.body.getReader);
console.log(typeof response.body[Symbol.asyncIterator]);
console.log(response.body.locked);

// console.log(JSON.stringify(await response.json()));

try {
    await response.json()
    console.log("JSON");
} catch (error) {
    console.log(error);
}

// let body = Uint8Array.from([]);
// for await (const chunk of response.body) {
//     body += chunk;
// }

// console.log(body);
// console.log(response.body.locked);
// console.log(response.body.locked);