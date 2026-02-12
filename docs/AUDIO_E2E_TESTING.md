# Audio E2E Testing Guide

This document describes the comprehensive Audio E2E testing infrastructure for Soul Player across all supported platforms.

## Overview

The Audio E2E test suite validates:
- Device hotplug detection and handling
- Playback integration with device changes
- Platform-specific audio API behavior
- Virtual device configuration and switching
- Performance characteristics of device monitoring

## CI/CD Integration

### Workflow: `audio-e2e-tests.yml`

The workflow runs automatically on:
- Pull requests to `main` or `develop`
- Pushes to `main` or `develop`
- Manual workflow dispatch

### Status Badge

Add this badge to your README.md:

```markdown
[![Audio E2E Tests](https://github.com/YOUR_USERNAME/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg)](https://github.com/YOUR_USERNAME/soul-player/actions/workflows/audio-e2e-tests.yml)
```

## Platform-Specific Setup

### Linux (Ubuntu)

**Virtual Device:** ALSA snd-aloop kernel module

**Configuration:**
```bash
# Load kernel module
sudo modprobe snd-aloop

# Verify devices
aplay -l
arecord -l
```

**ALSA Configuration:** The CI creates `~/.config/alsa/asoundrc` with loopback device configuration.

**Key Features:**
- Zero-latency loopback for testing
- Supports multiple sample rates
- No additional software installation required
- Works in GitHub Actions runners

**Limitations:**
- Requires kernel module support
- May not be available in all containerized environments
- Falls back to headless mode if module unavailable

### macOS

**Virtual Device:** BlackHole 2ch

**Installation:**
```bash
brew install blackhole-2ch
```

**Key Features:**
- Multi-channel support (2ch version used for CI)
- CoreAudio native integration
- Low latency performance
- Persistent across reboots

**Initialization:**
- 5-second wait after installation for device registration
- Verified via `system_profiler SPAudioDataType`

**Limitations:**
- Requires Homebrew
- First-time installation may need system permissions
- Device appears as "BlackHole 2ch" in system preferences

### Windows

**Virtual Device:** VB-Cable

**Installation:**
- Downloaded from VB-Audio website
- Installed silently via PowerShell
- 64-bit version for modern runners

**Key Features:**
- WASAPI and WDM support
- Zero-latency digital loopback
- Supports up to 192kHz sample rates
- Persistent after reboot

**Limitations:**
- May require administrator privileges for installation
- Installation can fail in restricted environments (tests continue in fallback mode)
- Requires ~10 seconds for driver initialization

**Fallback Mode:**
If VB-Cable installation fails, tests run with whatever default audio device is available, which may cause some tests to be skipped.

## Test Structure

### Test Files

1. **`device_hotplug_e2e.rs`** - Device monitoring and hotplug events
   - Enumeration performance
   - Hotplug notification latency
   - Device removal detection
   - Default device change detection
   - Watcher cleanup

2. **`playback_hotplug_integration_e2e.rs`** - Playback integration
   - Monitor + playback coexistence
   - Device events during playback
   - Sample rate checking
   - Concurrent operations
   - Edge cases (no devices, paused playback, etc.)

3. **`device_handling_test.rs`** - Device switching and error handling
   - Device validation
   - Invalid device handling
   - Sample rate mismatches

4. **Platform-specific tests:**
   - `platform_linux_test.rs` - ALSA-specific tests
   - `platform_macos_test.rs` - CoreAudio-specific tests
   - `platform_windows_test.rs` - WASAPI-specific tests

### Test Execution

**Build:**
```bash
cargo test --no-run --release \
  --package soul-audio-desktop \
  --test device_hotplug_e2e \
  --test playback_hotplug_integration_e2e \
  --test device_handling_test
```

**Run:**
```bash
cargo test --release \
  --package soul-audio-desktop \
  --test device_hotplug_e2e \
  --test playback_hotplug_integration_e2e \
  --test device_handling_test \
  -- --test-threads=1 --nocapture
```

**Single-threaded execution** (`--test-threads=1`) is critical because:
- Audio devices are global system resources
- Multiple tests accessing devices concurrently can cause conflicts
- Some platform APIs are not thread-safe for device enumeration

## Retry Logic

Tests run with automatic retry (3 attempts) to handle:
- CI environment flakiness
- Temporary device initialization delays
- Race conditions in device detection
- Platform-specific timing issues

**Retry Behavior:**
- Maximum 3 attempts per test run
- 5-second delay between retries
- Failure only occurs if all attempts fail
- Success on first attempt short-circuits

## Metrics Collection

### JSON Output Format

Each platform generates:
- `{platform}-metrics-raw.json` - Raw test output in JSON format
- `{platform}-summary.json` - Aggregated summary

**Summary Format:**
```json
{
  "platform": "linux|macos|windows",
  "runner": "ubuntu-latest|macos-latest|windows-latest",
  "virtual_device": "ALSA snd-aloop|BlackHole 2ch|VB-Cable",
  "timestamp": "2026-02-11T12:34:56Z",
  "test_result": "success|failure"
}
```

### Artifacts

**Retention:** 30 days

**Available Artifacts:**
- `audio-e2e-metrics-linux` - Linux test results
- `audio-e2e-metrics-macos` - macOS test results
- `audio-e2e-metrics-windows` - Windows test results
- `audio-e2e-metrics-combined` - All platforms aggregated

**Download:**
```bash
gh run download <run-id> -n audio-e2e-metrics-combined
```

## PR Integration

### Automated Comments

On pull requests, the workflow automatically posts a comment with results:

```markdown
## 🎵 Audio E2E Test Results

| Platform | Status |
|----------|--------|
| 🐧 Linux (ALSA) | ✅ Passed |
| 🍎 macOS (CoreAudio) | ✅ Passed |
| 🪟 Windows (WASAPI) | ✅ Passed |
```

### GitHub Step Summary

The workflow also generates a GitHub Actions step summary visible in the workflow run UI.

## Debugging

### View Logs

**Live logs during CI run:**
1. Go to Actions tab
2. Select "Audio E2E Tests" workflow
3. Click on the specific run
4. Expand platform job (Linux/macOS/Windows)
5. View step output

**Key sections to check:**
- "Setup virtual audio device" - Device configuration
- "Run audio E2E tests" - Test execution with retry attempts
- "Collect test metrics" - Metrics generation

### Local Reproduction

**Linux:**
```bash
# Setup (one-time)
sudo modprobe snd-aloop

# Run tests
cargo test --release --package soul-audio-desktop --test device_hotplug_e2e -- --nocapture
```

**macOS:**
```bash
# Setup (one-time)
brew install blackhole-2ch

# Run tests
cargo test --release --package soul-audio-desktop --test device_hotplug_e2e -- --nocapture
```

**Windows:**
```powershell
# Setup (one-time, requires admin)
# Download and install VB-Cable from https://vb-audio.com/Cable/

# Run tests
cargo test --release --package soul-audio-desktop --test device_hotplug_e2e -- --nocapture
```

### Common Issues

**Linux: "snd-aloop not available"**
- **Cause:** Kernel module not loaded or unavailable
- **Solution:** Tests fall back to headless mode automatically
- **Fix:** Run `sudo modprobe snd-aloop` manually

**macOS: "BlackHole not found"**
- **Cause:** Installation incomplete or device not initialized
- **Solution:** Wait 10 seconds after installation
- **Fix:** `brew reinstall blackhole-2ch && sleep 10`

**Windows: "VB-Cable installation failed"**
- **Cause:** Insufficient privileges or download failure
- **Solution:** Tests continue in fallback mode
- **Fix:** Install manually with admin privileges

**All Platforms: "Tests timeout"**
- **Cause:** Device initialization taking too long
- **Solution:** Retry logic handles this automatically
- **Fix:** Check if virtual device is properly installed

## Performance Expectations

### Device Enumeration

| Platform | Target | CI Typical |
|----------|--------|------------|
| macOS (CoreAudio) | < 50ms | ~30ms |
| Linux (PipeWire) | < 100ms | ~60ms |
| Windows (WinRT) | < 150ms | ~100ms |
| CPAL Fallback | < 1000ms | ~500ms |

### Hotplug Notification

| Platform | Real-time | Polling |
|----------|-----------|---------|
| macOS | < 100ms | N/A |
| Linux (PipeWire) | < 100ms | ~1000ms |
| Windows (WinRT) | < 100ms | N/A |
| CPAL Fallback | N/A | ~1000ms |

## Future Improvements

### Planned Enhancements

1. **Docker-based testing** - Testcontainers for isolated audio environments
2. **Performance regression tracking** - Historical metrics comparison
3. **Coverage reporting** - Audio-specific code coverage
4. **Stress testing** - Device switching under load
5. **Latency measurements** - Actual audio playback latency metrics

### Integration with Main CI

The audio E2E workflow is independent but can be integrated into the main CI via:

```yaml
# In .github/workflows/ci.yml
jobs:
  ci-success:
    needs:
      - format
      - clippy
      - build-and-test
      - audio-e2e  # Add reference
```

## Troubleshooting

### Workflow Fails to Start

**Check:**
- Workflow file syntax (YAML formatting)
- Branch protection rules
- Repository permissions for Actions

**Fix:**
```bash
# Validate YAML
yamllint .github/workflows/audio-e2e-tests.yml

# Check workflow status
gh workflow list
gh workflow view audio-e2e-tests.yml
```

### Tests Pass Locally But Fail in CI

**Common causes:**
- Different virtual device behavior
- Timing differences in CI runners
- Missing system dependencies
- Audio permissions in CI environment

**Debug:**
1. Check step output for device detection
2. Verify virtual device installation logs
3. Compare local vs CI device enumeration output
4. Run with `--nocapture` to see all debug output

### Metrics Not Generated

**Check:**
- JSON parsing errors in logs
- File permissions for `test-results/` directory
- Artifact upload step completed successfully

**Fix:**
```bash
# Manually generate metrics locally
cargo test --release --test device_hotplug_e2e -- -Z unstable-options --format json > metrics.json
```

## Contributing

When adding new audio E2E tests:

1. **Add test file** to `libraries/soul-audio-desktop/tests/`
2. **Update workflow** to include new test in build/run steps
3. **Document** expected behavior in test file comments
4. **Verify** tests pass on all three platforms locally
5. **Check** CI passes before merging

**Test Naming Convention:**
- `*_e2e.rs` - End-to-end integration tests
- `platform_*_test.rs` - Platform-specific tests
- `*_integration.rs` - Cross-component integration tests

---

**Last Updated:** 2026-02-11
**Workflow Version:** 1.0
**Platforms:** Linux, macOS, Windows
