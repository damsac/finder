# Finder project justfile
ffi_crate := "finder-ffi"

# === Core ===

test:
    cargo test --workspace

build:
    cargo build --workspace

check:
    cargo check --workspace

clippy:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all -- --check

fmt-fix:
    cargo fmt --all

# === iOS ===

ios-build:
    ./scripts/ios-build --crate-name {{ffi_crate}}

ios-build-release:
    ./scripts/ios-build --crate-name {{ffi_crate}} --release

# === CI / QA ===

pre-commit: fmt clippy

pre-merge: fmt clippy test
