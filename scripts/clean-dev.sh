#!/usr/bin/env bash
# Clean all development artifacts for Soul Player

set -e

echo "🧹 Cleaning Soul Player development artifacts..."

# Clean Rust build artifacts
echo ""
echo "1. Cleaning Rust target directory..."
if [ -d "target" ]; then
    rm -rf target
    echo "   ✓ Removed target/"
fi

if [ -d "applications/desktop/src-tauri/target" ]; then
    rm -rf applications/desktop/src-tauri/target
    echo "   ✓ Removed applications/desktop/src-tauri/target/"
fi

# Clean frontend dist folders
echo ""
echo "2. Cleaning frontend dist folders..."
find applications -type d -name "dist" -prune -exec rm -rf {} + 2>/dev/null || true
echo "   ✓ Removed all dist/ folders"

# Clean node_modules cache (optional)
# echo ""
# echo "3. Cleaning node_modules cache..."
# find . -type d -path "*/node_modules/.cache" -prune -exec rm -rf {} + 2>/dev/null || true

# Clean Yarn cache
echo ""
echo "3. Cleaning Yarn cache..."
yarn cache clean --all 2>/dev/null || true

# Clean SQLx offline data notice
echo ""
echo "4. SQLx offline data..."
if [ -d "libraries/soul-storage/.sqlx" ]; then
    echo "   ⚠ SQLx offline data found. To regenerate, run:"
    echo "   cargo sqlx prepare -- --lib"
fi

echo ""
echo "✅ Cleanup complete! Run 'yarn dev:desktop' to start fresh."
echo ""
echo "💡 Tips:"
echo "   • First start will be slower (rebuilding everything)"
echo "   • Frontend HMR should work after Vite starts"
echo "   • Rust changes require full rebuild"
