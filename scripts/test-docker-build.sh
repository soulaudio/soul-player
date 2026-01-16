#!/usr/bin/env bash
# =============================================================================
# Test Docker Build for Soul Server
# =============================================================================
# This script emulates the CI Docker build environment locally.
# It runs the same Docker build command that GitHub Actions uses.
#
# Usage:
#   ./scripts/test-docker-build.sh [--no-cache] [--platform PLATFORM]
#
# Options:
#   --no-cache          Build without using Docker cache
#   --platform PLATFORM Build for specific platform (e.g., linux/amd64, linux/arm64)
#   --help              Show this help message
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
NO_CACHE=""
PLATFORM=""
IMAGE_NAME="soul-server:local-test"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --no-cache)
      NO_CACHE="--no-cache"
      shift
      ;;
    --platform)
      PLATFORM="--platform $2"
      shift 2
      ;;
    --help)
      head -n 15 "$0" | tail -n +3 | sed 's/^# //'
      exit 0
      ;;
    *)
      echo -e "${RED}Error: Unknown option $1${NC}"
      exit 1
      ;;
  esac
done

# Check if we're in the project root
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "applications/server" ]]; then
  echo -e "${RED}Error: This script must be run from the project root directory${NC}"
  exit 1
fi

echo -e "${BLUE}==============================================================================${NC}"
echo -e "${BLUE}Soul Player Server - Local Docker Build Test${NC}"
echo -e "${BLUE}==============================================================================${NC}"
echo ""
echo -e "${YELLOW}This will build the Docker image exactly as CI does.${NC}"
echo -e "${YELLOW}Build options:${NC}"
echo -e "  Image name: ${GREEN}${IMAGE_NAME}${NC}"
echo -e "  No cache:   ${GREEN}${NO_CACHE:-No}${NC}"
echo -e "  Platform:   ${GREEN}${PLATFORM:-Default (current architecture)}${NC}"
echo ""
echo -e "${YELLOW}Starting build...${NC}"
echo ""

# Build the Docker image
if docker build \
  -f applications/server/Dockerfile \
  -t "${IMAGE_NAME}" \
  ${NO_CACHE} \
  ${PLATFORM} \
  . ; then
  echo ""
  echo -e "${GREEN}==============================================================================${NC}"
  echo -e "${GREEN}✓ Docker build completed successfully!${NC}"
  echo -e "${GREEN}==============================================================================${NC}"
  echo ""
  echo -e "${BLUE}Image details:${NC}"
  docker images "${IMAGE_NAME}" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"
  echo ""
  echo -e "${YELLOW}To run the server:${NC}"
  echo -e "  ${GREEN}docker run -p 8080:8080 -v \$(pwd)/data:/app/data ${IMAGE_NAME}${NC}"
  echo ""
  echo -e "${YELLOW}To test the build for multiple platforms (as CI does):${NC}"
  echo -e "  ${GREEN}./scripts/test-docker-build.sh --platform linux/amd64${NC}"
  echo -e "  ${GREEN}./scripts/test-docker-build.sh --platform linux/arm64${NC}"
  echo ""
else
  echo ""
  echo -e "${RED}==============================================================================${NC}"
  echo -e "${RED}✗ Docker build failed!${NC}"
  echo -e "${RED}==============================================================================${NC}"
  echo ""
  echo -e "${YELLOW}Common issues:${NC}"
  echo -e "  1. Ensure Docker is running"
  echo -e "  2. Check that you have enough disk space"
  echo -e "  3. Try running with --no-cache if you suspect cache issues"
  echo -e "  4. Ensure all required files exist (check .dockerignore)"
  echo ""
  exit 1
fi
