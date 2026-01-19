#!/usr/bin/env bash
# Build Flatpak for Soul Player
# This script creates a Flatpak package for universal Linux distribution

set -e

VERSION="${1:-0.1.1}"
APP_ID="io.github.soulaudio.SoulPlayer"

echo "Building Flatpak for Soul Player v${VERSION}..."

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if flatpak-builder is installed
if ! command -v flatpak-builder &> /dev/null; then
    echo "Error: flatpak-builder not found"
    echo "Install with: sudo apt-get install flatpak-builder"
    echo "or: sudo dnf install flatpak-builder"
    exit 1
fi

# Check if cargo-sources.json exists, if not, generate it
if [ ! -f "$SCRIPT_DIR/cargo-sources.json" ]; then
    echo ""
    echo "⚠️  cargo-sources.json not found. Generating vendored dependencies..."
    echo "   This is required for Flatpak builds (network sandbox restriction)"
    echo ""

    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        echo "Error: cargo not found. Please install Rust toolchain."
        echo "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    # Check if Python 3 is installed
    if ! command -v python3 &> /dev/null; then
        echo "Error: python3 not found. Required for flatpak-cargo-generator.py"
        exit 1
    fi

    # Run vendor script
    chmod +x "$SCRIPT_DIR/vendor-cargo.sh"
    "$SCRIPT_DIR/vendor-cargo.sh"

    echo ""
    echo "✅ Vendored dependencies generated"
    echo ""
else
    echo "✅ Using existing cargo-sources.json"
fi

# Install required runtimes and SDKs
echo "Checking Flatpak runtimes..."
if ! flatpak list --runtime | grep -q "org.gnome.Platform/x86_64/45"; then
    echo "Installing GNOME runtime..."
    flatpak install -y flathub org.gnome.Platform//45 org.gnome.Sdk//45
fi

# Install SDK extensions
echo "Checking SDK extensions..."
flatpak install -y flathub org.freedesktop.Sdk.Extension.rust-stable//23.08 || true
flatpak install -y flathub org.freedesktop.Sdk.Extension.node20//23.08 || true

# Build Flatpak
echo "Building Flatpak..."
flatpak-builder \
    --force-clean \
    --repo=repo \
    build-dir \
    "$SCRIPT_DIR/${APP_ID}.yml"

# Export to single-file bundle for distribution
echo "Creating Flatpak bundle..."
flatpak build-bundle \
    repo \
    ${APP_ID}_${VERSION}_x86_64.flatpak \
    ${APP_ID}

if [ -f "${APP_ID}_${VERSION}_x86_64.flatpak" ]; then
    echo "✅ Flatpak created: ${APP_ID}_${VERSION}_x86_64.flatpak"

    # Calculate SHA256
    sha256sum "${APP_ID}_${VERSION}_x86_64.flatpak" > "${APP_ID}_${VERSION}_x86_64.flatpak.sha256"
    echo "✅ Checksum created: ${APP_ID}_${VERSION}_x86_64.flatpak.sha256"
else
    echo "❌ Error: Flatpak was not created"
    exit 1
fi

# Optional: Install locally for testing
read -p "Install locally for testing? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    flatpak-builder --user --install --force-clean build-dir "$SCRIPT_DIR/${APP_ID}.yml"
    echo "✅ Installed locally. Run with: flatpak run ${APP_ID}"
fi

echo "Done!"
