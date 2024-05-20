import { readFile, writeFile, rm } from 'node:fs/promises';

for (let i = 0; i < 100; i++) {
    const content = await readFile('./local/examples/fs/data.txt', { encoding: 'utf8' });
    console.log(content);

    await writeFile(`./local/examples/fs/mocks/data-bun-${i}.txt`, content)
    console.log("Content written to file");

    rm(`./local/examples/fs/mocks/data-bun-${i}.txt`);
}
