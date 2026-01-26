#!/bin/bash

# Complete restart script for marketing dev server
# This clears all caches and restarts fresh

echo "🧹 Stopping any running dev servers..."
pkill -f "next dev" || true

echo "🗑️  Clearing Next.js cache..."
rm -rf .next

echo "🗑️  Clearing node modules cache..."
rm -rf node_modules/.cache

echo "✅ Cache cleared!"
echo ""
echo "🚀 Starting dev server..."
yarn dev
