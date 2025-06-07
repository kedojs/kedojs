for (let i = 0; i < 4000; i++) {
  const content = Deno.readTextFileSync(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
  );

  Deno.writeTextFileSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-${i}.txt`,
    content,
  );

  Deno.removeSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/mocks/data-${i}.txt`,
  );
}

console.log("Multiple ReadFileSync test passed.\n");
