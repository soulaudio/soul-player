#!/usr/bin/env bash
# Vendor Cargo dependencies for Flatpak build
# This script generates a cargo-sources.json file using the official flatpak-cargo-generator
# Required by Flatpak builds since they run in a network sandbox
#
# Reference: https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=============================================="
echo "Flatpak Cargo Dependency Vendoring"
echo "=============================================="
echo ""
echo "This script generates cargo-sources.json for offline Flatpak builds"
echo "Reference: https://github.com/flatpak/flatpak-builder-tools"
echo ""

cd "$PROJECT_ROOT"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found. Please install Rust toolchain."
    echo "   Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Error: python3 not found. Required for flatpak-cargo-generator.py"
    exit 1
fi

# Install Python dependencies for flatpak-cargo-generator
echo "Installing Python dependencies (tomlkit, aiohttp)..."
pip3 install --user tomlkit aiohttp 2>&1 | grep -v "Requirement already satisfied" || true
echo ""

# Check if flatpak-cargo-generator.py exists
GENERATOR="$SCRIPT_DIR/flatpak-cargo-generator.py"
if [ ! -f "$GENERATOR" ]; then
    echo "❌ Error: flatpak-cargo-generator.py not found"
    echo "   Expected location: $GENERATOR"
    echo "   This file should be in the repository. If missing, download from:"
    echo "   https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"
    exit 1
fi

# Check if Cargo.lock exists
if [ ! -f "Cargo.lock" ]; then
    echo "⚠️  Cargo.lock not found. Generating..."
    cargo generate-lockfile
fi

echo "Step 1: Generating cargo-sources.json"
echo "----------------------------------------"
echo "This file contains all Cargo dependencies in Flatpak format"
echo ""

# Generate cargo-sources.json using the official tool
# This tool reads Cargo.lock and generates a Flatpak manifest with all dependencies
python3 "$GENERATOR" Cargo.lock -o "$SCRIPT_DIR/cargo-sources.json"

if [ ! -f "$SCRIPT_DIR/cargo-sources.json" ]; then
    echo "❌ Error: Failed to generate cargo-sources.json"
    exit 1
fi

# Count number of sources generated
SOURCE_COUNT=$(python3 -c "import json; print(len(json.load(open('$SCRIPT_DIR/cargo-sources.json'))))" 2>/dev/null || echo "unknown")
echo "✅ Generated cargo-sources.json with $SOURCE_COUNT dependencies"
echo ""

echo "Step 2: Creating cargo config for vendored dependencies"
echo "--------------------------------------------------------"

# Create cargo-config.toml for offline builds
# This tells cargo to use the vendored dependencies instead of crates.io
cat > "$SCRIPT_DIR/cargo-config.toml" << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "cargo/vendor"
EOF

echo "✅ Created cargo-config.toml"
echo ""

# Calculate size of cargo-sources.json
SOURCES_SIZE=$(du -h "$SCRIPT_DIR/cargo-sources.json" | cut -f1)

echo "=============================================="
echo "✅ Cargo vendoring complete!"
echo "=============================================="
echo ""
echo "Generated files:"
echo "  ✓ .flatpak/cargo-sources.json       ($SOURCES_SIZE, $SOURCE_COUNT dependencies)"
echo "  ✓ .flatpak/cargo-config.toml        (cargo offline config)"
echo ""
echo "How it works:"
echo "  1. cargo-sources.json contains all crate tarballs as Flatpak sources"
echo "  2. Flatpak downloads all sources before the build (no network during build)"
echo "  3. cargo-config.toml tells cargo to use vendored dependencies"
echo "  4. Build runs with 'cargo build --offline' flag"
echo ""
echo "Next steps:"
echo "  1. Commit cargo-sources.json to git (it's tracked in the repository)"
echo "  2. Build Flatpak with: .flatpak/build-flatpak.sh"
echo "  3. The Flatpak manifest will automatically include these sources"
echo ""
echo "References:"
echo "  - https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo"
echo "  - https://belmoussaoui.com/blog/8-how-to-flatpak-a-rust-application/"
echo ""
