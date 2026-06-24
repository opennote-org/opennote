#!/usr/bin/env bash
set -e

echo "cd into opennote-ui..."
cd ./crates/opennote-desktop

echo "Building desktop binary..."
cargo build --release

cd ../..

# Package directory
PACKAGE_DIR="./target/release/linux-package"
mkdir -p "$PACKAGE_DIR"

# Copy the binary
cp ./target/release/opennote-desktop "$PACKAGE_DIR/"

echo "Creating tar.gz archive..."
ARCHIVE_NAME="opennote-linux-x86_64.tar.gz"
tar -czf "./target/${ARCHIVE_NAME}" -C ./target/release linux-package

echo "Archive created at ./target/${ARCHIVE_NAME}"