# Run: make [target] [file=filename]
run:
	(cargo run --manifest-path ./cli/Cargo.toml -- run $(file))

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
