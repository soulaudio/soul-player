#!/bin/bash
# Install system dependencies for Soul Player development
# Usage: ./scripts/install-deps.sh

set -e

echo "=== Soul Player Development Environment Setup ==="
echo ""

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ -n "$WINDIR" ]]; then
    OS="windows"
fi

echo "Detected OS: $OS"
echo ""

# Check for required tools
check_command() {
    if command -v "$1" &> /dev/null; then
        echo "[OK] $1 found: $(command -v $1)"
        return 0
    else
        echo "[MISSING] $1 not found"
        return 1
    fi
}

install_rust() {
    if ! check_command rustc; then
        echo ""
        echo "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    echo "Rust version: $(rustc --version)"
}

install_node() {
    if ! check_command node; then
        echo ""
        echo "[WARNING] Node.js not found. Please install Node.js 20+ manually:"
        echo "  - https://nodejs.org/"
        echo "  - Or use nvm: https://github.com/nvm-sh/nvm"
        return 1
    fi
    echo "Node version: $(node --version)"
}

# ============================================================================
# Linux (Ubuntu/Debian)
# ============================================================================
install_linux() {
    echo ""
    echo "=== Installing Linux Dependencies ==="
    echo ""

    # Update package list
    sudo apt-get update

    # Tauri/WebKit dependencies
    echo "Installing Tauri dependencies..."
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf \
        pkg-config

    # Audio dependencies (ALSA)
    echo "Installing audio dependencies..."
    sudo apt-get install -y \
        libasound2-dev

    # Build dependencies (CMake for r8brain resampler)
    echo "Installing build tools..."
    sudo apt-get install -y \
        cmake \
        build-essential \
        clang

    # GTK dependencies for Tauri
    echo "Installing GTK dependencies..."
    sudo apt-get install -y \
        libglib2.0-dev \
        libgtk-3-dev

    # SQLite (for development)
    echo "Installing SQLite..."
    sudo apt-get install -y sqlite3

    # Cargo tools
    echo "Installing Cargo tools..."
    cargo install cargo-audit --locked 2>/dev/null || true
    cargo install sqlx-cli --no-default-features --features sqlite --locked 2>/dev/null || true

    echo ""
    echo "[OK] Linux dependencies installed successfully!"
}

# ============================================================================
# macOS
# ============================================================================
install_macos() {
    echo ""
    echo "=== Installing macOS Dependencies ==="
    echo ""

    # Check for Xcode Command Line Tools
    if ! xcode-select -p &> /dev/null; then
        echo "Installing Xcode Command Line Tools..."
        xcode-select --install
        echo "Please wait for Xcode tools to finish installing, then run this script again."
        exit 0
    fi
    echo "[OK] Xcode Command Line Tools installed"

    # Check for Homebrew
    if ! check_command brew; then
        echo ""
        echo "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi

    # Install dependencies via Homebrew
    echo "Installing dependencies via Homebrew..."
    brew install \
        cmake \
        pkg-config \
        sqlite

    # Cargo tools
    echo "Installing Cargo tools..."
    cargo install cargo-audit --locked 2>/dev/null || true
    cargo install sqlx-cli --no-default-features --features sqlite --locked 2>/dev/null || true

    echo ""
    echo "[OK] macOS dependencies installed successfully!"
}

# ============================================================================
# Windows (Git Bash / MSYS2)
# ============================================================================
install_windows() {
    echo ""
    echo "=== Windows Dependencies ==="
    echo ""
    echo "Windows requires manual installation of some dependencies."
    echo "Please ensure the following are installed:"
    echo ""
    echo "1. Visual Studio Build Tools (C++ workload)"
    echo "   https://visualstudio.microsoft.com/visual-cpp-build-tools/"
    echo ""
    echo "2. CMake (add to PATH)"
    echo "   https://cmake.org/download/"
    echo "   Or: winget install Kitware.CMake"
    echo ""
    echo "3. LLVM/Clang (for ASIO bindgen)"
    echo "   https://releases.llvm.org/"
    echo "   Or: winget install LLVM.LLVM"
    echo "   Set LIBCLANG_PATH=C:\\Program Files\\LLVM\\bin"
    echo ""
    echo "4. WebView2 Runtime (usually pre-installed on Windows 10/11)"
    echo "   https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
    echo ""

    # Check what's installed
    echo "Checking installed tools..."
    echo ""

    check_command cmake || echo "   Install: winget install Kitware.CMake"
    check_command clang || echo "   Install: winget install LLVM.LLVM"

    if [[ -z "$LIBCLANG_PATH" ]]; then
        echo "[WARNING] LIBCLANG_PATH not set"
        echo "   Add to environment: LIBCLANG_PATH=C:\\Program Files\\LLVM\\bin"
    else
        echo "[OK] LIBCLANG_PATH=$LIBCLANG_PATH"
    fi

    # Cargo tools
    echo ""
    echo "Installing Cargo tools..."
    cargo install cargo-audit --locked 2>/dev/null || true
    cargo install sqlx-cli --no-default-features --features sqlite --locked 2>/dev/null || true

    echo ""
    echo "After installing dependencies, run this script again to verify."
}

# ============================================================================
# Main
# ============================================================================

echo "=== Checking Prerequisites ==="
echo ""

install_rust
install_node

case $OS in
    linux)
        install_linux
        ;;
    macos)
        install_macos
        ;;
    windows)
        install_windows
        ;;
    *)
        echo "Unknown OS: $OSTYPE"
        echo "Please install dependencies manually. See README.md"
        exit 1
        ;;
esac

echo ""
echo "=== Final Setup ==="
echo ""

# Enable corepack for Yarn
if check_command corepack; then
    echo "Enabling Corepack for Yarn 4.x..."
    corepack enable 2>/dev/null || sudo corepack enable 2>/dev/null || true
fi

# Setup SQLx offline mode
if [[ -f "./scripts/setup-sqlx.sh" ]]; then
    echo "Setting up SQLx..."
    chmod +x ./scripts/setup-sqlx.sh
    ./scripts/setup-sqlx.sh || true
fi

echo ""
echo "=============================================="
echo "Setup complete! Next steps:"
echo ""
echo "  1. yarn install        # Install Node dependencies"
echo "  2. yarn dev:desktop    # Run desktop app"
echo ""
echo "See README.md for more commands."
echo "=============================================="
