## Console
```bash
hyperfine "deno run --allow-read --allow-write ./examples/console.js" "bun ./examples/console.js" "node ./examples/console.js" "./target/release/kedo run ./examples/console.js" --warmup=10
```

## Performance
```bash
hyperfine "deno run --allow-read --allow-write ./examples/performance.js" "bun ./examples/performance.js" "node ./examples/performance.js" "./target/release/kedo run ./examples/performance.js" --warmup=10
```

## Read file
```bash
hyperfine "deno run --allow-read --allow-write ./local/examples/fs/read-file-deno.js" "./target/release/kedo run ./local/examples/fs/read-file-kedo.js" "bun ./local/examples/fs/read-file-node.js" "node ./local/examples/fs/read-file-node.js"  --warmup=10
```
