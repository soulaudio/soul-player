# FLAC Stutter Detection Testing

## Problem

The file `D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac` stutters at the start of playback.

## Test Suite

Location: `libraries/soul-audio-desktop/tests/flac_stutter_detection_e2e_test.rs`

### Test 1: Direct Source Analysis (Recommended First)

**What it does:**
- Loads the FLAC file directly through `LocalAudioSource`
- Reads and analyzes the first 500ms of decoded audio
- Detects amplitude discontinuities, pops, clicks, and gaps
- Provides detailed analysis with visualizations

**How to run:**
```bash
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_stutter_detection_direct_source -- --include-ignored --nocapture
```

**What to look for:**
- `🔴 STUTTER DETECTED!` - Test fails with detailed analysis
- `✅ NO STUTTER DETECTED` - Audio starts cleanly

**Output includes:**
- RMS level analysis
- Maximum amplitude jumps (in dB)
- Number of large discontinuities
- Visual waveform (ASCII art)
- Potential causes if stutter detected

### Test 2: Full Playback System

**What it does:**
- Tests the complete playback pipeline (including audio output)
- Plays the file through `DesktopPlayback`
- Monitors state transitions and errors
- Requires manual listening verification

**How to run:**
```bash
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_stutter_detection_full_playback -- --include-ignored --nocapture
```

**What to listen for:**
- Pops/clicks at the very start (0-100ms)
- Brief silence followed by sudden audio
- Glitchy/stuttering sound

### Test 3: Resampling Comparison

**What it does:**
- Compares native sample rate (44.1kHz) vs resampled (48kHz)
- Determines if resampling introduces artifacts

**How to run:**
```bash
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test test_flac_compare_with_resampling -- --include-ignored --nocapture
```

**What it reveals:**
- If resampling makes stutter worse → resampler issue
- If both have stutter → decoder or source file issue
- If neither has stutter → intermittent/system-specific issue

## Detection Methods

### 1. Amplitude Jump Analysis
- Detects sudden volume changes (> 0.2 = -14dB)
- Indicates pops, clicks, or encoder delay artifacts

### 2. Silence Gap Detection
- Finds silence (< -60dB) followed by sudden onset (> -20dB)
- Indicates buffer underruns or prebuffering issues

### 3. RMS Energy Analysis
- Measures overall energy in first 500ms
- Low RMS + sudden jumps = stutter pattern

### 4. Waveform Visualization
- ASCII graph of amplitude over time
- Visual inspection for anomalies

## Thresholds (Configurable)

```rust
// In the test file:
const AMPLITUDE_JUMP_THRESHOLD: f32 = 0.2;    // -14dB
const SILENCE_THRESHOLD: f32 = 0.001;          // -60dB
const SUDDEN_ONSET_THRESHOLD: f32 = 0.1;       // -20dB
```

Adjust these if too sensitive/not sensitive enough.

## Expected Output (Stutter Detected)

```
╔════════════════════════════════════════════════════════════╗
║  FLAC Stutter Detection Test (Direct Source)              ║
╚════════════════════════════════════════════════════════════╝

[TEST] Loading FLAC file: D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac
✓ FLAC file loaded successfully
  Duration: 195.23s

[TEST] Reading first 500ms of audio...
✓ Read 44100 samples (500.0ms)

[ANALYSIS] Analyzing for stutters, pops, and clicks...

┌─ Analysis Results ─────────────────────────────────────┐
│ RMS Level:        0.045321 (-26.9dB)
│ Max Jump:         0.342156 (-9.3dB) at sample 1245
│ Large Jumps:      7 (threshold: 0.20)
│ Silence Gap:      YES ⚠️
│ Sudden Onset:     12.4ms (amplitude: 0.156)
└────────────────────────────────────────────────────────┘

[WAVEFORM] Amplitude over time (25ms windows):
[WAVEFORM] Scale: █ = high, ▓ = medium, ░ = low, . = silence
[WAVEFORM]     0ms |  -67.3dB | .
[WAVEFORM]    25ms |  -42.1dB | ░░░░░░
[WAVEFORM]    50ms |  -18.5dB | ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
[WAVEFORM]    75ms |  -15.2dB | ████████████████████

[VERDICT]
🔴 STUTTER DETECTED!

Potential causes:
  • Large amplitude discontinuity (-9.3dB jump at sample 1245)
    → Possible encoder delay not skipped
    → Possible decoder startup artifact
  • Silence gap followed by sudden onset
    → Possible buffer underrun
    → Possible prebuffering issue
  • Multiple large jumps detected (7)
    → Possible decoding glitches
    → Possible resampling artifacts
```

## Expected Output (Clean Start)

```
[VERDICT]
✅ NO STUTTER DETECTED
Audio starts cleanly with no artifacts.
```

## Troubleshooting

### File Not Found
```
⚠️  FLAC file not found: D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac
This test requires the specific file to be present.
```
→ Update `FLAC_FILE_PATH` constant in the test file to your actual path.

### Test Passes But You Hear Stuttering
Possible causes:
1. **Timing issue**: Stutter only occurs in real-time playback, not in analysis
   - Run Test 2 (full playback) to verify
2. **System-specific**: Audio driver or hardware issue
   - Check logs in `%APPDATA%\Soul Player\logs\`
3. **Intermittent**: Buffer underrun under load
   - Run test multiple times

### Test Fails But You Don't Hear Stuttering
Possible causes:
1. **Too sensitive**: Lower thresholds in test
2. **Natural dynamics**: Song has quiet intro
   - Check waveform visualization
3. **Encoder delay**: Normal artifact, already being skipped in playback
   - Compare with Test 2 (should sound clean)

## Next Steps After Detection

### If Encoder Delay Issue
- Check `ENCODER_DELAY_FRAMES` in `libraries/soul-audio-desktop/src/sources/local_audio_source.rs`
- May need format-specific delay values (MP3 vs FLAC)
- See `encoder_delay_skip_test.rs` for reference

### If Buffer Underrun
- Check `MIN_BUFFER_SAMPLES` in playback config
- Increase prebuffering time
- Check CPU usage during playback

### If Resampling Artifacts
- Test with native device sample rate
- Check resampler configuration
- May need higher quality resampling settings

### If Decoder Issue
- Try re-encoding the file
- Check for corrupted frames in source
- Test with different decoder (FFmpeg vs Symphonia)

## Related Files

- `libraries/soul-audio-desktop/src/sources/local_audio_source.rs` - Audio source implementation
- `libraries/soul-audio-desktop/tests/encoder_delay_skip_test.rs` - Encoder delay tests
- `libraries/soul-audio-desktop/tests/pause_during_startup_e2e_test.rs` - Playback state tests
- `libraries/soul-playback/src/manager.rs` - Playback manager

## Quick Commands

```bash
# Run all stutter detection tests
cd libraries/soul-audio-desktop
cargo test --test flac_stutter_detection_e2e_test -- --include-ignored --nocapture

# Run specific test
cargo test --test flac_stutter_detection_e2e_test test_flac_stutter_detection_direct_source -- --include-ignored --nocapture

# Compare with encoder delay tests
cargo test --test encoder_delay_skip_test -- --include-ignored --nocapture
```

---

**Last Updated**: 2026-02-11
