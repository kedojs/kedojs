async function readFiles(i) {
  const content = await Kedo.readFile(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
  );
  await Kedo.writeFile(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-kedo-${i}.txt`,
    content,
  );
  await Kedo.remove(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-kedo-${i}.txt`,
    false,
  );
}

const promises = [];
for (let i = 0; i < 8000; i++) {
  promises.push(readFiles(i));
}

const result = await Promise.all(promises);
console.log("Multiple ReadFileSync test passed.\n");
