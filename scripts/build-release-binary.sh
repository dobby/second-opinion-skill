#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"

echo "Building release binary..."
cd "$PROJECT_DIR"
cargo build --release

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
BINARY_NAME="second-opinion-${OS}-${ARCH}"

mkdir -p "$BIN_DIR"
cp "target/release/second-opinion" "$BIN_DIR/$BINARY_NAME"
chmod +x "$BIN_DIR/$BINARY_NAME"

echo "Built: $BIN_DIR/$BINARY_NAME"
