#!/usr/bin/env bash
# Setup virtual audio devices for E2E testing
# Supports: Linux (ALSA), macOS (BlackHole), Windows (via WSL)
#
# Usage:
#   ./scripts/setup-virtual-audio.sh        # Auto-detect platform
#   ./scripts/setup-virtual-audio.sh linux  # Force Linux setup
#   ./scripts/setup-virtual-audio.sh macos  # Force macOS setup

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        CYGWIN*|MINGW*|MSYS*) echo "windows";;
        *)          echo "unknown";;
    esac
}

# Setup Linux virtual audio (ALSA snd-aloop)
setup_linux() {
    log_info "Setting up Linux virtual audio device (ALSA snd-aloop)"

    # Check if running with sudo
    if [ "$EUID" -ne 0 ]; then
        log_warn "Some operations require sudo. You may be prompted for your password."
    fi

    # Load snd-aloop kernel module
    log_info "Loading snd-aloop kernel module..."
    if sudo modprobe snd-aloop; then
        log_info "✅ snd-aloop module loaded successfully"
    else
        log_error "Failed to load snd-aloop module"
        log_warn "This may be normal in containerized environments or systems without module support"
        return 1
    fi

    # Verify module is loaded
    if lsmod | grep -q snd_aloop; then
        log_info "✅ snd-aloop module is active"
    else
        log_warn "snd-aloop module not visible in lsmod"
    fi

    # List ALSA devices
    log_info "Available playback devices:"
    aplay -l || log_warn "Could not list playback devices"

    log_info "Available capture devices:"
    arecord -l || log_warn "Could not list capture devices"

    # Create ALSA configuration
    log_info "Creating ALSA configuration..."
    mkdir -p ~/.config/alsa

    cat > ~/.config/alsa/asoundrc <<'EOF'
# Virtual loopback device for Soul Player E2E testing
# This creates a default device that routes to the ALSA loopback

pcm.!default {
    type plug
    slave.pcm "loopback"
}

pcm.loopback {
    type asym
    playback.pcm "hw:Loopback,0,0"
    capture.pcm "hw:Loopback,1,0"
}

ctl.!default {
    type hw
    card Loopback
}
EOF

    log_info "✅ ALSA configuration created at ~/.config/alsa/asoundrc"

    # Test ALSA configuration
    log_info "Testing ALSA configuration..."
    if aplay -L | grep -q "default"; then
        log_info "✅ Default device configured"
    else
        log_warn "Default device not found in aplay -L"
    fi

    log_info ""
    log_info "Linux virtual audio setup complete!"
    log_info "To persist the module across reboots, run:"
    log_info "  echo 'snd-aloop' | sudo tee -a /etc/modules-load.d/audio-testing.conf"
}

# Setup macOS virtual audio (BlackHole)
setup_macos() {
    log_info "Setting up macOS virtual audio device (BlackHole)"

    # Check if Homebrew is installed
    if ! command -v brew &> /dev/null; then
        log_error "Homebrew is not installed"
        log_info "Install Homebrew from https://brew.sh then re-run this script"
        return 1
    fi

    log_info "✅ Homebrew found"

    # Check if BlackHole is already installed
    if brew list blackhole-2ch &> /dev/null; then
        log_info "BlackHole 2ch is already installed"
        log_info "Reinstalling to ensure it's up to date..."
        brew reinstall blackhole-2ch
    else
        log_info "Installing BlackHole 2ch..."
        brew install blackhole-2ch
    fi

    # Wait for device to initialize
    log_info "Waiting for BlackHole to initialize (5 seconds)..."
    sleep 5

    # Verify installation
    log_info "Checking for BlackHole device..."
    if system_profiler SPAudioDataType | grep -i "blackhole"; then
        log_info "✅ BlackHole device found"
        system_profiler SPAudioDataType | grep -A 10 -i "blackhole"
    else
        log_warn "BlackHole device not detected immediately"
        log_warn "It may appear after a few seconds or a system restart"
    fi

    log_info ""
    log_info "macOS virtual audio setup complete!"
    log_info "You can verify the device in System Preferences > Sound"
    log_info "To set as default output (optional):"
    log_info "  System Preferences > Sound > Output > BlackHole 2ch"
}

# Setup Windows virtual audio (via WSL - informational only)
setup_windows() {
    log_warn "Windows virtual audio setup requires manual installation"
    log_info ""
    log_info "To install VB-Cable on Windows:"
    log_info "1. Download from: https://vb-audio.com/Cable/"
    log_info "2. Extract the ZIP file"
    log_info "3. Right-click VBCABLE_Setup_x64.exe"
    log_info "4. Select 'Run as administrator'"
    log_info "5. Follow the installation wizard"
    log_info "6. Restart your computer when prompted"
    log_info ""
    log_info "After installation, VB-Cable will appear in Sound settings as:"
    log_info "  - 'CABLE Input' (virtual microphone)"
    log_info "  - 'CABLE Output' (virtual speaker)"
    log_info ""
    log_info "For PowerShell automation, see:"
    log_info "  .github/workflows/audio-e2e-tests.yml (Windows job)"
}

# Verify setup by running basic tests
verify_setup() {
    local platform=$1

    log_info "Verifying setup by running basic audio tests..."

    case $platform in
        linux)
            log_info "Testing ALSA device access..."
            if aplay -L | head -20; then
                log_info "✅ ALSA devices accessible"
            else
                log_warn "Could not access ALSA devices"
            fi
            ;;
        macos)
            log_info "Testing CoreAudio device access..."
            if system_profiler SPAudioDataType | grep -c "Audio Device" > /dev/null; then
                log_info "✅ CoreAudio devices accessible"
            else
                log_warn "Could not access CoreAudio devices"
            fi
            ;;
        windows)
            log_info "Manual verification required on Windows"
            log_info "Check Sound settings to verify VB-Cable installation"
            ;;
    esac

    log_info ""
    log_info "Running Rust audio E2E tests..."
    log_info "This may take a few minutes..."

    if cargo test --release --package soul-audio-desktop --test device_hotplug_e2e -- --test-threads=1 --nocapture 2>&1 | head -50; then
        log_info "✅ Audio E2E tests started successfully"
        log_info "(Showing first 50 lines of output - full test results above)"
    else
        log_warn "Tests may have issues - check output above"
    fi
}

# Cleanup function
cleanup() {
    local platform=$1

    log_info "Cleaning up virtual audio setup..."

    case $platform in
        linux)
            log_info "Removing ALSA configuration..."
            rm -f ~/.config/alsa/asoundrc
            log_info "Unloading snd-aloop module..."
            sudo modprobe -r snd-aloop || log_warn "Could not unload module"
            log_info "✅ Linux cleanup complete"
            ;;
        macos)
            log_info "Uninstalling BlackHole..."
            brew uninstall blackhole-2ch || log_warn "Could not uninstall BlackHole"
            log_info "✅ macOS cleanup complete"
            ;;
        windows)
            log_info "Uninstall VB-Cable via Windows Settings > Apps"
            ;;
    esac
}

# Main script
main() {
    local platform="${1:-}"
    local action="${2:-setup}"

    # Detect platform if not specified
    if [ -z "$platform" ]; then
        platform=$(detect_platform)
        log_info "Auto-detected platform: $platform"
    fi

    # Validate platform
    case $platform in
        linux|macos|windows)
            ;;
        *)
            log_error "Unknown platform: $platform"
            log_info "Supported platforms: linux, macos, windows"
            exit 1
            ;;
    esac

    # Execute action
    case $action in
        setup)
            log_info "=== Soul Player - Virtual Audio Setup ==="
            log_info "Platform: $platform"
            log_info ""

            case $platform in
                linux)
                    setup_linux
                    verify_setup linux
                    ;;
                macos)
                    setup_macos
                    verify_setup macos
                    ;;
                windows)
                    setup_windows
                    ;;
            esac
            ;;
        cleanup)
            cleanup "$platform"
            ;;
        verify)
            verify_setup "$platform"
            ;;
        *)
            log_error "Unknown action: $action"
            log_info "Supported actions: setup, cleanup, verify"
            exit 1
            ;;
    esac

    log_info ""
    log_info "=== Setup Complete ==="
    log_info "You can now run audio E2E tests with:"
    log_info "  cargo test --release --package soul-audio-desktop --test device_hotplug_e2e"
    log_info ""
    log_info "For cleanup, run:"
    log_info "  $0 $platform cleanup"
}

# Run main function
main "$@"
