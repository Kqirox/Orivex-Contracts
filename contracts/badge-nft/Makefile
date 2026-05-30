default: build

all: test

test: build
	cargo test --features contract,testutils

build:
	stellar contract build --features contract
	@ls -l ../target/wasm32v1-none/release/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
