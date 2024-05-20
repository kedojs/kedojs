
for (let i = 0; i < 100; i++) {
    const content = await Deno.readTextFile('./local/examples/fs/data.txt');
    console.log(content);

    await Deno.writeTextFile(`./local/examples/fs/mocks/data-${i}.txt`, content)
    console.log("Content written to file");

    await Deno.remove(`./local/examples/fs/mocks/data-${i}.txt`);
}
