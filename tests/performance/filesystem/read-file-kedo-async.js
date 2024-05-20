

for (let i = 0; i < 100; i++) {
    const content = await Kedo.readFile('./local/examples/fs/data.txt');
    console.log(content);

    await Kedo.writeFile(`./local/examples/fs/mocks/data-kedo-${i}.txt`, content)
    console.log("Content written to file");

    await Kedo.remove(`./local/examples/fs/mocks/data-kedo-${i}.txt`, false);
}
