#!/usr/bin/env bash
# Vendor Cargo dependencies for Flatpak build
# This script generates a cargo-sources.json file that includes all vendored dependencies
# Required by Flatpak builds since they run in a network sandbox

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Vendoring Cargo dependencies for Flatpak..."
cd "$PROJECT_ROOT"

# Check if cargo vendor exists
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust toolchain."
    exit 1
fi

# Clean any existing vendor directory
if [ -d ".flatpak/vendor" ]; then
    echo "Cleaning existing vendor directory..."
    rm -rf .flatpak/vendor
fi

# Create vendor directory
mkdir -p .flatpak/vendor

# Vendor all dependencies
echo "Running cargo vendor..."
cargo vendor .flatpak/vendor > .flatpak/cargo-config.toml

echo "✅ Cargo dependencies vendored to .flatpak/vendor/"

# Now generate cargo-sources.json for Flatpak
echo "Generating cargo-sources.json for Flatpak manifest..."

# Create a Python script to generate the sources JSON
cat > .flatpak/generate-cargo-sources.py << 'PYTHON_SCRIPT'
#!/usr/bin/env python3
"""
Generate cargo-sources.json for Flatpak manifest
This converts vendored cargo dependencies into Flatpak source format
"""

import json
import os
import hashlib
from pathlib import Path

def sha256_file(filepath):
    """Calculate SHA256 hash of a file"""
    h = hashlib.sha256()
    with open(filepath, 'rb') as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def generate_cargo_sources(vendor_dir):
    """Generate Flatpak sources from vendored cargo directory"""
    sources = []

    vendor_path = Path(vendor_dir)
    if not vendor_path.exists():
        print(f"Error: Vendor directory not found: {vendor_dir}")
        return sources

    # Each subdirectory in vendor/ is a crate
    crate_dirs = sorted([d for d in vendor_path.iterdir() if d.is_dir()])

    print(f"Found {len(crate_dirs)} vendored crates")

    for crate_dir in crate_dirs:
        crate_name = crate_dir.name

        # Read .cargo-checksum.json to get the package info
        checksum_file = crate_dir / '.cargo-checksum.json'
        if not checksum_file.exists():
            print(f"Warning: No checksum file for {crate_name}, skipping")
            continue

        with open(checksum_file, 'r') as f:
            checksum_data = json.load(f)

        # Get package name and version from directory name
        # Format is typically: crate-name-version
        parts = crate_name.rsplit('-', 1)
        if len(parts) != 2:
            print(f"Warning: Cannot parse crate name/version from {crate_name}, skipping")
            continue

        pkg_name, pkg_version = parts

        # Create a source entry for this vendored crate
        source = {
            "type": "inline",
            "contents": json.dumps(checksum_data),
            "dest": f"cargo/vendor/{crate_name}/.cargo-checksum.json"
        }
        sources.append(source)

        # Add all files in the crate directory
        for file_path in crate_dir.rglob('*'):
            if file_path.is_file() and file_path.name != '.cargo-checksum.json':
                rel_path = file_path.relative_to(vendor_path)

                # For small files, inline them. For larger files, reference them.
                file_size = file_path.stat().st_size

                if file_size < 100000:  # 100KB threshold
                    # Inline small files
                    try:
                        with open(file_path, 'r', encoding='utf-8') as f:
                            content = f.read()
                        sources.append({
                            "type": "inline",
                            "contents": content,
                            "dest": f"cargo/vendor/{rel_path}"
                        })
                    except UnicodeDecodeError:
                        # Binary file, skip inlining
                        pass

    return sources

def main():
    vendor_dir = Path(__file__).parent / 'vendor'
    output_file = Path(__file__).parent / 'cargo-sources.json'

    print(f"Generating Flatpak cargo sources from {vendor_dir}")

    sources = generate_cargo_sources(vendor_dir)

    if not sources:
        print("Warning: No sources generated")
        # Create empty sources file so build doesn't fail
        sources = []

    # Write sources JSON
    with open(output_file, 'w') as f:
        json.dump(sources, f, indent=2)

    print(f"✅ Generated {output_file} with {len(sources)} source entries")

if __name__ == '__main__':
    main()
PYTHON_SCRIPT

chmod +x .flatpak/generate-cargo-sources.py

# Run the Python script
python3 .flatpak/generate-cargo-sources.py

# Alternative approach: Create a simple tarball of vendored dependencies
# This is more efficient than inlining thousands of files
echo "Creating vendor tarball for Flatpak..."
cd .flatpak
tar -czf vendor.tar.gz vendor/
cd ..

# Calculate checksum
VENDOR_SHA256=$(sha256sum .flatpak/vendor.tar.gz | cut -d' ' -f1)

echo ""
echo "✅ Cargo vendoring complete!"
echo ""
echo "Generated files:"
echo "  - .flatpak/vendor/              (vendored dependencies)"
echo "  - .flatpak/vendor.tar.gz        (tarball for Flatpak)"
echo "  - .flatpak/cargo-config.toml    (cargo config for vendored deps)"
echo ""
echo "Vendor tarball SHA256: $VENDOR_SHA256"
echo ""
echo "Next steps:"
echo "  1. Commit vendor.tar.gz to git (or upload to release)"
echo "  2. Update Flatpak manifest to include vendor.tar.gz source"
echo "  3. Build Flatpak with: .flatpak/build-flatpak.sh"
echo ""
