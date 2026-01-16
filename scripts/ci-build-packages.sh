#!/bin/bash
# CI Build Packages - Build full installers/packages
# This emulates the CI release build phase

set -e

PLATFORM="${1:-linux}"

echo "==================================="
echo "CI Build Packages - $PLATFORM"
echo "==================================="

# Ensure we're in the project root
cd "$(dirname "$0")/.."

echo ""
echo "→ Installing dependencies..."
yarn install --immutable

echo ""
echo "→ Building packages..."

case $PLATFORM in
    linux)
        echo "Building Linux packages (DEB, RPM, AppImage)..."
        yarn build:desktop --bundles deb,rpm,appimage
        echo ""
        echo "✅ Packages created:"
        find applications/desktop/src-tauri/target/release/bundle -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \) -exec ls -lh {} \;
        ;;
    windows)
        echo "⚠️  Windows packaging requires Windows or Tauri CLI 2.0+ with cross-compilation"
        echo "Building Windows binary only..."
        cd applications/desktop/src-tauri
        cargo build --release --target x86_64-pc-windows-gnu
        echo ""
        echo "✅ Binary created:"
        ls -lh target/x86_64-pc-windows-gnu/release/soul-player.exe || true
        ;;
    *)
        echo "❌ Unknown platform: $PLATFORM"
        exit 1
        ;;
esac

echo ""
echo "✅ Build completed!"
