# Linux Build Setup Guide

This guide covers setting up your Linux system for building Soul Player from source.

## Prerequisites

### Required Dependencies

| Component | Debian/Ubuntu | Fedora/RHEL | Arch Linux | Purpose |
|-----------|---------------|-------------|------------|---------|
| **Rust** | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | Same | Same | Build toolchain |
| **pkg-config** | `apt-get install pkg-config` | `dnf install pkgconf` | `pacman -S pkg-config` | Library detection |
| **ALSA** | `apt-get install libasound2-dev` | `dnf install alsa-lib-devel` | `pacman -S alsa-lib` | Audio output |

### Optional Dependencies

| Component | Installation | Purpose |
|-----------|--------------|---------|
| **Moon** | `cargo install moon --locked` | Task orchestration |
| **Tauri CLI** | `cargo install tauri-cli --locked` | Desktop app bundling |

## Quick Setup

### 1. Automated Dependency Check

Run our dependency checker script:

```bash
./scripts/check-dependencies.sh
```

This will:
- ✅ Verify all required dependencies
- ✅ Check Rust version (>= 1.75.0)
- ✅ Detect your Linux distribution
- ✅ Provide distribution-specific install commands

### 2. Manual Installation

#### Debian/Ubuntu

```bash
# Update package list
sudo apt-get update

# Install build dependencies
sudo apt-get install -y \
    pkg-config \
    libasound2-dev \
    build-essential

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should be >= 1.75.0
pkg-config --version
pkg-config --libs alsa
```

#### Fedora

```bash
# Install build dependencies
sudo dnf install -y \
    pkgconf \
    alsa-lib-devel \
    gcc

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Arch Linux

```bash
# Install build dependencies
sudo pacman -S \
    pkg-config \
    alsa-lib \
    base-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Building Soul Player

### Option 1: Using Cargo Directly

```bash
# Clone the repository
git clone https://github.com/yourusername/soul-player.git
cd soul-player

# Build all workspace crates
cargo build --workspace

# Build in release mode (optimized)
cargo build --workspace --release

# Build specific crate
cargo build -p soul-audio-desktop

# Run tests
cargo test --workspace
```

### Option 2: Using Moon (Recommended)

```bash
# Install Moon
cargo install moon --locked

# Build everything
moon run :build

# Run tests
moon run :test

# Run all CI checks
moon run :ci-check

# Build desktop app
moon run :build-desktop
```

## Verification

### 1. Verify Dependencies

```bash
# Check all dependencies
./scripts/check-dependencies.sh

# Manual verification
pkg-config --libs alsa
pkg-config --cflags alsa
ldconfig -p | grep libasound
```

### 2. Build Test

```bash
# Build soul-audio-desktop (tests audio dependencies)
cargo build -p soul-audio-desktop

# If successful, you'll see:
# "Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs"
```

### 3. Run Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p soul-audio-desktop

# Expected output:
# "test result: ok. X passed; 0 failed..."
```

## Troubleshooting

### Issue: `alsa-sys` build fails with "pkg-config not found"

**Cause**: `pkg-config` is not installed.

**Solution**:
```bash
# Debian/Ubuntu
sudo apt-get install pkg-config

# Fedora
sudo dnf install pkgconf

# Arch
sudo pacman -S pkg-config
```

### Issue: `alsa-sys` build fails with "Could not run pkg-config --libs alsa"

**Cause**: ALSA development libraries are not installed.

**Solution**:
```bash
# Debian/Ubuntu
sudo apt-get install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel

# Arch
sudo pacman -S alsa-lib
```

### Issue: Tests fail with "ALSA lib ... Unknown PCM default"

**Cause**: No audio devices available (common in headless/CI environments).

**Solution**: This is expected behavior. Tests are designed to gracefully handle missing audio devices. The tests pass by skipping audio playback when no device is available.

### Issue: "error: linker `cc` not found"

**Cause**: C compiler not installed.

**Solution**:
```bash
# Debian/Ubuntu
sudo apt-get install build-essential

# Fedora
sudo dnf install gcc

# Arch
sudo pacman -S base-devel
```

### Issue: Rust version too old

**Cause**: Rust version < 1.75.0 (MSRV).

**Solution**:
```bash
rustup update stable
rustc --version  # Verify >= 1.75.0
```

## CI/CD Integration

### GitHub Actions

Our CI automatically installs dependencies on Linux runners:

```yaml
- name: Install Linux dependencies
  if: runner.os == 'Linux'
  run: |
    sudo apt-get update
    sudo apt-get install -y libasound2-dev pkg-config
```

### Local Pre-Commit Checks

Run before committing:

```bash
# Check dependencies
./scripts/check-dependencies.sh

# Run all CI checks locally
moon run :ci-check

# Or with cargo
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

## Development Workflow

### 1. First-Time Setup

```bash
# Clone repository
git clone https://github.com/yourusername/soul-player.git
cd soul-player

# Check dependencies
./scripts/check-dependencies.sh

# Install any missing dependencies (script will tell you how)

# Initial build
cargo build --workspace
```

### 2. Daily Development

```bash
# Pull latest changes
git pull

# Build changes
cargo build -p soul-audio-desktop

# Run tests
cargo test -p soul-audio-desktop

# Format code
cargo fmt --all

# Lint
cargo clippy --all-targets

# Run audio output test
cargo run --example sine_wave  # (when example is created)
```

### 3. Before Committing

```bash
# Run full CI checks
moon run :ci-check

# Or manually
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

## Build Configurations

### Debug Build (Default)

```bash
cargo build -p soul-audio-desktop
```

**Characteristics**:
- ❌ Not optimized
- ✅ Fast compilation
- ✅ Debug symbols included
- 📦 Larger binary size

### Release Build

```bash
cargo build -p soul-audio-desktop --release
```

**Characteristics**:
- ✅ Fully optimized
- ❌ Slower compilation
- ❌ No debug symbols (stripped)
- 📦 Smaller binary size
- 🚀 Best performance

### Development Profile

Optimizes dependencies but keeps debug info:

```toml
# Already configured in workspace Cargo.toml
[profile.dev.package."*"]
opt-level = 2
```

This gives you:
- ✅ Fast dependency code (audio processing)
- ✅ Fast compilation of your code
- ✅ Debug symbols for your code

## Platform-Specific Notes

### WSL (Windows Subsystem for Linux)

Audio device access in WSL is limited. Tests will skip audio playback but still verify code correctness.

```bash
# Tests will show:
# "No audio device found - skipping test"
# "Audio device unavailable - skipping test"

# This is expected and tests still pass
```

### Docker/CI

Headless environments won't have audio devices. Use:

```bash
# Build without running audio tests
cargo build --workspace

# Run tests (they'll skip audio device tests)
cargo test --workspace
```

### Cross-Compilation

For ARM targets:

```bash
# Install cross-compilation target
rustup target add aarch64-unknown-linux-gnu

# Install ARM toolchain
sudo apt-get install gcc-aarch64-linux-gnu

# Build for ARM
cargo build --target aarch64-unknown-linux-gnu
```

## Advanced: Custom Audio Backend

If you need to use a specific audio backend:

### PulseAudio

```bash
sudo apt-get install libpulse-dev
# CPAL will automatically detect and use PulseAudio
```

### JACK

```bash
sudo apt-get install libjack-dev
# Enable JACK feature in Cargo.toml if needed
```

### PipeWire

```bash
sudo apt-get install libpipewire-0.3-dev
# PipeWire provides PulseAudio compatibility
```

## Performance Tuning

### Optimize Audio Thread Priority

For better real-time audio performance:

```bash
# Add your user to realtime group
sudo usermod -a -G audio $USER

# Configure realtime limits
echo "@audio - rtprio 95" | sudo tee -a /etc/security/limits.conf
echo "@audio - memlock unlimited" | sudo tee -a /etc/security/limits.conf

# Logout and login for changes to take effect
```

## Summary Checklist

Before building Soul Player, ensure:

- [ ] Rust >= 1.75.0 installed (`rustc --version`)
- [ ] `pkg-config` installed (`pkg-config --version`)
- [ ] ALSA development libraries installed (`pkg-config --libs alsa`)
- [ ] Dependencies verified (`./scripts/check-dependencies.sh`)
- [ ] Repository cloned
- [ ] Build succeeds (`cargo build -p soul-audio-desktop`)
- [ ] Tests pass (`cargo test -p soul-audio-desktop`)

## Getting Help

If you encounter issues:

1. **Run dependency checker**: `./scripts/check-dependencies.sh`
2. **Check this guide**: Look for your specific error in Troubleshooting section
3. **Check logs**: Run with `RUST_LOG=debug` for detailed output
4. **Open an issue**: [GitHub Issues](https://github.com/yourusername/soul-player/issues)

## Related Documentation

- [LINUX_PACKAGING.md](LINUX_PACKAGING.md) - Creating .deb/.rpm/AppImage packages
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [TESTING.md](TESTING.md) - Testing strategy
