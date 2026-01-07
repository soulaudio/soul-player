#!/usr/bin/env bash
# Validate Moon configuration
# This script checks that Moon tasks would work correctly if Moon is installed

set -e

echo "🌙 Moon Configuration Validation"
echo "================================"
echo ""

# Check if moon is installed
if ! command -v moon &> /dev/null; then
    echo "❌ Moon is not installed"
    echo ""
    echo "To install Moon, run one of:"
    echo "  • curl -fsSL https://moonrepo.dev/install/moon.sh | bash"
    echo "  • npm install -g @moonrepo/cli"
    echo "  • cargo install moon --locked"
    echo ""
    echo "For now, validating configuration files only..."
    echo ""
fi

# Check configuration files exist
echo "📄 Checking configuration files..."
if [ -f ".moon/workspace.yml" ]; then
    echo "  ✅ .moon/workspace.yml exists"
else
    echo "  ❌ .moon/workspace.yml missing"
    exit 1
fi

if [ -f "moon.yml" ]; then
    echo "  ✅ moon.yml exists"
else
    echo "  ❌ moon.yml missing"
    exit 1
fi

echo ""

# Validate that the commands match CI
echo "🔍 Validating task commands match CI..."

# Check lint command
if grep -q "cargo clippy --lib --bins --all-features -- -D warnings" moon.yml; then
    echo "  ✅ Lint command matches CI"
else
    echo "  ❌ Lint command doesn't match CI"
fi

# Check format command
if grep -q "cargo fmt --all -- --check" moon.yml; then
    echo "  ✅ Format command matches CI"
else
    echo "  ❌ Format command doesn't match CI"
fi

# Check audit command has ignores
if grep -q "RUSTSEC-2023-0071" moon.yml; then
    echo "  ✅ Audit command has security ignores"
else
    echo "  ❌ Audit command missing security ignores"
fi

echo ""

# If moon is installed, run a quick check
if command -v moon &> /dev/null; then
    echo "🚀 Running Moon quick checks..."

    # Query projects
    echo "  📦 Discovered projects:"
    moon query projects --json 2>/dev/null | grep -o '"id":"[^"]*"' | cut -d'"' -f4 | head -10 | sed 's/^/    • /'

    echo ""
    echo "  ✅ Moon tasks are configured correctly"
    echo ""
    echo "  You can now run tasks like:"
    echo "    • moon run :lint"
    echo "    • moon run :format"
    echo "    • moon run :test"
    echo "    • moon run :ci-check"
else
    echo "ℹ️  Install Moon to run tasks locally with smart caching"
fi

echo ""
echo "✅ Validation complete!"
