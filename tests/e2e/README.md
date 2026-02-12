# Audio E2E Testing Suite

End-to-end audio testing infrastructure for Soul Player without mocks.

## Overview

This test suite validates real audio playback quality by:
1. **Monitoring actual audio output** via virtual loopback devices
2. **Measuring timing** from user interaction to audio output
3. **Detecting quality issues** (stutters, gaps, false starts)
4. **Running in CI/CD** across all platforms

## Quick Start

### Prerequisites

Install a virtual audio device for your platform:

#### Linux
```bash
# Load snd-aloop kernel module
sudo modprobe snd-aloop

# Verify
aplay -l | grep Loopback
```

#### macOS
```bash
# Install BlackHole (free, open-source)
brew install blackhole-2ch

# Verify
system_profiler SPAudioDataType | grep BlackHole
```

#### Windows
```powershell
# Install VB-Cable (free version)
# Download from: https://vb-audio.com/Cable/
# Or via Chocolatey:
choco install vb-cable

# Verify
Get-AudioDevice -List | Select-String "CABLE"
```

### Running Tests

#### List Available Devices
```bash
cargo test --test audio_initialization_latency list_available_audio_devices -- --nocapture
```

Output example:
```
=== Available Audio Devices ===

Input Devices:
  [0] Built-in Microphone
  [1] BlackHole 2ch
      ^ Virtual device detected - suitable for testing
```

#### Run Initialization Latency Tests
```bash
# Linux
AUDIO_TEST_DEVICE="hw:Loopback,0,0" cargo test --test audio_initialization_latency

# macOS
AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test --test audio_initialization_latency

# Windows
$env:AUDIO_TEST_DEVICE="CABLE Input"
cargo test --test audio_initialization_latency
```

#### Run Stutter Detection Tests
```bash
# Same device configuration as above
AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test --test audio_stutter_detection
```

## Test Suite Structure

```
tests/e2e/
├── README.md                           # This file
├── audio_initialization_latency.rs    # Cold/warm start latency tests
├── audio_stutter_detection.rs         # Stutter, gap, false-start detection
└── common.rs                           # Shared test infrastructure (TODO)
```

## Test Categories

### 1. Initialization Latency Tests (`audio_initialization_latency.rs`)

**Validates**: Lazy initialization delay issue

Tests:
- `test_first_play_latency_cold_start` - Measures delay from app launch → first audio
- `test_warm_start_latency` - Measures delay after engine initialized
- `list_available_audio_devices` - Helper to find virtual devices

**Thresholds**:
- Cold start: < 1000ms (1 second)
- Warm start: < 100ms (imperceptible)

**Current Status**: ⚠️ Infrastructure ready, requires Tauri WebDriver integration

### 2. Stutter Detection Tests (`audio_stutter_detection.rs`)

**Validates**: Audio stutter and false-start issues

Tests:
- `test_phase_discontinuity_detection` - Unit test for discontinuity detection algorithm
- `test_silence_gap_detection` - Unit test for gap detection algorithm
- `test_false_start_detection` - Unit test for restart detection algorithm
- `test_audio_quality_analysis_clean_signal` - Validates detection on clean signal
- `test_no_stutter_at_playback_start_e2e` - E2E test (requires Tauri integration)

**Thresholds**:
- Stutters: 0 (zero tolerance)
- Silence gaps: 0 (zero tolerance)
- False starts: 0 (zero tolerance)
- Max discontinuity: < 0.5 magnitude

**Current Status**: ✅ Analysis algorithms complete, ⚠️ requires Tauri integration

## Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AUDIO_TEST_DEVICE` | Device name to use for testing | `"BlackHole 2ch"` |
| `AUDIO_E2E_REAL_DEVICE` | Enable real device testing (set to any value) | `"1"` |
| `AUDIO_TEST_TIMEOUT` | Test timeout in seconds | `"10"` |
| `AUDIO_METRICS_OUTPUT` | JSON output path for metrics | `"metrics.json"` |

### Example Configuration

```bash
# Full configuration
export AUDIO_TEST_DEVICE="BlackHole 2ch"
export AUDIO_E2E_REAL_DEVICE="1"
export AUDIO_TEST_TIMEOUT="15"
export AUDIO_METRICS_OUTPUT="target/audio-metrics.json"

cargo test --test audio_initialization_latency -- --nocapture
```

## CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/audio-e2e.yml
name: Audio E2E Tests

on: [push, pull_request]

jobs:
  audio-e2e-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install BlackHole
        run: brew install blackhole-2ch

      - name: Wait for audio device
        run: |
          for i in {1..30}; do
            if system_profiler SPAudioDataType | grep -q BlackHole; then
              echo "BlackHole ready"
              break
            fi
            echo "Waiting for BlackHole... ($i/30)"
            sleep 2
          done

      - name: List audio devices
        run: |
          cargo test --test audio_initialization_latency list_available_audio_devices -- --nocapture

      - name: Run audio E2E tests
        env:
          AUDIO_TEST_DEVICE: "BlackHole 2ch"
          AUDIO_E2E_REAL_DEVICE: "1"
          AUDIO_METRICS_OUTPUT: "target/audio-metrics.json"
        run: |
          cargo test --test audio_initialization_latency -- --nocapture
          cargo test --test audio_stutter_detection -- --nocapture

      - name: Upload metrics
        uses: actions/upload-artifact@v3
        with:
          name: audio-metrics
          path: target/audio-metrics.json
```

## Troubleshooting

### Device Not Found

**Problem**: `Device 'BlackHole 2ch' not found`

**Solution**:
```bash
# List all devices
cargo test --test audio_initialization_latency list_available_audio_devices -- --nocapture

# Use exact name from output
export AUDIO_TEST_DEVICE="exact device name from list"
```

### No Audio Detected

**Problem**: Test times out waiting for audio

**Possible causes**:
1. **Wrong device selected** - Verify device name matches exactly
2. **Loopback not configured** - Ensure virtual device routes output → input
3. **App not playing audio** - Check app logs for playback errors
4. **Volume muted** - Ensure system/app volume > 0

**Debugging**:
```bash
# Enable detailed logging
RUST_LOG=debug cargo test --test audio_initialization_latency -- --nocapture

# Check if device is receiving audio
# Linux:
arecord -D hw:Loopback,1,0 -f S16_LE -r 44100 -c 2 test.wav

# macOS:
rec -c 2 -r 44100 test.wav trim 0 5
```

### Tests Flaky in CI

**Problem**: Tests pass locally but fail in CI

**Common causes**:
1. **Resource contention** - CI runners are slower/busier
2. **Audio driver delays** - Virtual devices take time to initialize
3. **Timing assumptions** - Hard-coded sleep durations

**Solutions**:
```rust
// Increase timeouts for CI
let timeout = if std::env::var("CI").is_ok() {
    Duration::from_secs(10)
} else {
    Duration::from_secs(5)
};

// Poll for device readiness
for i in 0..30 {
    if device_is_ready() { break; }
    std::thread::sleep(Duration::from_secs(1));
}
```

### Permission Errors (Linux)

**Problem**: `Failed to open device: Permission denied`

**Solution**:
```bash
# Add user to audio group
sudo usermod -aG audio $USER

# Or run with sudo (not recommended for CI)
sudo cargo test --test audio_initialization_latency
```

## Test Asset Generation

Generate test audio files:

```bash
# Install sox (audio Swiss army knife)
brew install sox  # macOS
sudo apt install sox  # Linux
choco install sox  # Windows

# Run generation script
./scripts/generate-test-audio.sh
```

This creates:
- `tests/assets/1khz-sine-10s.wav` - Pure 1kHz sine wave for phase continuity testing
- `tests/assets/1khz-sine-30s.wav` - Extended test
- `tests/assets/distinctive-intro.wav` - Track with clear intro pattern for false-start detection
- `tests/assets/distinctive-intro-first-500ms.wav` - Reference pattern

## Development Workflow

### Adding New Tests

1. **Write the test**:
   ```rust
   #[test]
   fn test_new_audio_quality_check() {
       let config = AudioTestConfig::from_env();
       // ... test implementation
   }
   ```

2. **Test locally** with real device:
   ```bash
   AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test test_new_audio_quality_check -- --nocapture
   ```

3. **Add to CI** (in `.github/workflows/audio-e2e.yml`)

4. **Document** in this README

### Debugging Tests

Enable detailed logging:
```bash
RUST_LOG=debug,cpal=trace AUDIO_TEST_DEVICE="BlackHole 2ch" \
  cargo test --test audio_initialization_latency -- --nocapture
```

Save recorded audio for inspection:
```rust
// In test:
let recorded = recorder.stop_recording();
save_audio_to_wav(&recorded, 44100, "debug-output.wav");
```

Then analyze with audio tools:
```bash
# View waveform
audacity debug-output.wav

# Check for discontinuities
sox debug-output.wav -n stat
```

## Metrics & Monitoring

### Viewing Metrics

After test run:
```bash
cat target/audio-metrics.json
```

Example output:
```json
{
  "timestamp": "2026-02-11T10:30:00Z",
  "commit": "abc123",
  "metrics": {
    "play_to_audio_latency_ms": 450,
    "total_samples": 88200,
    "test_duration_ms": 2000
  }
}
```

### Tracking Over Time

Use the metrics to:
1. **Detect regressions** - Compare against baseline
2. **Track improvements** - Visualize latency trends
3. **Platform differences** - Compare across OS

Example tracking script:
```bash
#!/bin/bash
# scripts/track-audio-metrics.sh

DATE=$(date +%Y-%m-%d)
COMMIT=$(git rev-parse --short HEAD)

cargo test --test audio_initialization_latency
jq ".commit = \"$COMMIT\" | .date = \"$DATE\"" target/audio-metrics.json \
  >> metrics-history.jsonl
```

## Next Steps

### Phase 1: Complete Basic Infrastructure ✅
- [x] Audio monitoring with `cpal`
- [x] Phase discontinuity detection
- [x] Silence gap detection
- [x] False start detection

### Phase 2: Tauri Integration (In Progress)
- [ ] WebDriver setup for Soul Player
- [ ] Test track injection
- [ ] Play button automation
- [ ] Full E2E test execution

### Phase 3: CI/CD Rollout
- [ ] Linux CI configuration
- [ ] macOS CI configuration
- [ ] Windows CI configuration
- [ ] Metrics dashboard

### Phase 4: Extended Testing
- [ ] Buffer underrun stress tests
- [ ] CPU load impact tests
- [ ] Long-running stability tests
- [ ] Sample rate transition tests

## Resources

### Documentation
- [Main Testing Strategy](../../docs/AUDIO_E2E_TESTING_STRATEGY.md) - Comprehensive testing approach
- [Testing Guide](../../docs/TESTING.md) - General testing guidelines
- [Architecture](../../docs/ARCHITECTURE.md) - System architecture

### External Resources
- [Audio loopback testing (Android AOSP)](https://source.android.com/docs/compatibility/cts/audio-loopback-latency)
- [BlackHole - macOS virtual audio](https://github.com/ExistentialAudio/BlackHole)
- [VB-Cable - Windows virtual audio](https://vb-audio.com/Cable/)
- [cpal - Cross-platform audio I/O](https://github.com/RustAudio/cpal)
- [LatencyMon - Real-time audio testing](https://www.resplendence.com/latencymon)

### Related Code
- `libraries/soul-audio/tests/playback_glitch_test.rs` - Existing glitch detection tests
- `applications/desktop/src-tauri/src/playback_lazy.rs` - Lazy initialization implementation
- `libraries/soul-playback/src/manager.rs` - Core playback manager

---

**Maintained by**: Soul Player team
**Last updated**: 2026-02-11
**Questions?**: Open an issue or see [TESTING.md](../../docs/TESTING.md)
