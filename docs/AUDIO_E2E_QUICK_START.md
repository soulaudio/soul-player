# Audio E2E Testing - Quick Start

Get up and running with audio E2E tests in under 5 minutes.

## TL;DR

```bash
# Linux
sudo modprobe snd-aloop
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e

# macOS
brew install blackhole-2ch && sleep 5
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e

# Windows (PowerShell as Admin)
.\scripts\setup-virtual-audio.ps1
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e
```

## Platform Setup (One-Time)

### Linux (Ubuntu/Debian)

**1. Install dependencies:**
```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev alsa-utils
```

**2. Load virtual audio device:**
```bash
sudo modprobe snd-aloop
```

**3. Verify:**
```bash
aplay -l | grep Loopback
```

**Persist across reboots:**
```bash
echo 'snd-aloop' | sudo tee -a /etc/modules-load.d/audio-testing.conf
```

### macOS

**1. Install BlackHole:**
```bash
brew install blackhole-2ch
```

**2. Wait for initialization:**
```bash
sleep 5
```

**3. Verify:**
```bash
system_profiler SPAudioDataType | grep BlackHole
```

### Windows

**1. Download VB-Cable:**
- https://vb-audio.com/Cable/

**2. Install (as Administrator):**
```powershell
# Or use the automated script:
.\scripts\setup-virtual-audio.ps1
```

**3. Verify:**
```powershell
Get-CimInstance Win32_SoundDevice | Where-Object { $_.Name -like "*CABLE*" }
```

## Running Tests

### Quick Test (Single Test)

```bash
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e::test_enumeration_performance
```

### Full Suite

```bash
cargo test --release -p soul-audio-desktop \
  --test device_hotplug_e2e \
  --test playback_hotplug_integration_e2e \
  --test device_handling_test \
  -- --test-threads=1 --nocapture
```

### Platform-Specific Tests

```bash
# Linux only
cargo test --release -p soul-audio-desktop --test platform_linux_test

# macOS only
cargo test --release -p soul-audio-desktop --test platform_macos_test

# Windows only
cargo test --release -p soul-audio-desktop --test platform_windows_test
```

## Common Issues

### "No audio devices found"

**Solution:** Verify virtual device is installed:
- Linux: `aplay -l | grep Loopback`
- macOS: `system_profiler SPAudioDataType | grep BlackHole`
- Windows: Check Sound settings for "CABLE"

### "Tests timeout"

**Solution:** Increase timeout or run with fewer threads:
```bash
cargo test --release -- --test-threads=1 --nocapture
```

### "Permission denied" (Linux)

**Solution:** Run with sudo for module loading:
```bash
sudo modprobe snd-aloop
# Then run tests as normal user
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e
```

### "BlackHole not found" (macOS)

**Solution:** Wait longer after installation:
```bash
brew reinstall blackhole-2ch
sleep 10  # Wait longer
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e
```

## CI/CD Integration

### Status

Check workflow status:
```bash
gh workflow view audio-e2e-tests.yml
```

### Run Manually

Trigger workflow:
```bash
gh workflow run audio-e2e-tests.yml
```

### View Results

```bash
gh run list --workflow=audio-e2e-tests.yml
gh run view <run-id>
```

### Download Metrics

```bash
gh run download <run-id> -n audio-e2e-metrics-combined
```

## Debugging

### Enable Verbose Logging

```bash
RUST_LOG=debug cargo test --release -p soul-audio-desktop --test device_hotplug_e2e -- --nocapture
```

### Run Single Test with Backtrace

```bash
RUST_BACKTRACE=1 cargo test --release -p soul-audio-desktop --test device_hotplug_e2e::test_enumeration_performance -- --nocapture
```

### Check Virtual Device

**Linux:**
```bash
aplay -L  # List all PCM devices
cat ~/.config/alsa/asoundrc  # Check ALSA config
```

**macOS:**
```bash
system_profiler SPAudioDataType  # List all audio devices
```

**Windows:**
```powershell
Get-CimInstance Win32_SoundDevice | Format-Table  # List all audio devices
```

## Helper Scripts

### Automated Setup

**Linux/macOS:**
```bash
./scripts/setup-virtual-audio.sh
```

**Windows:**
```powershell
.\scripts\setup-virtual-audio.ps1
```

### Cleanup

**Linux/macOS:**
```bash
./scripts/setup-virtual-audio.sh linux cleanup  # or macos
```

**Windows:**
```powershell
.\scripts\setup-virtual-audio.ps1 -Cleanup
```

### Verify Installation

**All platforms:**
```bash
./scripts/setup-virtual-audio.sh verify         # Linux/macOS
.\scripts\setup-virtual-audio.ps1 -Verify       # Windows
```

## Performance Baselines

Expected test performance on modern hardware:

| Test | Linux | macOS | Windows |
|------|-------|-------|---------|
| Enumeration | ~60ms | ~30ms | ~100ms |
| Hotplug Start | ~100ms | ~50ms | ~150ms |
| Full Suite | ~30s | ~25s | ~40s |

If tests take significantly longer, check:
- Virtual device is properly installed
- No other audio applications using the device
- System resources (CPU/memory)

## Next Steps

- Full documentation: [docs/AUDIO_E2E_TESTING.md](./AUDIO_E2E_TESTING.md)
- Workflow definition: [.github/workflows/audio-e2e-tests.yml](../.github/workflows/audio-e2e-tests.yml)
- Test source code: `libraries/soul-audio-desktop/tests/`

## Getting Help

**GitHub Issues:**
- Search: https://github.com/YOUR_USERNAME/soul-player/issues
- Create: Use "Audio E2E Testing" label

**CI Failures:**
- Check workflow run logs
- Download artifacts for detailed metrics
- Compare with passing runs

**Local Issues:**
- Verify virtual device installation
- Check system audio permissions
- Run with verbose logging

---

**Last Updated:** 2026-02-11
