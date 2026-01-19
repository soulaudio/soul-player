#!/bin/bash
# Update PKGBUILD-bin with new version and SHA256 checksum
# Usage: ./update-pkgbuild.sh <version> <sha256sum>
# Example: ./update-pkgbuild.sh 0.1.2 abc123...

set -euo pipefail

VERSION="${1}"
SHA256="${2}"

if [ -z "$VERSION" ] || [ -z "$SHA256" ]; then
    echo "Usage: $0 <version> <sha256sum>"
    echo "Example: $0 0.1.2 abc123def456..."
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKGBUILD_FILE="${SCRIPT_DIR}/PKGBUILD-bin"

if [ ! -f "$PKGBUILD_FILE" ]; then
    echo "Error: PKGBUILD-bin not found at $PKGBUILD_FILE"
    exit 1
fi

echo "Updating PKGBUILD-bin to version ${VERSION}..."

# Update pkgver
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" "$PKGBUILD_FILE"

# Reset pkgrel to 1 for new version
sed -i "s/^pkgrel=.*/pkgrel=1/" "$PKGBUILD_FILE"

# Update sha256sums
sed -i "s/^sha256sums=.*/sha256sums=('${SHA256}')/" "$PKGBUILD_FILE"

echo "✅ PKGBUILD-bin updated successfully"
echo "   Version: ${VERSION}"
echo "   SHA256: ${SHA256:0:16}..."
echo ""
echo "Updated file: $PKGBUILD_FILE"
