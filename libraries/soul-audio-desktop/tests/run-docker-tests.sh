#!/bin/bash
# Run Docker-based E2E tests for device switching

set -e

echo "========================================"
echo "Soul Audio Desktop - Docker E2E Tests"
echo "========================================"
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "ERROR: Docker is not installed or not in PATH"
    echo "Please install Docker Desktop and try again"
    exit 1
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    echo "ERROR: Docker daemon is not running"
    echo "Please start Docker Desktop and try again"
    exit 1
fi

echo "✓ Docker is available"
echo ""

# Build the audio test Docker image
echo "Building audio test Docker image..."
cd "$(dirname "$0")/docker"
docker build -t soul-audio-test:latest -f Dockerfile.audio-test .
echo "✓ Docker image built successfully"
echo ""

# Go back to library root
cd ../..

# Run the Docker-based E2E tests
echo "Running Docker E2E tests..."
echo ""
cargo test --features docker-tests --test e2e_device_switching_docker -- --nocapture

echo ""
echo "========================================"
echo "All Docker E2E tests completed!"
echo "========================================"
