import { readFile, rm, writeFile } from "node:fs/promises";

async function readFiles(i) {
  const content = await readFile(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
    { encoding: "utf8" },
  );

  await writeFile(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-bun-${i}.txt`,
    content,
  );

  await rm(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-bun-${i}.txt`,
  );
}

const promises = [];
for (let i = 0; i < 8000; i++) {
  promises.push(readFiles(i));
}

const result = await Promise.all(promises);
console.log("Multiple ReadFileSync test passed.\n");
