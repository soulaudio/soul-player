#!/bin/bash
# Setup script for soul-storage tests
#
# This script:
# 1. Creates a temporary database for SQLx compile-time checking
# 2. Runs migrations
# 3. Optionally generates offline mode cache

set -e

cd "$(dirname "$0")"

echo "==> Setting up soul-storage test environment"

# Create temp directory
mkdir -p .tmp

# Set DATABASE_URL
export DATABASE_URL="sqlite://$(pwd)/.tmp/sqlx-check.db"

echo "==> Creating database at: $DATABASE_URL"
cargo sqlx database create || echo "Database already exists"

echo "==> Running migrations"
cargo sqlx migrate run

echo "==> Generating offline mode cache (sqlx prepare)"
cargo sqlx prepare

echo ""
echo "==> Setup complete!"
echo ""
echo "You can now run tests with:"
echo "  cargo test"
echo ""
echo "Or check compilation with:"
echo "  cargo check"
echo ""
echo "To run tests with DATABASE_URL set:"
echo "  DATABASE_URL=\"$DATABASE_URL\" cargo test"
