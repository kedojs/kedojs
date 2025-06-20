

const testDir = new Kedo.DirEntry({
  parentPath: 'testDir',
  name: 'Testing',
  isDir: true,
  isFile: false,
  isSymlink: true
});

console.log(testDir.name);
console.log(testDir instanceof Kedo.DirEntry);

Kedo.readDir('./tests').then((data) => {
  console.log("readDir");
  console.log(Array.isArray(data));
  console.log("Is DirEntry", data[0] instanceof Kedo.DirEntry);
  console.log(JSON.stringify(data));
  console.log(data);
  console.log(Object.keys(data[0]));
  console.log(data.length);
}).catch((err) => {
  console.log(err);
});