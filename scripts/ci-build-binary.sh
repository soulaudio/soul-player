#!/bin/bash
# CI Build Binary - Build Rust binary only
# This emulates the CI binary build phase

set -e

PLATFORM="${1:-linux}"

echo "==================================="
echo "CI Build Binary - $PLATFORM"
echo "==================================="

# Ensure we're in the project root
cd "$(dirname "$0")/.."

echo ""
echo "→ Installing dependencies..."
yarn install --immutable

echo ""
echo "→ Building Rust binary..."
cd applications/desktop/src-tauri

case $PLATFORM in
    linux)
        cargo build --release --target x86_64-unknown-linux-gnu
        ;;
    windows)
        cargo build --release --target x86_64-pc-windows-gnu
        ;;
    *)
        echo "❌ Unknown platform: $PLATFORM"
        exit 1
        ;;
esac

echo ""
echo "✅ Binary build completed!"
ls -lh target/*/release/soul-player* || true
