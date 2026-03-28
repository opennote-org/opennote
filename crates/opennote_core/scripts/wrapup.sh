#!/usr/bin/env sh

main() {
    # Run this script before PR.
    # It will help you reformat your project to meet the CI requirements
    echo "Formatting the project..."
    cargo fmt --all

    echo "Running clippy lint..."
    cargo clippy --all-targets --all-features -- -A warnings

    echo "Checking compilations..."
    cargo check --all --all-features

    echo "Running unit-tests..."
    cargo test --all --all-features
}

main "$@"
