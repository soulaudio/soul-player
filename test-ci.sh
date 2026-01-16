#!/usr/bin/env bash
# test-ci.sh - Run CI tests locally in Docker
# This script replicates the exact CI environment from .github/workflows/ci.yml

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

show_help() {
    cat <<EOF
Soul Player CI Test Runner (Docker)
====================================

Run integration tests locally in an environment that exactly matches GitHub Actions CI.

USAGE:
    ./test-ci.sh [OPTIONS]

OPTIONS:
    --build         Force rebuild Docker image (use after Dockerfile.ci changes)
    --shell         Open interactive bash shell in CI container
    --clean         Remove all Docker volumes (cargo cache, target directory)
    --clippy        Run clippy lints instead of tests
    --format        Run format check instead of tests
    --package NAME  Run tests for specific package only (e.g., --package soul-playback)
    --help          Show this help message

EXAMPLES:
    # Run all integration tests (like CI)
    ./test-ci.sh

    # Run tests for a specific package
    ./test-ci.sh --package soul-playback

    # Run clippy (lint check)
    ./test-ci.sh --clippy

    # Run format check
    ./test-ci.sh --format

    # Open interactive shell for debugging
    ./test-ci.sh --shell

    # Rebuild image after Dockerfile changes
    ./test-ci.sh --build

    # Clean all caches and rebuild
    ./test-ci.sh --clean --build

PERFORMANCE:
    First run: ~5-10 minutes (downloads dependencies, compiles)
    Subsequent runs: ~1-3 minutes (cached dependencies and target)

VOLUMES (persistent caches):
    - cargo-registry:   Downloaded crates
    - cargo-git:        Git dependencies
    - cargo-target-ci:  Compiled artifacts (target/ directory)

To clean volumes: ./test-ci.sh --clean

EOF
    exit 0
}

# Parse arguments
BUILD=false
SHELL=false
CLEAN=false
CLIPPY=false
FORMAT=false
PACKAGE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --build)
            BUILD=true
            shift
            ;;
        --shell)
            SHELL=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --clippy)
            CLIPPY=true
            shift
            ;;
        --format)
            FORMAT=true
            shift
            ;;
        --package)
            PACKAGE="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information."
            exit 1
            ;;
    esac
done

# Ensure Docker is running
echo -e "${BLUE}Checking Docker...${NC}"
if ! docker info >/dev/null 2>&1; then
    echo -e "${RED}ERROR: Docker is not running. Please start Docker.${NC}"
    exit 1
fi

# Clean volumes if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning Docker volumes...${NC}"
    docker compose -f docker-compose.ci.yml down -v
    echo -e "${GREEN}Volumes cleaned.${NC}"
    if [ "$BUILD" = false ]; then
        exit 0
    fi
fi

# Build image if requested or if it doesn't exist
IMAGE_NAME="soul-player-ci-test"
if [ "$BUILD" = true ] || ! docker images -q $IMAGE_NAME 2>/dev/null | grep -q .; then
    echo -e "${BLUE}Building Docker image...${NC}"
    docker compose -f docker-compose.ci.yml build
    echo -e "${GREEN}Image built successfully.${NC}"
fi

# Open interactive shell if requested
if [ "$SHELL" = true ]; then
    echo -e "${BLUE}Opening interactive shell...${NC}"
    docker compose -f docker-compose.ci.yml run --rm ci-test bash
    exit $?
fi

# Determine command to run
COMMAND=""
if [ "$CLIPPY" = true ]; then
    echo -e "${BLUE}Running clippy...${NC}"
    COMMAND="mkdir -p applications/desktop/dist && \
echo '<!DOCTYPE html><html><head><title>Soul Player</title></head><body><div id=\"root\">Loading...</div></body></html>' > applications/desktop/dist/index.html && \
cargo clippy --workspace --lib --bins --release -- -D warnings"
elif [ "$FORMAT" = true ]; then
    echo -e "${BLUE}Running format check...${NC}"
    COMMAND="cargo fmt --all --check"
elif [ -n "$PACKAGE" ]; then
    echo -e "${BLUE}Running tests for package: $PACKAGE${NC}"
    COMMAND="mkdir -p applications/desktop/dist && \
echo '<!DOCTYPE html><html><head><title>Soul Player</title></head><body><div id=\"root\">Loading...</div></body></html>' > applications/desktop/dist/index.html && \
cargo test --tests --release -p $PACKAGE -- --test-threads=1"
else
    echo -e "${BLUE}Running all integration tests...${NC}"
    # Use default command from Dockerfile
    COMMAND=""
fi

# Run the container
if [ -n "$COMMAND" ]; then
    docker compose -f docker-compose.ci.yml run --rm ci-test bash -c "$COMMAND"
else
    docker compose -f docker-compose.ci.yml up --abort-on-container-exit
fi

EXIT_CODE=$?

# Show result
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "\n${GREEN}✅ Success! All checks passed.${NC}"
else
    echo -e "\n${RED}❌ Failed. Exit code: $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
