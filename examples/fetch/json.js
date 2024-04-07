const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(typeof response.body[Symbol.asyncIterator]);
console.log(response.body.locked);

try {
    console.log("JSON", await response.json());
} catch (error) {
    console.log(error);
}

console.log(response.body.locked);
