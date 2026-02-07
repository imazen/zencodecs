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
