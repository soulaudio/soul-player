#!/bin/bash
# Generate test audio files for E2E audio testing
#
# Requires: sox (Sound eXchange)
#   macOS:   brew install sox
#   Linux:   sudo apt install sox
#   Windows: choco install sox

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ASSETS_DIR="$PROJECT_ROOT/tests/assets"

echo "=== Audio Test Asset Generator ==="
echo "Output directory: $ASSETS_DIR"

# Check if sox is installed
if ! command -v sox &> /dev/null; then
    echo "Error: sox is not installed"
    echo ""
    echo "Install with:"
    echo "  macOS:   brew install sox"
    echo "  Linux:   sudo apt install sox"
    echo "  Windows: choco install sox"
    exit 1
fi

# Create assets directory if it doesn't exist
mkdir -p "$ASSETS_DIR"

echo ""
echo "Generating test audio files..."

# 1. Pure 1kHz sine wave (10 seconds) - For phase continuity testing
echo "  [1/6] 1kHz sine wave (10s)..."
sox -n -r 44100 -c 2 -b 16 "$ASSETS_DIR/1khz-sine-10s.wav" synth 10 sine 1000

# 2. Pure 1kHz sine wave (30 seconds) - Extended testing
echo "  [2/6] 1kHz sine wave (30s)..."
sox -n -r 44100 -c 2 -b 16 "$ASSETS_DIR/1khz-sine-30s.wav" synth 30 sine 1000

# 3. Distinctive intro pattern - For false-start detection
# Pattern: kick drum at 0s, 0.5s, 1.0s
echo "  [3/6] Distinctive intro pattern..."
sox -n -r 44100 -c 2 -b 16 "$ASSETS_DIR/distinctive-intro.wav" \
    synth 0.1 sine 60 : synth 0.4 sine 0 : \
    synth 0.1 sine 60 : synth 0.4 sine 0 : \
    synth 0.1 sine 60 : synth 9.4 sine 0

# 4. Extract first 500ms of intro for pattern matching
echo "  [4/6] Intro pattern reference (500ms)..."
sox "$ASSETS_DIR/distinctive-intro.wav" "$ASSETS_DIR/distinctive-intro-first-500ms.wav" trim 0 0.5

# 5. Silence (1 second) - For testing silence detection
echo "  [5/6] Silence (1s)..."
sox -n -r 44100 -c 2 -b 16 "$ASSETS_DIR/silence-1s.wav" trim 0 1

# 6. Mixed frequency content - Realistic audio testing
echo "  [6/6] Mixed content (complex waveform)..."
sox -n -r 44100 -c 2 -b 16 "$ASSETS_DIR/mixed-content.wav" synth 10 \
    sine 440:880 sine 880:1760 sine 220:440 \
    fade t 0.5 9.5 0.5

echo ""
echo "=== Generated Files ==="
ls -lh "$ASSETS_DIR"/*.wav | awk '{print $9, "(" $5 ")"}'

echo ""
echo "=== Verification ==="

# Verify each file
for file in "$ASSETS_DIR"/*.wav; do
    filename=$(basename "$file")
    info=$(sox --i "$file" 2>/dev/null || echo "Error reading file")
    echo "$filename:"
    echo "  $info" | grep -E "(Sample Rate|Channels|Duration|Sample Encoding)" | sed 's/^/  /'
done

echo ""
echo "✅ Test audio assets generated successfully!"
echo ""
echo "Next steps:"
echo "  1. Set up virtual audio device (see tests/e2e/README.md)"
echo "  2. Run tests:"
echo "       AUDIO_TEST_DEVICE=\"your-device\" cargo test --test audio_stutter_detection"
