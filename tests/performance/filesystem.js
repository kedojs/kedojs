async function readFile(path) {
  const context = await Kedo.readFile("tests/console.js");
  const context2 = await Kedo.readFile("tests/console.js");
  const context3 = await Kedo.readFile("tests/console.js");
}

async function readFileDeno(path) {
  // const context = await Deno.readTextFile("tests/console.js");
  // const context2 = await Deno.readTextFile("tests/console.js");
  // const context3 = await Deno.readTextFile("tests/console.js");
}

const promises = [];
for (let i = 0; i < 4000; i++) {
  promises.push(readFile(i));
  // const context = await Deno.readTextFile("tests/console.js");
  // const context = await Kedo.readFile("tests/console.js");
  // console.log(context);

  // const context2 = await Deno.readTextFile("tests/console.js");
  // const context2 = await Kedo.readFile("tests/console.js");
  // console.log(context2);

  // const context3 = await Deno.readTextFile("tests/console.js");
  // const context3 = await Kedo.readFile("tests/console.js");
  // console.log(context3);
}

await Promise.all(promises);
