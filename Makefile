# Run: make [target] [file=filename]
# How to run test for a module: make test-module module=text_encoding::tests

run:
	(RUST_BACKTRACE=1 cargo run --manifest-path ./cli/Cargo.toml -- run $(file))

bundle:
	(cargo run --manifest-path ./cli/Cargo.toml -- bundle --output=$(output) --entry=$(entry) --minify)

bundle-std:
	(cargo run --manifest-path ./kedo-std/Cargo.toml -- bundle --output=src/@std/dist --minify)

build:
	(cargo build --manifest-path ./cli/Cargo.toml)

release:
	(cargo build --release --manifest-path ./cli/Cargo.toml)

build-lib:
	(cargo build)

lib-release:
	(cargo build --release)

test:
	(cargo test)

test-module:
	(cargo test --package kedo_core --lib -- modules::$(module) --show-output)

.PHONY: run build release lib lib-release test test-module build-lib bundle bundle-std
