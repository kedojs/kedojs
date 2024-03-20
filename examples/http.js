const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(response.title);

console.log(JSON.stringify(response));

Kedo.writeFileSync("todos.json", JSON.stringify(response));
