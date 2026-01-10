#!/bin/bash
set -e

# Build WASM module for soul-playback
# This script is called automatically during marketing build

echo "Building soul-playback WASM module..."

# Navigate to soul-playback directory
cd "$(dirname "$0")/../libraries/soul-playback"

# Build with wasm-pack
wasm-pack build \
  --target web \
  --out-dir ../../applications/marketing/src/wasm/soul-playback \
  --release \
  -- --features wasm

echo "WASM build complete: applications/marketing/src/wasm/soul-playback/"
