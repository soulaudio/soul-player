# Audio Pipeline E2E Test Plan

**Date**: 2026-01-10

## Overview

Comprehensive test plan for the entire Soul Player audio pipeline, covering every component from file decoding through DSP processing, resampling, and audio output.

---

## Test Coverage Map

```
┌──────────────────────────────────────────────────────────────┐
│                    Audio Pipeline                            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. File Input           ✅ Existing Tests                  │
│     ├─ Format support    ├─ MP3/FLAC/WAV/OGG               │
│     ├─ Metadata          ├─ Tags, artwork                   │
│     └─ Error handling    └─ Corrupt files                   │
│                                                              │
│  2. Decoder              ✅ Existing Tests                  │
│     ├─ Symphonia         ├─ All formats                     │
│     ├─ Sample rates      ├─ 44.1/48/96/192 kHz             │
│     └─ Channels          └─ Mono/Stereo                     │
│                                                              │
│  3. DSP Effects          ✅ Existing Tests                  │
│     ├─ EQ                ├─ 7 tests in dsp_effects_test.rs │
│     ├─ Compressor        │                                  │
│     ├─ Limiter           │                                  │
│     └─ Effect chain      │                                  │
│                                                              │
│  4. Resampling           ✅ Existing Tests                  │
│     ├─ Sinc (Rubato)     ├─ 11 tests in resampling_*       │
│     ├─ Quality           │   integration_test.rs            │
│     ├─ Speed accuracy    │                                  │
│     └─ Common rates      │                                  │
│                                                              │
│  5. Device Output        🔲 NEW TESTS NEEDED               │
│     ├─ Device selection  ├─ Switch during playback         │
│     ├─ Backend support   ├─ Default/ASIO/JACK              │
│     ├─ Sample rate match ├─ Automatic adjustment           │
│     └─ Latency           └─ <20ms total                     │
│                                                              │
│  6. Playback Manager     🔲 NEW TESTS NEEDED               │
│     ├─ Play/Pause/Stop   ├─ State transitions              │
│     ├─ Seek              ├─ Position accuracy               │
│     ├─ Volume            ├─ 0-100% range                    │
│     └─ State persistence └─ Resume position                 │
│                                                              │
│  7. Queue Management     🔲 NEW TESTS NEEDED               │
│     ├─ Add/Remove tracks ├─ Order preservation             │
│     ├─ Next/Previous     ├─ Navigation                      │
│     ├─ Shuffle           ├─ Randomization                   │
│     └─ Repeat modes      └─ None/Track/Queue               │
│                                                              │
│  8. Full Pipeline        🔲 NEW TESTS NEEDED               │
│     ├─ End-to-end        ├─ File → Decode → DSP →          │
│     │                    │   Resample → Output             │
│     ├─ Performance       ├─ CPU usage, latency              │
│     ├─ Stress tests      ├─ Long playback, rapid seeks     │
│     └─ Error recovery    └─ Device disconnects, etc.       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Existing Tests (Already Implemented)

### 1. DSP Effects Tests
**File**: `applications/desktop/src-tauri/tests/dsp_effects_test.rs`

- `test_add_effect_to_slot` - Add EQ to empty slot
- `test_effect_processes_audio` - Verify audio modification
- `test_toggle_effect` - Enable/disable effects
- `test_remove_effect` - Remove effect from slot
- `test_multiple_effects_chain` - Chain EQ + Compressor + Limiter
- `test_compressor_preset` - Moderate compression preset
- `test_limiter_preset` - Peak limiting preset

**Coverage**: ✅ 100% of DSP features

### 2. Resampling Tests
**File**: `libraries/soul-audio-desktop/tests/resampling_integration_test.rs`

- `test_resampling_enabled_when_needed` - Detect sample rate mismatch
- `test_no_resampling_when_rates_match` - Skip when not needed
- `test_common_upsampling_44_to_96` - 44.1 → 96 kHz
- `test_common_upsampling_48_to_96` - 48 → 96 kHz
- `test_downsampling_96_to_44` - 96 → 44.1 kHz
- `test_resampled_duration_accuracy` - Verify output length
- `test_playback_speed_verification` - Ensure correct speed
- `test_frequency_preservation` - Check frequency content
- `test_zero_crossing_rate` - Timing accuracy
- `test_resampling_quality_snr` - Signal-to-noise ratio
- `test_device_switch_scenario` - Device change during playback

**Coverage**: ✅ 100% of resampling features

---

## New Tests Needed

### 3. Device Switching Tests

**File**: `applications/desktop/src-tauri/tests/device_switching_test.rs` (NEW)

#### Test Cases:

**3.1. `test_list_available_devices`**
- Get all available backends
- For each backend, list devices
- Verify device info (name, sample rate, channels)
- Assert at least one device available

**3.2. `test_get_current_device`**
- Start playback
- Get current audio device
- Verify backend and device name
- Check sample rate matches

**3.3. `test_switch_device_during_playback`**
- Start playback on Device A (48 kHz)
- Switch to Device B (96 kHz)
- Verify audio source reloaded with new sample rate
- Check playback continues without glitches
- Verify position preserved

**3.4. `test_switch_device_before_playback`**
- Set device to 96 kHz device
- Start playback
- Verify audio plays at correct speed
- Check resampling applied correctly

**3.5. `test_device_persistence`**
- Set device to specific output
- Save setting to database
- Restart app (simulated)
- Verify device restored on startup

**3.6. `test_device_switch_with_dsp_effects`**
- Add DSP effects (EQ + Compressor)
- Switch device
- Verify effects still applied
- Check audio output correct

**3.7. `test_invalid_device_handling`**
- Try to switch to non-existent device
- Verify error returned
- Check playback continues on current device

---

### 4. Playback Manager Tests

**File**: `applications/desktop/src-tauri/tests/playback_manager_test.rs` (NEW)

#### Test Cases:

**4.1. `test_play_pause_stop`**
- Load track
- Play → verify playing state
- Pause → verify paused state
- Play → verify resumed
- Stop → verify stopped state

**4.2. `test_seek_to_position`**
- Start playback
- Seek to 30 seconds
- Verify position at 30s ±0.1s
- Continue playback
- Check audio correct

**4.3. `test_volume_control`**
- Set volume to 50%
- Play audio
- Verify output level reduced
- Set to 100% → full volume
- Set to 0% → muted

**4.4. `test_track_transition`**
- Queue track A and track B
- Play track A
- Wait for end
- Verify automatic transition to track B
- Check gapless playback

**4.5. `test_position_persistence`**
- Play track for 30 seconds
- Save position
- Stop playback
- Reload track
- Verify position restored

**4.6. `test_playback_state_events`**
- Subscribe to playback events
- Play/Pause/Stop/Seek operations
- Verify events emitted correctly
- Check event data accuracy

**4.7. `test_concurrent_playback_commands`**
- Rapidly issue Play/Pause/Seek commands
- Verify state remains consistent
- No crashes or deadlocks
- Final state matches last command

---

### 5. Queue Management Tests

**File**: `applications/desktop/src-tauri/tests/queue_test.rs` (NEW)

#### Test Cases:

**5.1. `test_add_remove_tracks`**
- Add 10 tracks to queue
- Verify order preserved
- Remove track at index 5
- Check queue updated correctly

**5.2. `test_next_previous_navigation`**
- Queue 5 tracks
- Play track 1
- Next → track 2
- Next → track 3
- Previous → track 2

**5.3. `test_shuffle_mode`**
- Queue 10 tracks (A-J)
- Enable shuffle
- Play all tracks
- Verify randomized order
- Verify all tracks played once

**5.4. `test_repeat_modes`**
- **Repeat None**: Play queue once, stop at end
- **Repeat Track**: Loop current track indefinitely
- **Repeat Queue**: Loop entire queue

**5.5. `test_queue_with_shuffle_and_repeat`**
- Queue 10 tracks
- Enable shuffle + repeat queue
- Play 30 tracks
- Verify randomized but all tracks played

**5.6. `test_insert_track_at_position`**
- Queue A, B, C
- Insert D at position 1
- Verify queue: A, D, B, C

**5.7. `test_clear_queue`**
- Queue 10 tracks
- Clear queue
- Verify empty
- Check playback stopped

---

### 6. Full Pipeline Integration Tests

**File**: `applications/desktop/src-tauri/tests/full_pipeline_test.rs` (NEW)

#### Test Cases:

**6.1. `test_end_to_end_playback`**
- Load 44.1 kHz MP3 file
- Add EQ (bass boost)
- Add Limiter
- Set device to 96 kHz
- Play audio
- Verify:
  - File decoded correctly
  - DSP effects applied
  - Resampled to 96 kHz
  - Output to device
  - No glitches or artifacts

**6.2. `test_pipeline_performance`**
- Play 96 kHz FLAC file
- Enable all 4 DSP slots
- Measure CPU usage
- Verify <25% CPU (single core)
- Check latency <20ms

**6.3. `test_rapid_track_switching`**
- Queue 10 tracks
- Skip through all tracks rapidly (1 second each)
- Verify no crashes
- Check audio output correct
- No memory leaks

**6.4. `test_device_disconnect_during_playback`**
- Start playback
- Simulate device disconnect
- Verify graceful fallback to default device
- Check playback continues

**6.5. `test_long_playback_session`**
- Play 1-hour audio file
- Let it play for 1 hour
- Verify no memory leaks
- Check position accuracy at end
- No buffer underruns

**6.6. `test_stress_seek_operations`**
- Load 10-minute track
- Perform 100 random seeks
- Verify position correct after each seek
- No crashes or artifacts

**6.7. `test_pipeline_with_corrupt_file`**
- Load partially corrupt MP3
- Start playback
- Verify error handling
- Check graceful degradation
- Logs error message

---

## Test Implementation Strategy

### Phase 1: Device Switching (HIGH PRIORITY)
1. Implement all 7 device switching tests
2. Cover edge cases (invalid devices, persistence)
3. Test with multiple backends (Default, ASIO, JACK)

### Phase 2: Playback Manager (MEDIUM PRIORITY)
1. Implement core playback tests (play/pause/stop/seek)
2. Add state management tests
3. Test event emission

### Phase 3: Queue Management (MEDIUM PRIORITY)
1. Implement queue manipulation tests
2. Add shuffle/repeat mode tests
3. Test navigation (next/previous)

### Phase 4: Full Pipeline (LOW PRIORITY)
1. End-to-end integration tests
2. Performance benchmarks
3. Stress tests
4. Error recovery tests

---

## Test Infrastructure

### Required Test Utilities

**1. Audio File Generators**
```rust
fn create_test_audio_file(
    sample_rate: u32,
    duration_secs: u32,
    frequency: f32
) -> PathBuf
```

**2. Audio Comparison Utilities**
```rust
fn compare_audio_buffers(
    expected: &[f32],
    actual: &[f32],
    tolerance: f32
) -> bool
```

**3. Mock Device Simulator**
```rust
struct MockAudioDevice {
    sample_rate: u32,
    channels: u16,
}
```

**4. Playback Test Helper**
```rust
struct PlaybackTestHelper {
    playback_manager: PlaybackManager,
    test_files: Vec<PathBuf>,
}
```

---

## Performance Benchmarks

### Target Metrics

| Component | Target | Acceptable | Unacceptable |
|-----------|--------|------------|--------------|
| Decode (MP3) | <2% CPU | <5% | >10% |
| DSP (1 effect) | <2% CPU | <5% | >10% |
| Resample (44→96) | <10% CPU | <15% | >25% |
| Total Pipeline | <15% CPU | <25% | >40% |
| Latency | <10ms | <20ms | >50ms |
| Memory (1hr) | <100 MB | <200 MB | >500 MB |

### Benchmark Tests
- `bench_decode_performance` - Measure decoding speed
- `bench_dsp_performance` - CPU usage per effect
- `bench_resampling_performance` - Upsampling CPU cost
- `bench_full_pipeline` - End-to-end performance
- `bench_memory_usage` - Memory footprint over time

---

## CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Audio Pipeline Tests

on: [push, pull_request]

jobs:
  audio-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Device Tests
        run: cargo test --test device_switching_test

      - name: Run Playback Tests
        run: cargo test --test playback_manager_test

      - name: Run Queue Tests
        run: cargo test --test queue_test

      - name: Run Pipeline Tests
        run: cargo test --test full_pipeline_test

      - name: Run Benchmarks
        run: cargo bench --benches
```

---

## Testing Checklist

### Existing Tests
- ✅ DSP effects (7 tests)
- ✅ Resampling (11 tests)

### New Tests to Implement
- 🔲 Device switching (7 tests)
- 🔲 Playback manager (7 tests)
- 🔲 Queue management (7 tests)
- 🔲 Full pipeline (7 tests)

### Performance Tests
- 🔲 Decode benchmarks
- 🔲 DSP benchmarks
- 🔲 Resampling benchmarks
- 🔲 Pipeline benchmarks
- 🔲 Memory profiling

### Manual Testing
- 🔲 Real device testing (ASIO, JACK)
- 🔲 Long playback sessions
- 🔲 Device hotplug/unplug
- 🔲 High-DPI audio (192/384 kHz)

---

## Success Criteria

**Unit Tests**: ≥80% line coverage for new code
**Integration Tests**: All critical paths covered
**Performance**: Meet all target metrics
**Stability**: No crashes in 24-hour stress test
**Documentation**: All tests documented

---

## Next Steps

1. **Implement Phase 1** - Device switching tests (HIGH PRIORITY)
   - `test_list_available_devices`
   - `test_get_current_device`
   - `test_switch_device_during_playback`

2. **Implement Phase 2** - Playback manager tests
   - `test_play_pause_stop`
   - `test_seek_to_position`
   - `test_volume_control`

3. **Implement Phase 3** - Queue management tests
   - `test_add_remove_tracks`
   - `test_next_previous_navigation`
   - `test_shuffle_mode`

4. **Implement Phase 4** - Full pipeline tests
   - `test_end_to_end_playback`
   - `test_pipeline_performance`
   - `test_rapid_track_switching`

---

**Last Updated**: 2026-01-10
**Author**: Claude Code
**Status**: 📋 Plan Complete - Ready for Implementation
