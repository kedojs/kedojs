const response = await Kedo.fetch('https://jsonplaceholder.typicode.com/posts', {
    method: 'POST',
    body: JSON.stringify({
        title: 'foo',
        body: 'bar',
        userId: 1,
    }),
    headers: {
        'Content-type': 'application/json; charset=UTF-8',
    },
});

console.log(response.status);
console.log(response.statusText);

for (const [key, value] of response.headers) {
    console.log(`${key}: ${value}`);
}

console.log(response.url);
console.log(JSON.stringify(await response.json()));