const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(JSON.stringify(response));
// console.log(JSON.stringify(await response.json()));

for (const [key, value] of response.headers.entries()) {
    console.log(key, value);
}
