#!/usr/bin/env bash
#
# Version Bumping Script for Soul Player (Wrapper)
#
# Usage: ./scripts/bump-version.sh <version>
# Example: ./scripts/bump-version.sh 0.1.0
#
# This script is a simple wrapper around bump-version.mjs (Node.js)
# for better cross-platform compatibility.
#

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if Node.js is available
if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js is required but not found"
    echo "Please install Node.js 20+ and try again"
    exit 1
fi

# Run the Node.js version bumper
exec node "$SCRIPT_DIR/bump-version.mjs" "$@"
