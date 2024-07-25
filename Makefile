# Run: make [target] [file=filename]
run:
	(cargo run --manifest-path ./cli/Cargo.toml -- run $(file))

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

release-lib:
	(cargo build --release)

test:
	(cargo test)

.PHONY: run build release lib lib-release
