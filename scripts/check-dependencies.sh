#!/bin/bash
# Dependency checker for Soul Player Linux builds
# This script verifies all required system dependencies are installed

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall success
ALL_OK=true

echo "=================================================="
echo "Soul Player - Linux Dependency Checker"
echo "=================================================="
echo ""

# Function to check if a command exists
check_command() {
    local cmd="$1"
    local pkg="$2"
    local install_cmd="$3"

    if command -v "$cmd" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $cmd found"
        return 0
    else
        echo -e "${RED}✗${NC} $cmd not found"
        echo -e "  ${YELLOW}Install:${NC} $install_cmd"
        ALL_OK=false
        return 1
    fi
}

# Function to check if a library is installed
check_library() {
    local lib="$1"
    local pkg="$2"
    local install_cmd="$3"

    if pkg-config --exists "$lib" 2>/dev/null; then
        local version=$(pkg-config --modversion "$lib")
        echo -e "${GREEN}✓${NC} $lib found (version $version)"
        return 0
    else
        echo -e "${RED}✗${NC} $lib not found"
        echo -e "  ${YELLOW}Install:${NC} $install_cmd"
        ALL_OK=false
        return 1
    fi
}

# Function to check if a file exists
check_file() {
    local file="$1"
    local pkg="$2"
    local install_cmd="$3"

    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file found"
        return 0
    else
        echo -e "${RED}✗${NC} $file not found"
        echo -e "  ${YELLOW}Install:${NC} $install_cmd"
        ALL_OK=false
        return 1
    fi
}

echo "Checking build tools..."
echo "----------------------"
check_command "cargo" "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
check_command "rustc" "rustc" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
check_command "pkg-config" "pkg-config" "sudo apt-get install pkg-config  # or yum/dnf install pkgconf"
echo ""

echo "Checking Rust version..."
echo "------------------------"
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    MIN_VERSION="1.75.0"

    if [ "$(printf '%s\n' "$MIN_VERSION" "$RUST_VERSION" | sort -V | head -n1)" = "$MIN_VERSION" ]; then
        echo -e "${GREEN}✓${NC} Rust $RUST_VERSION (>= $MIN_VERSION required)"
    else
        echo -e "${RED}✗${NC} Rust $RUST_VERSION (>= $MIN_VERSION required)"
        echo -e "  ${YELLOW}Update:${NC} rustup update stable"
        ALL_OK=false
    fi
fi
echo ""

echo "Checking audio libraries (required for desktop build)..."
echo "---------------------------------------------------------"
check_library "alsa" "libasound2-dev" "sudo apt-get install libasound2-dev  # or yum/dnf install alsa-lib-devel"
echo ""

echo "Checking optional development dependencies..."
echo "----------------------------------------------"
check_command "moon" "moon" "cargo install moon --locked  # or curl -fsSL https://moonrepo.dev/install/moon.sh | bash"
echo ""

# Detect distribution
echo "Detecting Linux distribution..."
echo "-------------------------------"
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo -e "${GREEN}✓${NC} Detected: $NAME $VERSION"
    echo ""

    # Provide distribution-specific installation commands
    if [ -n "$ID" ]; then
        case "$ID" in
            ubuntu|debian)
                INSTALL_CMD="sudo apt-get install"
                ;;
            fedora)
                INSTALL_CMD="sudo dnf install"
                ;;
            centos|rhel)
                INSTALL_CMD="sudo yum install"
                ;;
            arch)
                INSTALL_CMD="sudo pacman -S"
                ;;
            *)
                INSTALL_CMD="(use your package manager)"
                ;;
        esac
    fi
fi

# Summary
echo "=================================================="
if [ "$ALL_OK" = true ]; then
    echo -e "${GREEN}✓ All dependencies satisfied!${NC}"
    echo ""
    echo "You can now build Soul Player:"
    echo "  cargo build --workspace"
    echo "  # or"
    echo "  moon run :build"
    exit 0
else
    echo -e "${RED}✗ Some dependencies are missing.${NC}"
    echo ""
    echo "Quick install command for your distribution:"

    if [ -n "$ID" ]; then
        case "$ID" in
            ubuntu|debian)
                echo "  sudo apt-get update"
                echo "  sudo apt-get install -y pkg-config libasound2-dev"
                ;;
            fedora)
                echo "  sudo dnf install -y pkgconf alsa-lib-devel"
                ;;
            centos|rhel)
                echo "  sudo yum install -y pkgconfig alsa-lib-devel"
                ;;
            arch)
                echo "  sudo pacman -S pkg-config alsa-lib"
                ;;
            *)
                echo "  Install: pkg-config, ALSA development libraries"
                ;;
        esac
    fi

    echo ""
    echo "After installing dependencies, run this script again to verify."
    exit 1
fi
