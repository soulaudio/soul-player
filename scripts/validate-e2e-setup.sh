#!/usr/bin/env bash
# Validate E2E Testing Setup
# This script checks that all dependencies and configuration are properly installed

set -e

echo "=== Soul Player E2E Testing Setup Validation ==="
echo ""

ERRORS=0

# Check Rust toolchain
echo -n "Checking Rust toolchain... "
if command -v cargo &> /dev/null; then
    echo "✓ ($(cargo --version))"
else
    echo "✗ Rust not found"
    ERRORS=$((ERRORS + 1))
fi

# Check Node.js
echo -n "Checking Node.js... "
if command -v node &> /dev/null; then
    echo "✓ ($(node --version))"
else
    echo "✗ Node.js not found"
    ERRORS=$((ERRORS + 1))
fi

# Check Yarn
echo -n "Checking Yarn... "
if command -v yarn &> /dev/null; then
    echo "✓ ($(yarn --version))"
else
    echo "✗ Yarn not found"
    ERRORS=$((ERRORS + 1))
fi

# Check tauri-driver
echo -n "Checking tauri-driver... "
if command -v tauri-driver &> /dev/null; then
    echo "✓ ($(tauri-driver --version 2>&1 || echo 'installed'))"
else
    echo "✗ tauri-driver not found"
    echo "  Install with: cargo install tauri-driver"
    ERRORS=$((ERRORS + 1))
fi

# Check workspace members
echo -n "Checking workspace members... "
if grep -q "xtask" Cargo.toml; then
    echo "✓ xtask in workspace"
else
    echo "✗ xtask not in workspace"
    ERRORS=$((ERRORS + 1))
fi

# Check test dependencies
echo -n "Checking test dependencies... "
if grep -q "tauri-driver" Cargo.toml; then
    echo "✓ tauri-driver dependency found"
else
    echo "✗ tauri-driver dependency missing"
    ERRORS=$((ERRORS + 1))
fi

# Check package.json scripts
echo -n "Checking package.json scripts... "
if grep -q "test:audio:e2e" package.json; then
    echo "✓ E2E test scripts found"
else
    echo "✗ E2E test scripts missing"
    ERRORS=$((ERRORS + 1))
fi

# Check .env.example
echo -n "Checking .env.example... "
if [ -f "applications/desktop/e2e-tests/.env.example" ]; then
    echo "✓ Template found"
else
    echo "✗ .env.example missing"
    ERRORS=$((ERRORS + 1))
fi

# Check xtask crate
echo -n "Checking xtask crate... "
if [ -f "xtask/Cargo.toml" ]; then
    echo "✓ xtask/Cargo.toml found"
else
    echo "✗ xtask/Cargo.toml missing"
    ERRORS=$((ERRORS + 1))
fi

# Check E2E tests directory
echo -n "Checking E2E tests directory... "
if [ -d "applications/desktop/e2e-tests" ]; then
    echo "✓ E2E tests directory exists"
else
    echo "✗ E2E tests directory missing"
    ERRORS=$((ERRORS + 1))
fi

# Check WebdriverIO config
echo -n "Checking WebdriverIO config... "
if [ -f "applications/desktop/e2e-tests/wdio.conf.js" ]; then
    echo "✓ wdio.conf.js found"
else
    echo "✗ wdio.conf.js missing"
    ERRORS=$((ERRORS + 1))
fi

# Summary
echo ""
echo "==================================="
if [ $ERRORS -eq 0 ]; then
    echo "✓ All checks passed!"
    echo ""
    echo "You can now run E2E tests:"
    echo "  yarn test:audio:e2e"
    echo "  yarn test:audio:e2e:all"
    echo "  cargo xtask test audio e2e"
    exit 0
else
    echo "✗ $ERRORS check(s) failed"
    echo ""
    echo "Please fix the issues above before running E2E tests."
    exit 1
fi
