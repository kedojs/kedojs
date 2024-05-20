
for (let i = 0; i < 100; i++) {
    const content = Deno.readTextFileSync('./local/examples/fs/data.txt');
    console.log(content);

    Deno.writeTextFileSync(`./local/examples/fs/mocks/data-${i}.txt`, content)
    console.log("Content written to file");

    Deno.removeSync(`./local/examples/fs/mocks/data-${i}.txt`);
}
