# KedoJS

![KedoJS](./docs/logo-sm.jpg)

Kedo is a fast Javascript runtime.

Kedo is in an experimental stage, lacks elementary functionality at this stage, and is not ready for production, the API is not stable and may change as they are developing.


## Installation

```bash
brew install kedo
```

## Usage

```javascript
// myscript.js
const response = await Kedo.fetch("https://jsonplaceholder.typicode.com/todos/1")

console.log(response.title);

Kedo.writeFileSync("todos.json", JSON.stringify(response));

const content = Kedo.readFileSync('./todos.json');

console.log(content);

setTimeout(() => {
    console.log('Hello from KedoJS!');
}, 1000);
```

```bash
kedo run myscript.js
```

## TODO

Roadmap to v0.1.0

- [ ] File System
    - [x] readFile
    - [x] readFileSync
    - [x] writeFile
    - [x] writeFileSync
    - [x] readDir
    - [x] readDirSync
    - [ ] stat
    - [ ] statSync
    - [ ] unlink
    - [ ] unlinkSync
    - [ ] mkdir
    - [ ] mkdirSync
- [x] Console API
- [ ] HTTP Server
    - [ ] serve
- [ ] HTTP Client
    - [ ] Fetch API (WIP)
- [ ] Child Process
- [ ] OS
- [x] Timers
    - [x] setTimeout
    - [x] setInterval
    - [x] clearInterval
    - [x] clearTimeout
- [x] ES Modules
- [ ] REPL
- [x] URL
- [ ] Buffer
- [ ] Errors
- [ ] Crypto
- [ ] Process
- [x] Query String
- [ ] Events
- [ ] Streams
    - [x] Readable
    - [ ] Writable
    - [ ] Duplex
    - [ ] Transform
