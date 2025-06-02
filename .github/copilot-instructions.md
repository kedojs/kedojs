# KedoJS Guidelines

## 1. Project Understanding

- The root has multiple Cargo workspaces:
  • `cli/` → user-facing CLI
  • `kedo_js/` → Core standard library (std) for Kedo JavaScript runtime
  • `bundler/` → TypeScript compiler & bundler
  • `packages/kedo_std/` → Node/Bun‑like std APIs
  • `packages/kedo_web/` → Web‑standard APIs (fetch, Request, Response)
  • `packages/kedo_runtime/` → Runtime APIs (kedo runtime, classes setup, etc.)
  • `packages/kedo_fs/` → Filesystem APIs (fs, path, etc.)
  • `packages/kedo_core/` → Core APIs (jobs, state, etc.)
- Tests under `tests/` validate web specs, events, streams, filesystem, performance.

## 2. Suggestion Principles

- Reference existing types and functions before introducing new ones.
  E.g., use [`FetchRequestBuilder`](../packages/kedo_std/http/request.rs#L132) for constructing requests.
- Follow builder patterns already in place (method chaining, `build()` returning `Result`).
- Keep error handling Rust‑idiomatic: avoid panics, prefer `Result<T, E>`.
- Leverage existing stream abstractions (`IncomingBodyStream`, `StreamDecoder`).

## 3. Coding Conventions

- Rust code:
  • Use `snake_case` for functions and variables.
  • Use `PascalCase` for structs and enums.
  • Adhere to `rustfmt.toml` rules.
- JavaScript code:
  • Follow Web IDL naming: `fetch`, `Headers`, `URLSearchParams`.
  • Tests use `@kedo/assert`—import consistently.
  • Async functions should return Promises.

## 4. Best Practices

- **Modularity**: Split large modules (e.g. HTTP) into submodules (`request.rs`, `response.rs`).
- **Error Handling**: Wrap fallible operations in `Result`; provide clear error messages.
- **Testing**: Add tests in `tests/web/` for any new Web API feature.
- **Documentation**: Update `README.md` and inline `///` docs for public APIs.
- **Performance**: Use `hyperfine` scripts in `scripts/performance.sh` to benchmark changes.
- **Type Safety**: In Rust, prefer `&str`/`String` conversions via `.to_string()` only at boundaries.
- **Resource Cleanup**: Ensure any spawned timers or streams are properly closed/canceled.

## 5. Rust-JS Interop

- **Creating JS Classes**: Use the `#[js_class(resource = YourRustStruct)]` macro from `kedo_macros` on an empty Rust struct (e.g., `FetchRequestResource`) to expose `YourRustStruct` (e.g., `FetchRequest`) as a JS class. Instances are typically created via `ClassTable::get("ClassName").unwrap().object::<YourRustStruct>(ctx, Some(Box::new(rust_data)))`.
- **Defining Exports**: Use the `define_exports!(@function[fn1, fn2])` macro from `kedo_core::modules` within an empty struct (e.g., `FetchRequestOps`) to generate an `export` function. This function makes the listed Rust functions (`fn1`, `fn2`) available as properties on the JS module's `exports` object.
- **Managing Rust References in JS**:
  - When a JS object holds a pointer to a Rust struct (set via `JSObject::set_private_data`), use `kedo_utils::utils::downcast_ref::<YourRustStruct>(&js_object)` to safely get a `ManuallyDropClone<Box<YourRustStruct>>` reference back in Rust callbacks.
  - Use `kedo_utils::utils::upcast(Box::new(rust_data))` to get a raw pointer for `set_private_data`.
  - Use `kedo_utils::utils::drop_ptr::<YourRustStruct>(raw_ptr)` in the JS class's `finalize` callback to correctly free the Rust memory.

## Scripts

- Build the kedo_js standard library `make bundle-std`
  - Use this command for any changes made in the `kedo_js` in order to take effect
- Run test files e.g:

  - `make run file=./tests/streams/readable.js`
  - `make run file=./tests/assert/assert.test.js`

- Build rust cli in release mode `make release`
- Run javascript modules in release mode:
  - `./target/release/kedo run ./tests/web/fetch.js`
  - `./target/release/kedo run ./tests/http/server.js`
