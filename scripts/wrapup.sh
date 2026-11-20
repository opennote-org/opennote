#!/usr/bin/env sh

main() {
    set -e

    # Mirror the Rust PR checks in .github/workflows/rust-check.yml.
    echo "Checking formatting..."
    cargo fmt --all -- --check

    echo "Checking workspace libraries and binaries..."
    cargo check --workspace --lib --bins --locked

    echo "Running Clippy..."
    cargo clippy --workspace --lib --bins --locked -- -D warnings
}

main "$@"
