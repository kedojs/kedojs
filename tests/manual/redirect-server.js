const http = require('http');

let requestCount = 0;

const server = http.createServer((req, res) => {
    requestCount++;

    // console method
    console.log('Received request: ', req.method, req.url, requestCount);
    // prit all headers
    // console.log('Headers:', req.headers);

    if (requestCount <= 4) {
        console.log(`http://localhost:3000/${requestCount}`);
        res.writeHead(303, { 'Location': `http://localhost:3000/${requestCount}` });
        res.end();
    } else {
        requestCount = 0;
        // print request paylaod
        console.log('Received request payload:');
        req.pipe(process.stdout);
        res.writeHead(200, { 'Content-Type': 'text/plain' });
        res.end('Hello, world!');
    }
});

const PORT = 3000;
server.listen(PORT, () => {
    console.log(`Server is listening on port ${PORT}`);
});
