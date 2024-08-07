hyperfine "deno run ./tests/performance/console.js" "bun ./tests/performance/console.js" "node ./tests/performance/console.js" "./target/release/kedo run ./tests/performance/console.js" --warmup=10

hyperfine "deno run ./tests/performance/filesystem.js" "bun ./tests/performance/filesystem.js" "node ./tests/performance/filesystem.js" "./target/release/kedo run ./tests/performance/filesystem.js" --warmup=10

hyperfine "deno run --allow-read --allow-write ./tests/performance/filesystem/read-file-deno.js" "bun ./tests/performance/filesystem/read-file-node.js" "node ./tests/performance/filesystem/read-file-node.js" "./target/release/kedo run ./tests/performance/filesystem/read-file-kedo.js" --warmup=10

hyperfine "deno run ./tests/performance/console.js" "bun ./tests/performance/console.js" "node ./tests/performance/console.js" "./target/release/kedo run ./tests/performance/console.js" --warmup=10

hyperfine "deno run ./tests/performance/filesystem.js" "bun ./tests/performance/filesystem.js" "node ./tests/performance/filesystem.js" "./target/release/kedo run ./tests/performance/filesystem.js" --warmup=10

hyperfine "deno run --allow-read --allow-write ./tests/performance/filesystem/read-file-deno-async.js" "bun ./tests/performance/filesystem/read-file-node-async.mjs" "node ./tests/performance/filesystem/read-file-node-async.mjs" "./target/release/kedo run ./tests/performance/filesystem/read-file-kedo-async.js" --warmup=10

hyperfine "deno run ./tests/module/index.js" "bun ./tests/module/index.mjs" "node ./tests/module/index.mjs" "./target/release/kedo run ./tests/module/index.js" --warmup=10

hyperfine "deno run ./tests/performance/headers.js" "bun ./tests/performance/headers.js" "node ./tests/performance/headers.js" "./target/release/kedo run ./tests/performance/headers.js" --warmup=10

hyperfine "deno run ./tests/headers.js" "bun ./tests/headers.js" "node ./tests/headers.js" "./target/release/kedo run ./tests/headers.js" --warmup=10

hyperfine "deno run ./tests/streams/node-readable.js" "bun ./tests/streams/node-readable.js" "./target/release/kedo run ./tests/streams/readable.js" --warmup=10
