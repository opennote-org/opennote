#!/usr/bin/env sh

main() {
    # For now, it only works on macOS
    set -e

    echo "cd into opennote-ui..."
    cd ./crates/opennote-ui

    echo "Building desktop binary..."
    cargo build --release

    echo "Bundling app..."
    cd ../../
    uv run ./appify/__main__.py --executable ./target/release/opennote-ui --bundle ./target/release/opennote.app --icon-png ./assets/logo.png
}

main "$@"
