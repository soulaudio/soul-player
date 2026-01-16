#!/bin/bash
# CI Build Test - Run all tests
# This emulates the CI test phase

set -e

echo "==================================="
echo "CI Build Test"
echo "==================================="

# Ensure we're in the project root
cd "$(dirname "$0")/.."

echo ""
echo "→ Installing dependencies..."
yarn install --immutable

echo ""
echo "→ Running Rust tests..."
cargo test --all --verbose

echo ""
echo "→ Running frontend tests..."
yarn test

echo ""
echo "✅ All tests passed!"
