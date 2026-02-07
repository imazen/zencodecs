# zencodecs justfile

check: fmt clippy test

fmt:
    cargo fmt

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-features

build:
    cargo build --release --all-features

doc:
    cargo doc --all-features --no-deps --open

# Verify no_std compiles
check-no-std:
    cargo build --no-default-features --target wasm32-unknown-unknown

outdated:
    cargo outdated

# Cross-compilation targets (use --no-default-features --features png to avoid path deps)
test-i686:
    cross test --no-default-features --features png --target i686-unknown-linux-gnu

test-armv7:
    cross test --no-default-features --features png --target armv7-unknown-linux-gnueabihf

test-cross: test-i686 test-armv7
