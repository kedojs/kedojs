const { readFileSync, writeFileSync, rmSync } = require("node:fs");

for (let i = 0; i < 4000; i++) {
  const content = readFileSync(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
    {
      encoding: "utf8",
    },
  );

  writeFileSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/data-${i}.txt`,
    content,
  );

  rmSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/data-${i}.txt`,
  );
}

console.log("Multiple ReadFileSync test passed.\n");
