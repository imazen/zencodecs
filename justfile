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

# Feature permutation checks
feature-check:
    cargo test --no-default-features --features std
    cargo test --no-default-features --features "jpeg,png"
    cargo test --no-default-features --features "jpeg,webp,gif,png"

# Cross-compilation targets (use --no-default-features --features png to avoid path deps)
test-i686:
    cross test --no-default-features --features png --target i686-unknown-linux-gnu

test-armv7:
    cross test --no-default-features --features png --target armv7-unknown-linux-gnueabihf

test-cross: test-i686 test-armv7

# zcimg CLI
zcimg-build:
    cargo build --release --manifest-path zcimg/Cargo.toml

zcimg-run *ARGS:
    cargo run --release --manifest-path zcimg/Cargo.toml -- {{ARGS}}

# ═══════════════════════════════════════════════════════════
# Fuzzing
# ═══════════════════════════════════════════════════════════

# Seed the fuzz corpus from local sibling crates + external GitHub repos.
# Pass --local-only to skip external downloads.
fuzz-seed *ARGS:
    ./fuzz/seed_corpus.sh {{ARGS}}

# Seed corpus (local only, no network).
fuzz-seed-local:
    ./fuzz/seed_corpus.sh --local-only

# Build all fuzz targets (release mode with debug info).
fuzz-build:
    cd fuzz && cargo +nightly fuzz build

# List all available fuzz targets.
fuzz-list:
    cd fuzz && cargo +nightly fuzz list

# Run a specific fuzz target. Seeds corpus first (local only).
# Usage: just fuzz <target> [extra-libfuzzer-args]
# Example: just fuzz fuzz_decode -- -max_total_time=60
fuzz TARGET *ARGS:
    ./fuzz/seed_corpus.sh --local-only
    cd fuzz && cargo +nightly fuzz run {{TARGET}} corpus/seed/mixed -- -dict=multiformat.dict {{ARGS}}

# Run all fuzz targets for 60 seconds each (quick smoke test).
fuzz-smoke:
    ./fuzz/seed_corpus.sh --local-only
    #!/usr/bin/env bash
    set -euo pipefail
    cd fuzz
    for target in $(cargo fuzz list 2>/dev/null); do
        echo "=== $target (60s) ==="
        cargo fuzz run "$target" corpus/seed/mixed -- -dict=multiformat.dict -max_total_time=60 || true
    done

# Run all fuzz targets for 30 minutes each (deep fuzzing).
fuzz-deep:
    ./fuzz/seed_corpus.sh
    #!/usr/bin/env bash
    set -euo pipefail
    cd fuzz
    for target in $(cargo fuzz list 2>/dev/null); do
        echo "=== $target (30min) ==="
        cargo fuzz run "$target" corpus/seed/mixed -- -dict=multiformat.dict -max_total_time=1800 || true
    done

# Run a fuzz target with coverage instrumentation and generate a report.
# Usage: just fuzz-cov <target>
fuzz-cov TARGET:
    cd fuzz && cargo +nightly fuzz coverage {{TARGET}} corpus/seed/mixed
    @echo "Coverage data written to fuzz/coverage/{{TARGET}}/"

# Run only high-priority fuzz targets (probe, decode, exif, limits) for CI.
# Pass --local-only to fuzz-seed to avoid network in CI.
fuzz-ci DURATION="60":
    ./fuzz/seed_corpus.sh --local-only
    #!/usr/bin/env bash
    set -euo pipefail
    cd fuzz
    for target in fuzz_probe fuzz_decode fuzz_exif fuzz_decode_limits; do
        echo "=== $target ({{DURATION}}s) ==="
        cargo fuzz run "$target" corpus/seed/mixed -- -dict=multiformat.dict -max_total_time={{DURATION}}
    done

# Clean fuzz artifacts and coverage data (preserves corpus).
fuzz-clean:
    rm -rf fuzz/target fuzz/artifacts fuzz/coverage
