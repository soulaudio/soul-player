# Device Switch Race Condition Tests

## Overview

This document describes the HIGH PRIORITY race condition tests for device switching in `soul-audio-desktop`. These tests are designed to catch synchronization bugs that could cause panics, deadlocks, or audio dropouts in production.

**Test File:** `libraries/soul-audio-desktop/tests/device_switch_race_conditions_test.rs`

## Running the Tests

```bash
# Run all race condition tests
cargo test -p soul-audio-desktop --test device_switch_race_conditions_test

# Run specific test
cargo test -p soul-audio-desktop --test device_switch_race_conditions_test test_rapid_device_switches

# Run with logging enabled
RUST_LOG=debug cargo test -p soul-audio-desktop --test device_switch_race_conditions_test -- --nocapture
```

## Test Coverage

### 1. Concurrent Device Switch + Hotplug Removal
**Test:** `test_concurrent_device_switch_and_removal`

**Scenario:**
- Start playback on initial device
- Perform multiple device switches in rapid succession
- Simulates the race condition where a device is removed (hotplug event) during a switch

**Expected Behavior:**
- All switches complete successfully or fail gracefully
- No panics or deadlocks
- Final device state is valid
- System remains functional

**Why This Matters:**
In production, users may unplug USB audio devices while the app is switching to a different device. This can cause mutex deadlocks if not handled correctly.

---

### 2. Device Switch During Active Playback
**Test:** `test_device_switch_during_playback`

**Scenario:**
- Start playback and let audio callbacks execute
- Switch device while audio callbacks are actively processing samples
- Verify playback position is preserved across the switch

**Expected Behavior:**
- Device switches successfully during playback
- Position does not jump backwards or forwards excessively (max 2 second drift allowed)
- Audio stream remains functional on new device
- No audio glitches or dropouts

**Why This Matters:**
Audio callbacks run on a separate thread with mutex locks. If device switching doesn't properly coordinate with the callback thread, it can cause audio buffer underruns or position loss.

---

### 3. Stream Mutex Recovery After Errors
**Test:** `test_stream_mutex_recovery_after_errors`

**Scenario:**
- Attempt to switch to an invalid device (should fail)
- Verify system recovers gracefully
- Attempt a valid device switch after the error

**Expected Behavior:**
- Invalid device switch returns error (does not panic)
- Original device remains functional after error
- System can successfully perform valid operations after error
- No mutex poisoning

**Why This Matters:**
If an error during device switching causes mutex poisoning, all subsequent operations will panic. This test ensures error paths don't poison shared state.

---

### 4. Rapid Device Switches (Stress Test)
**Test:** `test_rapid_device_switches`

**Scenario:**
- Load a track and start playback
- Send 10 device switch commands in quick succession (10ms apart)
- Track success/failure count

**Expected Behavior:**
- All switches complete within 5 seconds (no deadlock)
- System remains responsive
- Final device state is valid
- Playback controls still work after stress test

**Why This Matters:**
Users may rapidly click different output devices in settings UI. This stress test ensures the command queue and mutex locks don't deadlock under rapid commands.

---

### 5. Multi-Threaded Concurrent Device Switches
**Test:** `test_multi_threaded_device_switches`

**Scenario:**
- Spawn 4 threads
- Use a barrier to synchronize thread start
- All threads attempt to switch device simultaneously

**Expected Behavior:**
- No panics from any thread
- No deadlocks
- At least one thread succeeds (last write wins)
- System remains functional after concurrent operations

**Why This Matters:**
While the UI is single-threaded, background services (e.g., audio device monitoring) may trigger device switches concurrently with user actions. This test ensures thread-safety.

---

### 6. Device Switch During Track Transition
**Test:** `test_device_switch_during_track_transition`

**Scenario:**
- Configure 2-second crossfade between tracks
- Start playback and skip to next track
- Switch device mid-crossfade

**Expected Behavior:**
- Track transition completes correctly on new device
- No audio artifacts or pops
- Position continues to advance
- Crossfade completes smoothly

**Why This Matters:**
Crossfade involves complex buffer management with two active decoders. Switching devices during crossfade can expose buffer synchronization bugs.

---

### 7. Device Switch With Queue Modifications
**Test:** `test_device_switch_with_queue_modifications`

**Scenario:**
- Load initial tracks and start playback
- Simultaneously switch device AND add track to queue

**Expected Behavior:**
- Both operations complete successfully (or fail gracefully)
- Queue state remains consistent
- Device state remains valid
- No data races or corruption

**Why This Matters:**
Queue modifications and device switches both acquire locks on the playback manager. This test ensures operations can interleave safely.

---

## Key Race Conditions Tested

### 1. **Stream Mutex Deadlock**
- **Problem:** Audio callback thread holds stream lock while command thread waits
- **Tested By:** Tests 2, 4, 5
- **Detection:** Test timeout (5 second limit)

### 2. **Mutex Poisoning**
- **Problem:** Panic in one thread poisons mutex, causing all future operations to panic
- **Tested By:** Test 3
- **Detection:** Error recovery paths continue to work

### 3. **Channel Disconnection**
- **Problem:** Audio callback channel receiver dropped, causing command sends to fail
- **Tested By:** All tests implicitly
- **Detection:** `send_command()` returns error

### 4. **Position Loss/Drift**
- **Problem:** Device switch causes playback position to jump or reset
- **Tested By:** Test 2
- **Detection:** Position assertions before/after switch

### 5. **Buffer Underruns**
- **Problem:** Device switch during crossfade causes audio glitches
- **Tested By:** Test 6
- **Detection:** Position should advance smoothly

---

## Test Limitations

### Hardware Required
These tests require real audio hardware to run. On CI environments without audio devices, tests will skip gracefully with:
```
[TEST] Audio device not available in test environment (expected): ...
```

### Real Hotplug Events
Tests simulate concurrent operations but cannot trigger real hotplug events. The `test_concurrent_device_switch_and_removal` test approximates this scenario with rapid switches.

### Timing Sensitivity
Some tests use `tokio::time::sleep()` to create race condition windows. Results may vary on slower/faster machines.

---

## Adding New Tests

When adding new race condition tests, follow this pattern:

```rust
#[tokio::test]
async fn test_your_race_condition() {
    // Note: Logging initialized via RUST_LOG env var
    eprintln!("[TEST] Starting your test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Your test logic here

            // Always cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}
```

**Key Guidelines:**
1. Always use `match` on `DesktopPlayback::new()` result
2. Tests should pass even when no audio hardware is available
3. Use `eprintln!` for diagnostics (not `tracing::info!`)
4. Always cleanup with `Stop` command
5. Use assertions that clearly describe what failed

---

## Integration with CI

These tests are marked to run in CI, but will skip gracefully if no audio hardware is present. This allows:

1. **Local development:** Tests run on developer machines with audio devices
2. **CI validation:** Tests compile and run without panicking (even if they skip)
3. **Pre-release testing:** Can be run on real hardware before releases

---

## Related Files

- **Implementation:** `libraries/soul-audio-desktop/src/playback.rs` - `switch_device()` method
- **Device Management:** `libraries/soul-audio-desktop/src/device.rs`
- **Output Handling:** `libraries/soul-audio-desktop/src/output.rs`
- **Other Tests:** `libraries/soul-audio-desktop/tests/device_switching_test.rs` (basic tests)

---

## Debugging Failed Tests

If a test fails, check:

1. **Deadlock:** Did test timeout? Check mutex lock ordering in `switch_device()`
2. **Panic:** Look for mutex poisoning error messages
3. **Channel Disconnect:** Check if stream was dropped unexpectedly
4. **Position Drift:** Verify audio callback is running on new device

Enable detailed logging:
```bash
RUST_LOG=soul_audio_desktop=trace cargo test -p soul-audio-desktop --test device_switch_race_conditions_test -- --nocapture test_name
```

---

**Last Updated:** 2026-01-25
**Author:** Claude Sonnet 4.5
**Priority:** HIGH - These tests prevent critical production bugs
