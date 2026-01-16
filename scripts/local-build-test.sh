#!/bin/bash
# Local build testing script - emulates CI build pipeline

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_DIR="$PROJECT_ROOT/docker/build-env"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${BLUE}============================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Parse arguments
PLATFORM="${1:-all}"
BUILD_TYPE="${2:-packages}"

show_help() {
    cat << EOF
Usage: $0 [PLATFORM] [BUILD_TYPE]

PLATFORM:
  linux           Build Linux packages only
  windows         Build Windows packages only (cross-compilation)
  all             Build all platforms (default)

BUILD_TYPE:
  check           Run cargo check and clippy only
  test            Run cargo test
  build           Build Rust binaries only
  packages        Build full installers/packages (default)

Examples:
  $0 linux check          # Check Linux build
  $0 windows build        # Build Windows binary
  $0 all packages         # Build all installers

Environment Variables:
  SKIP_DOCKER_BUILD=1     Skip rebuilding Docker images
  KEEP_CONTAINER=1        Don't remove container after build
  VERBOSE=1               Show detailed output

EOF
    exit 0
}

if [[ "$PLATFORM" == "-h" ]] || [[ "$PLATFORM" == "--help" ]]; then
    show_help
fi

# Change to project root
cd "$PROJECT_ROOT"

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed. Please install Docker Desktop or Docker Engine."
    exit 1
fi

# Build or pull Docker images
build_docker_images() {
    local platform=$1
    print_header "Building Docker Images for $platform"

    if [[ -n "$SKIP_DOCKER_BUILD" ]]; then
        print_warning "Skipping Docker build (SKIP_DOCKER_BUILD=1)"
        return
    fi

    cd "$DOCKER_DIR"

    if [[ "$platform" == "linux" ]] || [[ "$platform" == "all" ]]; then
        docker compose build linux-builder
        print_success "Linux builder image ready"
    fi

    if [[ "$platform" == "windows" ]] || [[ "$platform" == "all" ]]; then
        docker compose build windows-cross-builder
        print_success "Windows cross-builder image ready"
    fi

    cd "$PROJECT_ROOT"
}

# Run build in Docker container
run_build() {
    local platform=$1
    local build_type=$2
    local service_name=""

    case $platform in
        linux)
            service_name="linux-builder"
            ;;
        windows)
            service_name="windows-cross-builder"
            ;;
        *)
            print_error "Unknown platform: $platform"
            return 1
            ;;
    esac

    print_header "Building $platform ($build_type)"

    local docker_opts=""
    if [[ -n "$KEEP_CONTAINER" ]]; then
        docker_opts="--no-rm"
    fi

    local build_cmd=""
    case $build_type in
        check)
            build_cmd="./scripts/ci-build-check.sh"
            ;;
        test)
            build_cmd="./scripts/ci-build-test.sh"
            ;;
        build)
            build_cmd="./scripts/ci-build-binary.sh $platform"
            ;;
        packages)
            build_cmd="./scripts/ci-build-packages.sh $platform"
            ;;
        *)
            print_error "Unknown build type: $build_type"
            return 1
            ;;
    esac

    cd "$DOCKER_DIR"

    if [[ -n "$VERBOSE" ]]; then
        docker compose run --rm $docker_opts \
            --profile $(echo $platform | cut -d'-' -f1) \
            $service_name \
            bash -c "$build_cmd"
    else
        docker compose run --rm $docker_opts \
            --profile $(echo $platform | cut -d'-' -f1) \
            $service_name \
            bash -c "$build_cmd" 2>&1 | tee "build-$platform.log"
    fi

    local exit_code=${PIPESTATUS[0]}
    cd "$PROJECT_ROOT"

    if [[ $exit_code -eq 0 ]]; then
        print_success "$platform build completed successfully"
        return 0
    else
        print_error "$platform build failed (exit code: $exit_code)"
        if [[ -z "$VERBOSE" ]]; then
            print_warning "Check build-$platform.log for details"
        fi
        return 1
    fi
}

# Main execution
main() {
    print_header "Soul Player - Local Build Testing"
    echo "Platform: $PLATFORM"
    echo "Build Type: $BUILD_TYPE"
    echo ""

    # Build Docker images
    build_docker_images "$PLATFORM"

    # Run builds
    local failed=0

    if [[ "$PLATFORM" == "linux" ]] || [[ "$PLATFORM" == "all" ]]; then
        if ! run_build "linux" "$BUILD_TYPE"; then
            failed=$((failed + 1))
        fi
    fi

    if [[ "$PLATFORM" == "windows" ]] || [[ "$PLATFORM" == "all" ]]; then
        if ! run_build "windows" "$BUILD_TYPE"; then
            failed=$((failed + 1))
        fi
    fi

    # Summary
    print_header "Build Summary"
    if [[ $failed -eq 0 ]]; then
        print_success "All builds completed successfully!"
        exit 0
    else
        print_error "$failed build(s) failed"
        exit 1
    fi
}

main
