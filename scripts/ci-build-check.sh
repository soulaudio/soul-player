#!/bin/bash
# CI Build Check - Run format, clippy, and type checks
# This emulates the CI quality checks

set -e

echo "==================================="
echo "CI Build Check"
echo "==================================="

# Ensure we're in the project root
cd "$(dirname "$0")/.."

echo ""
echo "→ Installing dependencies..."
yarn install --immutable

echo ""
echo "→ Running Rust format check..."
cargo fmt --all --check

echo ""
echo "→ Running Clippy (linting)..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo ""
echo "→ Running TypeScript type checks..."
yarn type-check

echo ""
echo "→ Running ESLint..."
yarn lint

echo ""
echo "✅ All checks passed!"
