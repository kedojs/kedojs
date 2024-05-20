const { readFileSync, writeFileSync, rmSync } = require('node:fs');

for (let i = 0; i < 100; i++) {
    const content = readFileSync('./local/examples/fs/data.txt', { encoding: 'utf8' });
    console.log(content);

    writeFileSync(`./local/examples/fs/mocks/data-bun-${i}.txt`, content)
    console.log("Content written to file");

    rmSync(`./local/examples/fs/mocks/data-bun-${i}.txt`);
}
