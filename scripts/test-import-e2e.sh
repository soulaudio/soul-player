#!/bin/bash
# Run import/re-import E2E tests

set -e

echo "📦 Running import E2E tests..."

cargo test \
  --package soul-importer \
  --test e2e_reimport_tests \
  -- \
  --test-threads=1 \
  --nocapture

echo "✅ Import E2E tests passed"
