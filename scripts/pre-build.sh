#!/bin/bash
# Pre-build script for Soul Player
# This script should be run before building to ensure all dependencies are satisfied

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=================================================="
echo "Soul Player - Pre-Build Check"
echo "=================================================="
echo ""

# Run dependency checker
if [ -x "$SCRIPT_DIR/check-dependencies.sh" ]; then
    "$SCRIPT_DIR/check-dependencies.sh"
    DEPS_OK=$?
else
    echo -e "${YELLOW}Warning:${NC} Dependency checker script not found"
    echo "Proceeding without dependency validation..."
    DEPS_OK=0
fi

if [ $DEPS_OK -ne 0 ]; then
    echo ""
    echo -e "${RED}✗ Pre-build check failed!${NC}"
    echo "Please install missing dependencies before building."
    echo ""
    echo "Run this command to see what's missing:"
    echo "  ./scripts/check-dependencies.sh"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Pre-build check passed!${NC}"
echo "You can now proceed with building Soul Player."
exit 0
