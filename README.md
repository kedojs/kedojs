# KedoJS

![KedoJS](./docs/logo-sm.jpg)

Kedojs is a fast Javascript runtime.

Kedojs is in an experimental stage, lacks elementary functionality at this stage, and is not ready for production, the API is not stable and may change as they are developing.


## Installation

```bash
brew install kedo
```

## Usage

```javascript
// myscript.js
const content = Kedo.readFileSync('./examples/data.txt');
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
- [ ] HTTP Client
- [ ] Child Process
- [ ] OS
- [x] Timers
    - [x] setTimeout
    - [x] setInterval
    - [x] clearInterval
    - [x] clearTimeout
- [ ] ES Modules
- [ ] Fetch API
- [ ] REPL
- [ ] URL
- [ ] Buffer
- [ ] Errors
- [ ] Crypto
- [ ] Process
- [ ] Query String
- [ ] Events
- [ ] Streams
    - [ ] Readable
    - [ ] Writable
    - [ ] Duplex
    - [ ] Transform

