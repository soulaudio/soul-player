# Soul Player E2E Test Suite

Complete end-to-end audio testing with Tauri WebDriver integration.

## Overview

This test suite validates real audio playback quality by:
1. Launching the Soul Player app with WebDriver
2. Injecting test tracks and controlling playback via UI automation
3. Monitoring actual audio output via virtual loopback devices
4. Analyzing audio quality (stutters, gaps, latency)

## Quick Start

### 1. Install Dependencies

#### Rust Dependencies
```bash
# tauri-driver (WebDriver for Tauri apps)
cargo install tauri-driver

# Build the Soul Player app first
cd applications/desktop
yarn tauri build
```

#### Virtual Audio Device

**macOS:**
```bash
brew install blackhole-2ch
# Restart audio services
sudo killall coreaudiod
```

**Linux:**
```bash
sudo modprobe snd-aloop
# Verify
aplay -l | grep Loopback
```

**Windows:**
```powershell
# Install VB-Cable from https://vb-audio.com/Cable/
# Or via Chocolatey:
choco install vb-cable
# Restart audio service after installation
```

#### Test Audio Files
```bash
# Linux/macOS
./scripts/generate-test-audio.sh

# Windows
.\scripts\generate-test-audio.ps1
```

### 2. List Available Devices

Find your virtual device name:
```bash
cargo test --test audio_initialization_latency list_available_audio_devices -- --nocapture
```

Example output:
```
Input Devices:
  [0] Built-in Microphone
  [1] BlackHole 2ch
      ^ Virtual device detected - suitable for testing
```

### 3. Run Tests

```bash
# macOS
AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test --package soul-player-e2e-tests

# Linux
AUDIO_TEST_DEVICE="hw:Loopback,0,0" cargo test --package soul-player-e2e-tests

# Windows PowerShell
$env:AUDIO_TEST_DEVICE="CABLE Input"
cargo test --package soul-player-e2e-tests
```

## Test Files

### E2E Tests
- `e2e/audio_initialization_latency.rs` - Measures playback latency
  - `test_cold_start_latency_with_tauri` - First play latency (< 1000ms)
  - `test_warm_start_latency_with_tauri` - Subsequent play latency (< 100ms)

- `e2e/audio_stutter_detection.rs` - Detects audio quality issues
  - `test_no_stutter_at_playback_start_e2e` - Validates smooth playback start
  - `test_no_false_start_with_tauri` - Detects song restarts

### Helper Modules
- `e2e/common/tauri_helper.rs` - WebDriver integration
  - `TauriDriver::new()` - Launch Soul Player with WebDriver
  - `load_test_track(path)` - Inject test audio
  - `click_play()` / `click_pause()` - UI automation
  - `wait_for_element(selector)` - Wait for UI elements
  - `get_playback_state()` - Query playback state

## Architecture

### Test Flow

```
Test Process          WebDriver          Soul Player App         Audio Output
     |                    |                       |                    |
     |-- launch_app() --->|                       |                    |
     |                    |--- spawn process ---->|                    |
     |                    |                       |                    |
     |-- wait_window() -->|                       |                    |
     |                    |<-- window ready ------|                    |
     |                    |                       |                    |
     |-- load_track() --->|-- invoke command ---->|                    |
     |                    |                       |-- load audio ----->|
     |                    |                       |                    |
     |-- start_monitor() -|---------------------->|                    |
     |                    |                       |                    |
     |-- click_play() --->|-- find element ------>|                    |
     |                    |-- click() ----------->|                    |
     |                    |                       |-- start playback ->|
     |                    |                       |                    |
     |<---------------- [monitor detects audio] ----------------------|
     |                    |                       |                    |
     |-- measure_latency -|                       |                    |
     |                    |                       |                    |
     |-- assert_quality ->|                       |                    |
```

### Component Layers

1. **Test Layer** (`audio_*_test.rs`)
   - Test orchestration
   - Assertions and metrics
   - Environment configuration

2. **WebDriver Layer** (`tauri_helper.rs`)
   - App lifecycle management
   - UI automation
   - State queries

3. **Audio Monitor Layer** (in test files)
   - Loopback recording
   - Signal analysis
   - Quality metrics

4. **Soul Player App**
   - Runs with `--webdriver` flag
   - Exposes test commands
   - Normal audio playback

## Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AUDIO_TEST_DEVICE` | Virtual device for monitoring | `"BlackHole 2ch"` |
| `AUDIO_E2E_REAL_DEVICE` | Force real device mode | `"1"` |
| `AUDIO_TEST_TIMEOUT` | Test timeout (seconds) | `"15"` |
| `AUDIO_METRICS_OUTPUT` | JSON metrics output path | `"metrics.json"` |
| `RUST_LOG` | Logging level | `"debug"` |

### Example Configuration

```bash
export AUDIO_TEST_DEVICE="BlackHole 2ch"
export AUDIO_E2E_REAL_DEVICE="1"
export AUDIO_TEST_TIMEOUT="20"
export RUST_LOG="debug,soul_player_e2e_tests=trace"

cargo test --package soul-player-e2e-tests -- --nocapture
```

## Troubleshooting

### App Binary Not Found

**Error:** `Soul Player binary not found`

**Solution:**
```bash
cd applications/desktop
yarn tauri build
# Or for debug build:
yarn tauri dev --build
```

### Device Not Found

**Error:** `Device 'BlackHole 2ch' not found`

**Solution:**
```bash
# List devices
cargo test --test audio_initialization_latency list_available_audio_devices -- --nocapture

# Use exact name from output
export AUDIO_TEST_DEVICE="exact device name"
```

### WebDriver Connection Failed

**Error:** `Failed to connect to tauri-driver`

**Solution:**
```bash
# Install tauri-driver
cargo install tauri-driver

# Check if it's in PATH
which tauri-driver  # Unix
where tauri-driver  # Windows

# If not found, add to PATH or specify full path in tests
```

### No Audio Detected

**Error:** `No audio detected after 5s`

**Possible causes:**
1. Virtual device not routing output → input
2. System audio muted
3. Wrong device selected
4. App not playing audio

**Debug steps:**
```bash
# Enable verbose logging
RUST_LOG=debug cargo test --test audio_initialization_latency -- --nocapture

# Test device with system tool
# macOS:
rec -c 2 -r 44100 test.wav trim 0 5

# Linux:
arecord -D hw:Loopback,1,0 -f S16_LE -r 44100 -c 2 test.wav
```

### Tests Timeout

**Error:** Tests timeout waiting for app

**Solution:**
```bash
# Increase timeout
export AUDIO_TEST_TIMEOUT="30"

# Check if app is slow to start
RUST_LOG=debug cargo test -- --nocapture

# Try running app manually first
cd applications/desktop
yarn tauri dev
```

## CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/e2e-audio-tests.yml
name: E2E Audio Tests

on: [push, pull_request]

jobs:
  e2e-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          brew install blackhole-2ch sox
          cargo install tauri-driver
          yarn install

      - name: Generate test audio
        run: ./scripts/generate-test-audio.sh

      - name: Build app
        run: |
          cd applications/desktop
          yarn tauri build

      - name: Wait for audio device
        run: |
          for i in {1..30}; do
            if system_profiler SPAudioDataType | grep -q BlackHole; then
              echo "BlackHole ready"
              break
            fi
            sleep 2
          done

      - name: Run E2E tests
        env:
          AUDIO_TEST_DEVICE: "BlackHole 2ch"
          AUDIO_E2E_REAL_DEVICE: "1"
          RUST_LOG: "info"
        run: |
          cargo test --package soul-player-e2e-tests -- --nocapture

      - name: Upload test artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-failures
          path: |
            target/debug/*.png
            target/*.log
```

## Development Workflow

### Writing New Tests

1. **Create test function:**
   ```rust
   #[tokio::test]
   #[serial_test::serial]  // Run serially to avoid device conflicts
   async fn test_my_new_feature() {
       let driver = TauriDriver::new().await?;
       driver.wait_for_window(Duration::from_secs(15)).await?;

       // Your test logic
   }
   ```

2. **Add to test file:**
   - Latency tests → `audio_initialization_latency.rs`
   - Quality tests → `audio_stutter_detection.rs`

3. **Test locally:**
   ```bash
   AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test test_my_new_feature -- --nocapture
   ```

4. **Add documentation:**
   - Update this README
   - Add docstrings to test functions

### Debugging Tests

#### Enable Verbose Logging
```bash
RUST_LOG=trace cargo test -- --nocapture
```

#### Take Screenshots on Failure
```rust
if let Err(e) = my_test_operation().await {
    driver.screenshot("debug-failure.png").await?;
    panic!("Test failed: {}", e);
}
```

#### Save Recorded Audio
```rust
let recorded = recorder.stop_recording();
save_audio_to_wav(&recorded, 44100, "debug-recording.wav")?;
```

#### Run Single Test
```bash
cargo test test_cold_start_latency_with_tauri -- --nocapture
```

## Metrics and Reporting

### Metrics Output

Tests can export metrics to JSON:
```bash
AUDIO_METRICS_OUTPUT="metrics.json" cargo test
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

### Tracking Performance Over Time

```bash
#!/bin/bash
# Track metrics history
DATE=$(date +%Y-%m-%d)
COMMIT=$(git rev-parse --short HEAD)

AUDIO_METRICS_OUTPUT="metrics.json" cargo test --package soul-player-e2e-tests

jq ".commit = \"$COMMIT\" | .date = \"$DATE\"" metrics.json >> metrics-history.jsonl
```

## Resources

### Documentation
- [E2E README](e2e/README.md) - Detailed test documentation
- [CLAUDE.md](../CLAUDE.md) - Project guidelines
- [TESTING.md](../docs/TESTING.md) - General testing guide

### External Resources
- [tauri-driver documentation](https://tauri.app/v1/guides/testing/webdriver/introduction/)
- [fantoccini WebDriver client](https://github.com/jonhoo/fantoccini)
- [BlackHole virtual audio](https://github.com/ExistentialAudio/BlackHole)
- [VB-Cable virtual audio](https://vb-audio.com/Cable/)
- [cpal audio I/O](https://github.com/RustAudio/cpal)

### Related Code
- Desktop app: `applications/desktop/src-tauri/`
- Playback manager: `libraries/soul-playback/`
- Audio desktop: `libraries/soul-audio-desktop/`

---

**Maintained by:** Soul Player team
**Last updated:** 2026-02-11
**Questions?** Open an issue or see the main [README](../README.md)
