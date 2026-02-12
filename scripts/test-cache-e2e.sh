#!/bin/bash
# Run cache invalidation E2E tests

set -e

echo "💾 Running cache invalidation E2E tests..."

yarn workspace @soul-player/shared test \
  --testPathPattern cache \
  --watch=false

echo "✅ Cache E2E tests passed"
