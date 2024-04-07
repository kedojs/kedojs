// Kedo.writeFileSync();
// console.log("content written to file");

// const content = Kedo.readFileSync('./examples_/data.txt');

const content = Kedo.readFile('./examples/data.txt');

content.then((data) => {
  console.log(data);

  Kedo.writeFile('./data.txt', data).then(() => {
    console.log("content written to file");
    Kedo.removeSync('./data.txt');
  }).catch((err) => {
    console.log(err);
  });

}).catch((err) => {
  console.log(err);
});

const testDir = new KedoDirEntry('testDir', "Testing", true, false, true);

console.log(testDir);

Kedo.readDir('./examples').then((data) => {
  console.log("readDir");
  console.log(Array.isArray(data));
  console.log("Is KedoDirEntry", KedoDirEntry.is(data[0]));
  console.log(JSON.stringify(data));
  console.log(data);
  console.log(Object.keys(data[0]));
  console.log(data.length);
}).catch((err) => {
  console.log(err);
});

// // const content = "Hello, World!"
// console.log(content);
