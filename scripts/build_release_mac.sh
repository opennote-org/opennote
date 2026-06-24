#!/usr/bin/env sh

main() {
    set -e

    echo "cd into opennote-ui..."
    cd ./crates/opennote-desktop

    echo "Building desktop binary..."
    cargo build --release

    echo "Bundling app..."
    cd ../../
    uv run ./appify/__main__.py \
        --executable ./target/release/opennote-desktop \
        --bundle ./target/release/opennote.app \
        --icon-png ./assets/logo.png

    echo "Packaging DMG..."
    DMG_NAME="OpenNote.dmg"
    DMG_PATH="./target/release/${DMG_NAME}"

    # Remove old DMG if it exists
    rm -f "$DMG_PATH"

    # Create a compressed, read-only DMG from the .app
    hdiutil create \
        -volname "OpenNote" \
        -srcfolder ./target/release/opennote.app \
        -ov \
        -format UDZO \
        "$DMG_PATH"

    echo "Done! DMG created at ${DMG_PATH}"
}

main "$@"
