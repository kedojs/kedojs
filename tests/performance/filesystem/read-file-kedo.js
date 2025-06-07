for (let i = 0; i < 4000; i++) {
  const content = Kedo.readFileSync(
    "/Users/kcaicedo/Documents/Projects/kedojs/local/examples/fs/data.txt",
  );

  Kedo.writeFileSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/data-kedo-${i}.txt`,
    content,
  );

  Kedo.removeSync(
    `/Users/kcaicedo/Documents/Projects/kedojs/local/examples/data-kedo-${i}.txt`,
  );
}

console.log("Multiple ReadFileSync test passed.\n");
