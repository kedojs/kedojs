

for (let i = 0; i < 100; i++) {
    const content = Kedo.readFileSync('./local/examples/fs/data.txt');
    console.log(content);

    Kedo.writeFileSync(`./local/examples/fs/mocks/data-kedo-${i}.txt`, content)
    console.log("Content written to file");

    Kedo.removeSync(`./local/examples/fs/mocks/data-kedo-${i}.txt`, false);
}
