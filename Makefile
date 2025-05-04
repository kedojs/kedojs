# Run: make [target] [file=filename]
# How to run test for a module: make test-module module=text_encoding::tests

run:
	(RUST_BACKTRACE=1 cargo run --manifest-path ./cli/Cargo.toml -- run $(file))

flamegraph:
	(RUSTFLAGS='-Cforce-frame-pointers=yes' cargo flamegraph --root --bin kedo -- run $(file))

bundle:
	(cargo run --manifest-path ./cli/Cargo.toml -- bundle --output=$(output) --entry=$(entry) --minify)

bundle-std:
	(cargo run --manifest-path ./kedo_js/Cargo.toml -- bundle --output=build/@std/dist --minify)

build:
	(cargo build --manifest-path ./cli/Cargo.toml)

release:
	(RUSTFLAGS='-Cforce-frame-pointers=yes' cargo build --release --manifest-path ./cli/Cargo.toml)

build-lib:
	(cargo build)

lib-release:
	(cargo build --release)

test:
	(cargo test)

test-module:
	(cargo test --package kedo_core --lib -- modules::$(module) --show-output)

## Example: make bench-server req=1000 conc=10 port=8080
bench-server:
	@for i in $$(seq 1 $(times)); do \
		echo "Running benchmark test $$i of $(times)"; \
		ab -n $(req) -c $(conc) -T 'application/json' 'http://0.0.0.0:$(port)/'; \
	done

## run javascript file
bench-script:
	@for i in $$(seq 1 $(times)); do \
		echo "Running benchmark test $$i of $(times)"; \
		(RUST_BACKTRACE=1 cargo run --manifest-path ./cli/Cargo.toml -- run $(file)); \
	done
	

.PHONY: run build release lib lib-release test test-module build-lib bundle bundle-std
