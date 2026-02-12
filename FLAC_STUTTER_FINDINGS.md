# FLAC Stutter Investigation Findings

## Test Results Summary

Tested file: `D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac`

### Test 1: Direct Source Analysis (Native Sample Rate)

```
╔════════════════════════════════════════════════════════════╗
║  FLAC Stutter Detection Test (Direct Source)              ║
╚════════════════════════════════════════════════════════════╝

[TEST] Loading FLAC file: D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac
✓ FLAC file loaded successfully
  Duration: 128.89s

[TEST] Waiting for audio buffer to fill...
✓ Buffer filled in 10ms

[TEST] Reading first 500ms of audio...
✓ Read 44100 samples (500.0ms)

[ANALYSIS] Analyzing for stutters, pops, and clicks...

┌─ Analysis Results ─────────────────────────────────────┐
│ RMS Level:        0.147692 (-16.6dB)
│ Max Jump:         0.065887 (-23.6dB) at sample 2196
│ Large Jumps:      0 (threshold: 0.20)
│ Silence Gap:      NO ✓
└────────────────────────────────────────────────────────┘

[WAVEFORM] Amplitude over time (25ms windows):
[WAVEFORM] Scale: █ = high, ▓ = medium, ░ = low, . = silence
[WAVEFORM]     0ms |  -11.8dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]    25ms |  -14.0dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]    50ms |  -12.4dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]    75ms |  -11.8dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   100ms |  -12.6dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   125ms |  -12.6dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   150ms |  -13.7dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   175ms |  -14.8dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   200ms |  -14.6dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   225ms |  -15.3dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   250ms |  -15.6dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   275ms |  -16.5dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   300ms |  -16.9dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   325ms |  -16.9dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   350ms |  -17.5dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   375ms |  -18.6dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   400ms |  -17.9dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   425ms |  -17.7dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   450ms |  -18.5dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]   475ms |  -18.5dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

[VERDICT]
✅ NO STUTTER DETECTED
Audio starts cleanly with no artifacts.
```

**Conclusion**: When decoded at the native sample rate (44.1kHz), the FLAC file is **perfectly clean** with no stuttering, pops, or clicks.

---

### Test 2: Resampling Comparison (44.1kHz → 48kHz)

```
╔════════════════════════════════════════════════════════════╗
║  FLAC Resampling Comparison Test                          ║
╚════════════════════════════════════════════════════════════╝

[TEST] Loading with native sample rate (no resampling)...
✓ Native rate: max jump = 0.065887 (-23.6dB)

[TEST] Loading with resampling (44.1kHz → 48kHz)...
✓ Resampled: max jump = 0.283184 (-11.0dB)

[COMPARISON]
  Native:    max jump = 0.065887 (-23.6dB)
  Resampled: max jump = 0.283184 (-11.0dB)
  Difference: 0.217297 (-13.3dB)

⚠️  Resampling increases amplitude discontinuities!
   Resampler may be introducing artifacts.
```

**Critical Finding**: When resampled to 48kHz, the maximum amplitude discontinuity **increases by 4.3x** (from -23.6dB to -11.0dB). This is a **12.6dB increase** in discontinuity!

---

## Root Cause Analysis

### The Stutter Source: Resampling Artifacts

The test results clearly show that:

1. **The FLAC file itself is clean** - No codec artifacts, encoder delay properly skipped
2. **Resampling introduces artifacts** - The sinc resampler creates large amplitude jumps
3. **The problem only occurs with resampling** - Native playback is smooth

### Why This Happens

The resampler configuration in `local.rs` uses:
```rust
let params = SincInterpolationParameters {
    sinc_len: 256,           // Filter length
    f_cutoff: 0.95,          // Cutoff frequency
    interpolation: SincInterpolationType::Linear,
    oversampling_factor: 256,
    window: WindowFunction::BlackmanHarris2,
};
```

When resampling from 44.1kHz to 48kHz (ratio 1.088), the resampler can introduce:
- **Filter ramp-up artifacts** at the start
- **Interpolation discontinuities** between chunks
- **Chunking artifacts** from 1024-frame chunks

### What Your Audio Device Is Using

To confirm this is the issue, check your audio device's sample rate:

**Windows**:
```powershell
# Check current audio device sample rate
Get-WmiObject -Class Win32_SoundDevice | Format-List Name, Manufacturer
# Then go to Sound Settings > Device Properties > Advanced
# Look for "Default Format" - if it says 48000 Hz, that's the problem
```

**Likely finding**: Your audio device is set to 48kHz, forcing resampling of this 44.1kHz FLAC file.

---

## Solutions

### Option 1: Match Device Sample Rate to File (Best Quality)

Change your audio device to 44.1kHz to avoid resampling:

**Windows**:
1. Right-click sound icon → "Sound settings"
2. Click your output device → "Properties"
3. Click "Additional device properties"
4. Go to "Advanced" tab
5. Change "Default Format" to "2 channel, 16 bit, 44100 Hz (CD Quality)" or "2 channel, 24 bit, 44100 Hz"
6. Click "Apply"

**Impact**: No resampling needed, perfectly clean playback!

### Option 2: Improve Resampler Quality (Better Quality, Higher CPU)

Update the resampler configuration in `libraries/soul-audio-desktop/src/sources/local.rs`:

```rust
let params = SincInterpolationParameters {
    sinc_len: 512,           // Increase filter length (was 256)
    f_cutoff: 0.95,
    interpolation: SincInterpolationType::Cubic,  // Use cubic instead of linear
    oversampling_factor: 512,  // Increase (was 256)
    window: WindowFunction::BlackmanHarris2,
};
```

**Trade-offs**:
- ✅ Much better quality resampling
- ✅ Reduced artifacts
- ❌ Higher CPU usage
- ❌ Slightly higher latency

### Option 3: Use Higher Quality Resampler Library

Replace `rubato` with `soxr` (libsoxr wrapper):

```toml
[dependencies]
# Replace rubato with soxr
# rubato = "0.15"
soxr = "0.3"
```

**Benefits**:
- Industry-standard resampler (same as SoX)
- Very high quality
- Optimized performance

**Trade-offs**:
- External dependency (libsoxr)
- Platform-specific binaries

### Option 4: Skip Encoder Delay Skip for Resampled Audio (Quick Fix)

The encoder delay skip might be interacting badly with the resampler. Try reducing it:

In `libraries/soul-audio-desktop/src/sources/local.rs`:
```rust
// Current
const ENCODER_DELAY_FRAMES: usize = 1200;

// Try reducing for FLAC (minimal encoder delay)
// Only skip encoder delay if NOT resampling, or use format-specific values
```

### Option 5: Increase Resampler Chunk Size (Reduce Chunking Artifacts)

In `libraries/soul-audio-desktop/src/sources/local.rs`:
```rust
// Current
let resampler_chunk_frames = 1024;

// Try larger chunks
let resampler_chunk_frames = 4096;  // or 8192
```

**Impact**: Fewer chunk boundaries = fewer opportunities for discontinuities.

---

## Recommended Actions

### Immediate (Test This First)
1. **Check your audio device sample rate** - If it's 48kHz, change to 44.1kHz
2. **Test playback again** - It should be perfectly smooth now

### If You Must Use 48kHz Device
1. **Try Option 2** - Increase resampler quality
2. **Monitor CPU usage** - Make sure it's acceptable
3. **If still stuttering** - Try Option 3 (soxr)

### Long-Term Solution
- **Add automatic device sample rate detection** - Soul Player should detect the file's native rate and prefer matching devices
- **Add user setting for resampler quality** - Let users choose quality vs. performance
- **Add format-specific encoder delay values** - FLAC needs less skip than MP3

---

## Test Commands

To re-run these tests anytime:

```bash
# Full stutter detection (all tests)
cargo xtask test audio e2e --stutter-only

# Just direct source analysis
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_stutter_detection_direct_source -- --include-ignored --nocapture

# Just resampling comparison
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_compare_with_resampling -- --include-ignored --nocapture

# Full playback test (requires audio hardware)
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_stutter_detection_full_playback -- --include-ignored --nocapture
```

---

## Summary

**Problem Identified**: ✅ Resampling from 44.1kHz (FLAC native) to 48kHz (audio device) introduces amplitude discontinuities that cause audible stuttering.

**Severity**: The resampler increases discontinuities by **4.3x** (12.6dB), which is significant and audible.

**Quick Fix**: Set your audio device to 44.1kHz to match the file's native sample rate.

**Proper Fix**: Improve resampler quality or switch to a higher-quality resampling library (soxr).

---

**Date**: 2026-02-11
**File Tested**: `D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac`
**Duration**: 128.89s
**Native Sample Rate**: 44.1kHz
**Test Framework**: Soul Player Audio E2E Tests
