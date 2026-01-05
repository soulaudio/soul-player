#!/bin/bash
# Safe build script for Soul Player
# Runs dependency checks before building

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if we should skip dependency checks
SKIP_CHECKS="${SKIP_CHECKS:-false}"

if [ "$SKIP_CHECKS" != "true" ]; then
    echo -e "${YELLOW}Running pre-build checks...${NC}"
    echo ""
    "$SCRIPT_DIR/pre-build.sh" || exit 1
    echo ""
fi

echo -e "${GREEN}Starting build...${NC}"
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Parse arguments
BUILD_TYPE="${1:-debug}"
TARGET_CRATE="${2:-workspace}"

case "$BUILD_TYPE" in
    debug)
        if [ "$TARGET_CRATE" = "workspace" ]; then
            echo "Building workspace (debug)..."
            cargo build --workspace
        else
            echo "Building $TARGET_CRATE (debug)..."
            cargo build -p "$TARGET_CRATE"
        fi
        ;;
    release)
        if [ "$TARGET_CRATE" = "workspace" ]; then
            echo "Building workspace (release)..."
            cargo build --workspace --release
        else
            echo "Building $TARGET_CRATE (release)..."
            cargo build -p "$TARGET_CRATE" --release
        fi
        ;;
    *)
        echo "Usage: $0 [debug|release] [crate-name|workspace]"
        echo ""
        echo "Examples:"
        echo "  $0                              # Build workspace (debug)"
        echo "  $0 release                      # Build workspace (release)"
        echo "  $0 debug soul-audio-desktop     # Build specific crate (debug)"
        echo "  $0 release soul-audio-desktop   # Build specific crate (release)"
        echo ""
        echo "Environment variables:"
        echo "  SKIP_CHECKS=true                # Skip dependency checks"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✓ Build completed successfully!${NC}"

# Show binary location for release builds
if [ "$BUILD_TYPE" = "release" ]; then
    echo ""
    echo "Release binaries are in: target/release/"
fi
