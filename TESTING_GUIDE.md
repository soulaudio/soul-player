# Testing Guide

## Overview

This guide covers both automated and manual testing for the Soul Player audio playback fixes.

## Automated Tests

### Running All Tests

```bash
# Run all tests (from project root)
cargo test --all-features

# Run specific test suites
cargo test --package soul-audio-desktop --test resampling_integration_test
cargo test --package soul-player-desktop --test dsp_effects_test
```

### DSP Effects Tests

**Location**: `applications/desktop/src-tauri/tests/dsp_effects_test.rs`

Tests the DSP effect chain functionality:

```bash
# Run DSP tests (requires effects feature)
cd applications/desktop/src-tauri
cargo test --features effects dsp_effects

# Individual tests
cargo test --features effects test_add_effect_to_slot
cargo test --features effects test_effect_processes_audio
cargo test --features effects test_toggle_effect
cargo test --features effects test_multiple_effects_chain
```

**What's tested:**
- ✅ Effects can be added to slots
- ✅ Effects actually modify audio (not just UI)
- ✅ Effects can be toggled on/off
- ✅ Effects can be removed
- ✅ Multiple effects work together in chain
- ✅ Effect presets (Compressor, Limiter, EQ)

### Sample Rate Resampling Tests

**Location**: `libraries/soul-audio-desktop/tests/resampling_integration_test.rs`

Tests sample rate conversion and playback speed:

```bash
# Run resampling tests
cd libraries/soul-audio-desktop
cargo test resampling

# Individual tests
cargo test test_resampling_enabled_when_needed
cargo test test_common_sample_rate_conversions
cargo test test_resampled_duration_accuracy
cargo test test_playback_speed_verification
```

**What's tested:**
- ✅ Resampling enabled when sample rates differ
- ✅ No resampling when rates match
- ✅ Common conversions (44.1→96kHz, 48→96kHz, etc.)
- ✅ Duration accuracy after resampling
- ✅ Playback speed is correct
- ✅ Frequency preservation
- ✅ Timing accuracy (zero-crossing rate)
- ✅ Device switch scenario

## Manual Testing

### 1. Sample Rate Mismatch Fix

#### Test Scenario: Audio Playing Too Fast

**Setup:**
1. Use an audio device set to 96kHz (like Audient EVO8)
2. Play a 44.1kHz or 48kHz audio file

**Steps:**
1. Start the application
2. Select your 96kHz audio device (Settings → Audio → Output Device)
3. Play any MP3/FLAC file (usually 44.1kHz or 48kHz)

**Expected Console Output:**
```
[LocalAudioSource] File info:
  - Path: /path/to/song.mp3
  - Source sample rate: 44100 Hz
  - Target sample rate: 96000 Hz
  - Needs resampling: true
  - Speed ratio: 0.4594x
```

**Validation:**
- ✅ Audio plays at **normal speed** (not fast or slow)
- ✅ Console shows "Needs resampling: true"
- ✅ Target sample rate matches device (96000 Hz)
- ✅ Vocals sound natural (not chipmunk-like)

#### Test Scenario: Device Switch

**Steps:**
1. Start playback on Device A (e.g., 48kHz)
2. Switch to Device B (e.g., 96kHz) via Settings
3. Continue playback

**Expected Console Output:**
```
[DesktopPlayback] Switching device: backend=Default, device="Some Device"
[DesktopPlayback] Reloading audio source for new sample rate
[LocalAudioSource] File info:
  - Target sample rate: 96000 Hz
  - Needs resampling: true
[DesktopPlayback] Audio source reloaded with sample rate: 96000
```

**Validation:**
- ✅ Playback continues without crashing
- ✅ Audio speed remains correct after switch
- ✅ Position is preserved
- ✅ Console shows audio source reload

### 2. DSP Effects Fix

#### Test Scenario: Add EQ Effect

**Steps:**
1. Open Settings → DSP Effects
2. Click "Add Effect" → Choose "Parametric EQ"
3. Set Bass Boost preset (+6dB at 100Hz)
4. Click "Add to Chain"

**Expected Behavior:**
- ✅ Effect appears in effect chain UI (Slot 0: EQ - Enabled)
- ✅ Bass frequencies are noticeably louder
- ✅ Console shows: `[add_effect_to_chain] Slot 0: effect added`

#### Test Scenario: Toggle Effect

**Steps:**
1. With EQ effect added (bass boost)
2. Click the toggle button to disable
3. Click again to re-enable

**Validation:**
- ✅ When **enabled**: Bass is boosted
- ✅ When **disabled**: Bass returns to normal
- ✅ Toggle state shown in UI
- ✅ Audio changes immediately (no restart needed)

#### Test Scenario: Multiple Effects Chain

**Steps:**
1. Add EQ (Bass Boost) to Slot 0
2. Add Compressor (Moderate preset) to Slot 1
3. Add Limiter (-1dB threshold) to Slot 2
4. Play audio

**Expected Console Output:**
```
[add_effect_to_chain] Slot 0: effect added
[add_effect_to_chain] Slot 1: effect added
[add_effect_to_chain] Slot 2: effect added
```

**Validation:**
- ✅ All three effects show in chain UI
- ✅ Audio sounds compressed and limited (dynamics reduced)
- ✅ Bass is boosted (EQ)
- ✅ Peaks are controlled (Limiter)

#### Test Scenario: Remove Effect

**Steps:**
1. Add an effect to any slot
2. Click "Remove" button on that slot
3. Continue playback

**Validation:**
- ✅ Effect disappears from UI
- ✅ Audio returns to unprocessed state
- ✅ Console shows: `[remove_effect_from_chain] Slot X: effect removed`

### 3. Combined Scenario

#### Test: Effects + Resampling Together

**Steps:**
1. Use 96kHz device (Audient EVO8)
2. Play 44.1kHz file
3. Add EQ bass boost
4. Verify both work together

**Validation:**
- ✅ Audio plays at correct speed (resampling works)
- ✅ Bass is boosted (effects work)
- ✅ No audio glitches or crackling
- ✅ Console shows both resampling and effect logs

## Test Results Checklist

### Before Fix
- ❌ Audio plays too fast with 96kHz device
- ❌ DSP effects don't show in UI after adding
- ❌ Effects don't modify audio
- ❌ Device switching causes speed issues

### After Fix
- ✅ Audio plays at correct speed regardless of device sample rate
- ✅ Effects show in UI immediately after adding
- ✅ Effects actually process audio
- ✅ Effects can be toggled/removed
- ✅ Device switching maintains correct playback speed
- ✅ Multiple effects work together in chain

## Troubleshooting Tests

### If Tests Fail

**Build Issues:**
```bash
# Clean build
cargo clean
rm -rf target/
cargo build --all-features

# Or on Windows:
Remove-Item -Recurse -Force .\target
cargo build --all-features
```

**Missing Audio Device:**
```
Skipping test - no audio device: Device not found
```
This is normal in CI/headless environments. Tests will skip gracefully.

**Sample Rate Not Logged:**
Check that you're running with the latest code:
```bash
git pull
cargo build --all-features
```

## Performance Validation

### Resampling Performance

**Expected CPU usage** (44.1kHz → 96kHz):
- Fast preset: ~5% CPU (single core)
- Balanced preset: ~10% CPU
- High preset: ~15% CPU

**Latency:**
- Total audio latency: <20ms (including resampling + effects)

### Effect Chain Performance

**Per effect:**
- EQ (3-band): ~2% CPU
- Compressor: ~3% CPU
- Limiter: ~2% CPU
- Total chain (3 effects): <10% CPU

## Test Coverage

### Automated Tests
- **DSP Effects**: 7 tests
- **Resampling**: 11 tests
- **Total**: 18 end-to-end tests

### Manual Tests
- **Sample Rate**: 2 scenarios (mismatch, device switch)
- **DSP Effects**: 4 scenarios (add, toggle, chain, remove)
- **Combined**: 1 scenario

## CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: |
          cargo test --all-features
          cargo test --package soul-audio-desktop --test resampling_integration_test
          cargo test --package soul-player-desktop --test dsp_effects_test --features effects
```

## Reporting Issues

If tests fail or you encounter issues:

1. **Capture console output** (full logs from app startup)
2. **Note audio device specs** (name, sample rate, channels)
3. **List steps to reproduce**
4. **Include test results**: `cargo test --all-features 2>&1 | tee test-output.log`
5. **System info**: OS version, audio driver version

**Example Issue Report:**
```
Title: Resampling test fails on 192kHz device

Environment:
- OS: Windows 11
- Device: MOTU M4 (192kHz, 2ch)
- Rust: 1.75.0

Steps:
1. cargo test test_playback_speed_verification
2. Test fails with "Duration mismatch"

Console output:
[LocalAudioSource] Target sample rate: 192000 Hz
Error: Duration mismatch at 192000Hz: expected 3.000s ± 0.150s, got 3.250s

Logs attached: test-output.log
```

---

**Last Updated**: 2026-01-10
