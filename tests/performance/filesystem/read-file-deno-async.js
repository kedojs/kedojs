async function readFiles(i) {
  const content = await Deno.readTextFile(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
  );
  await Deno.writeTextFile(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-${i}.txt`,
    content,
  );
  await Deno.remove(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-${i}.txt`,
  );
}

const promises = [];
for (let i = 0; i < 8000; i++) {
  promises.push(readFiles(i));
}

const result = await Promise.all(promises);
console.log("Multiple ReadFileSync test passed.\n");
