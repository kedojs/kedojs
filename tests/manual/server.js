const http = require('http');

const server = http.createServer((req, res) => {
    if (req.method === 'POST') {
        let data = [];

        req.on('data', chunk => {
            data.push(chunk);
        });

        req.on('end', () => {
            const binaryData = Buffer.concat(data);
            console.log('Received binary data:', binaryData);
            res.writeHead(200, { 'Content-Type': 'text/plain' });
            res.end('Binary data received');
        });

        req.on('error', err => {
            console.error('Error receiving data:', err);
            res.writeHead(500, { 'Content-Type': 'text/plain' });
            res.end('Error receiving data');
        });
    } else {
        res.writeHead(405, { 'Content-Type': 'text/plain' });
        res.end('Method not allowed');
    }
});

const PORT = 3000;
server.listen(PORT, () => {
    console.log(`Server is listening on port ${PORT}`);
});