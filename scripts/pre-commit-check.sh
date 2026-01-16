#!/usr/bin/env bash
# Pre-commit quality checks
# Run this before committing to ensure CI will pass

set -e  # Exit on first error

echo "===================="
echo "Pre-Commit Checks"
echo "===================="
echo ""

# Rust checks
echo "→ Checking Rust formatting..."
cargo fmt --all --check
echo "✓ Rust formatting OK"
echo ""

echo "→ Running Clippy..."
cargo clippy --workspace --lib --bins --release -- -D warnings
echo "✓ Clippy OK"
echo ""

echo "→ Running Rust tests..."
cargo test --all --quiet
echo "✓ Rust tests OK"
echo ""

# TypeScript checks
echo "→ TypeScript check - Desktop..."
yarn workspace soul-player-desktop run tsc --noEmit
echo "✓ Desktop TypeScript OK"
echo ""

echo "→ TypeScript check - Shared..."
yarn workspace @soul-player/shared run tsc --noEmit
echo "✓ Shared TypeScript OK"
echo ""

echo "→ TypeScript check - Marketing..."
yarn workspace @soul-player/marketing run tsc --noEmit
echo "✓ Marketing TypeScript OK"
echo ""

echo "→ ESLint - Desktop..."
yarn workspace soul-player-desktop run lint
echo "✓ Desktop ESLint OK"
echo ""

echo "→ ESLint - Shared..."
yarn workspace @soul-player/shared run lint
echo "✓ Shared ESLint OK"
echo ""

echo "===================="
echo "✓ All checks passed!"
echo "===================="
echo "Safe to commit."
