# Queue Navigation E2E Tests

Comprehensive end-to-end tests for Soul Player's queue navigation, rewind, and next/previous logic.

## Overview

Located: `libraries/soul-audio-desktop/tests/queue_navigation_e2e_test.rs`

**Test Philosophy:**
- ✅ Uses **real audio files** from `test_data/` directory
- ✅ Tests **timing-sensitive navigation** with actual audio playback
- ✅ Covers **edge cases** and **combinations** of modes
- ❌ **No mocks** - ensures accurate behavior testing

## Test Categories

### 1. Rewind/Previous Logic Tests (7 tests)

Tests the 3-second threshold behavior for the "previous" button:

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_previous_within_3_seconds_goes_to_previous_track` | Press previous < 3s into track | Go to previous track |
| `test_previous_after_3_seconds_restarts_current_track` | Press previous > 3s into track | Restart current track |
| `test_previous_at_beginning_of_queue` | Press previous on first track | Restart first track |
| `test_rapid_previous_presses` | Rapid previous (3x) | Navigate backwards reliably |

**Critical Timing:**
- **< 3 seconds**: Navigate to previous track in history
- **> 3 seconds**: Restart current track from beginning
- **No history**: Always restart current track

### 2. Next Track Logic Tests (4 tests)

Tests forward navigation through the queue:

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_next_advances_through_queue` | Press next multiple times | Advance through all tracks in order |
| `test_next_at_end_of_queue_stops` | Press next at end (no repeat) | Stop playback |
| `test_rapid_next_presses` | Rapid next (10x) | Handle gracefully, no crashes |

### 3. Loop Mode: Off Tests (1 test)

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_loop_off_stops_at_end` | Queue ends with repeat off | Playback stops |

### 4. Loop Mode: All Tests (2 tests)

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_loop_all_wraps_to_beginning` | Queue ends with repeat all | Wrap to first track |
| `test_loop_all_multiple_cycles` | Skip through 2+ cycles | Cycle indefinitely |

### 5. Loop Mode: One Tests (2 tests)

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_loop_one_repeats_current_track` | Press next with repeat one | Restart same track |
| `test_loop_one_with_previous` | Press previous with repeat one | Restart current track |

**Note:** Loop one repeats the current track on next/previous, not the queue.

### 6. Shuffle Mode Tests (3 tests)

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_shuffle_off_maintains_order` | Play with shuffle off | Tracks play in original order |
| `test_shuffle_random_does_not_crash` | Play with shuffle random | Plays without crashing (order varies) |
| `test_shuffle_random_with_loop_all` | Shuffle + loop all | Cycles through shuffled queue indefinitely |

### 7. Edge Cases & Combinations (9 tests)

Complex navigation patterns and edge cases:

| Test | Scenario | Expected Behavior |
|------|----------|-------------------|
| `test_previous_then_next_restores_position` | Next → Previous → Next | Navigate correctly through history |
| `test_mixed_next_previous_navigation` | Complex pattern (N, N, P, N, P, P) | Handle all navigation correctly |
| `test_loop_one_does_not_affect_manual_navigation` | Manual nav with loop one | Loop one only affects auto-advance |
| `test_empty_queue_navigation` | Next/previous on empty queue | Handle gracefully, no crash |
| `test_single_track_queue_with_loop_off` | Single track, press next | Stop after track |
| `test_single_track_queue_with_loop_all` | Single track, press next (loop all) | Restart same track |
| `test_pause_resume_preserves_navigation_state` | Pause → Resume → Previous | Preserve navigation history |
| `test_rewind_bug_reproduction` | Next → immediate Previous (repeat) | Navigate backwards correctly, no skips |

**Bug Reproduction Test:**
`test_rewind_bug_reproduction` specifically targets the reported issue where "rewind seems to skip song or something". It tests the pattern that most likely triggers the bug:
1. Play track 1
2. Press next → track 2
3. Immediately press previous (< 3s) → should go back to track 1
4. Press next → track 2
5. Immediately press previous → should go back to track 1 (not skip)

## Running the Tests

### Run All Navigation Tests

```bash
# Via xtask (recommended)
cargo xtask test audio e2e

# Or directly
cd libraries/soul-audio-desktop
cargo test --test queue_navigation_e2e_test -- --include-ignored
```

### Run Specific Test

```bash
cd libraries/soul-audio-desktop
cargo test --test queue_navigation_e2e_test test_previous_within_3_seconds_goes_to_previous_track -- --include-ignored
```

### Run by Category

```bash
# All rewind/previous tests
cargo test --test queue_navigation_e2e_test previous -- --include-ignored

# All loop mode tests
cargo test --test queue_navigation_e2e_test loop -- --include-ignored

# All shuffle tests
cargo test --test queue_navigation_e2e_test shuffle -- --include-ignored
```

## Test Data Requirements

**Audio Files:**
- Location: `libraries/soul-audio-desktop/test_data/`
- Files needed:
  - `track_1.wav` (10 seconds, 1kHz sine wave)
  - `track_2.wav` (30 seconds, 1kHz sine wave)

**Generate test files:**
```bash
cd libraries/soul-audio-desktop
cargo run --bin generate_test_audio_rust
```

## Test Configuration

All tests use `#[ignore]` attribute - they require real audio hardware and won't run in CI by default.

**Why?**
- Real audio playback is needed to test timing-sensitive behavior
- Tests validate actual state transitions and audio system behavior
- Mocking would miss timing bugs (e.g., the 3-second threshold)

## Common Test Patterns

### 1. Helper Functions

```rust
// Create test track with real audio file
fn create_test_track(id: &str) -> QueueTrack

// Drain all events from playback system
fn drain_events(playback: &DesktopPlayback) -> Vec<PlaybackEvent>

// Get latest state from events
fn get_latest_state(events: &[PlaybackEvent]) -> Option<PlaybackState>

// Get latest track from events
fn get_latest_track(events: &[PlaybackEvent]) -> Option<String>
```

### 2. Test Structure

```rust
#[test]
#[ignore = "Requires real audio hardware"]
fn test_name() {
    // 1. Setup playback with config
    let playback = DesktopPlayback::new(config).unwrap();

    // 2. Load tracks
    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback.send_command(PlaybackCommand::LoadPlaylist(tracks)).unwrap();

    // 3. Start playback
    playback.send_command(PlaybackCommand::Play).unwrap();
    std::thread::sleep(Duration::from_millis(150)); // Wait for init

    // 4. Perform test actions
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150)); // Wait for command

    // 5. Assert results
    let events = drain_events(&playback);
    let state = get_latest_state(&events);
    assert_eq!(state, Some(PlaybackState::Playing));
}
```

### 3. Timing Considerations

**Critical delays:**
- **150ms after LoadPlaylist/Play**: Wait for audio initialization
- **100-150ms after Next/Previous**: Wait for fade completion
- **3500ms**: Wait to exceed 3-second rewind threshold
- **50ms**: Rapid command interval (< 3s threshold)

## Known Behaviors

### Previous Button Logic

```
Position in track < 3s:
  → Go to previous track (if history exists)
  → Or restart current track (if no history)

Position in track >= 3s:
  → Always restart current track
```

### Loop Modes

| Mode | Next at End | Previous at Start | Auto Track End |
|------|-------------|-------------------|----------------|
| Off | Stop | Restart current | Stop |
| All | Wrap to first | Restart current | Go to next (wrap) |
| One | Restart current | Restart current | Restart current |

### History Management

- History is maintained as a stack
- `next()` pushes current track to history
- `previous()` pops from history
- History preserved across pause/resume
- History cleared on new playlist load

## Troubleshooting

### Tests fail with "audio device not found"

Audio tests require real audio hardware. They're marked `#[ignore]` for CI.

Run with: `cargo test --test queue_navigation_e2e_test -- --include-ignored`

### "File not found" errors

Generate test audio files:
```bash
cd libraries/soul-audio-desktop
cargo run --bin generate_test_audio_rust
```

### Tests timeout or hang

Check that:
1. No other audio applications are using the device
2. Audio device is not in exclusive mode
3. Sufficient sleep delays between commands (150ms)

### Flaky test results

If tests are flaky:
1. Increase sleep delays (especially after LoadPlaylist)
2. Check audio device latency settings
3. Ensure system is not under heavy load

## Maintenance

**When to update these tests:**

1. **Rewind threshold changes**: Update 3-second delays if threshold is modified
2. **New loop modes added**: Add new test category
3. **Shuffle algorithm changes**: Update shuffle tests to match new behavior
4. **Navigation logic refactor**: Run full test suite to catch regressions

**Test health check:**
```bash
# Run full navigation test suite
cargo xtask test audio e2e

# Should see: 23 queue navigation tests passed
```

## Related Documentation

- [Audio E2E Testing Strategy](./AUDIO_E2E_TESTING_STRATEGY.md)
- [Audio E2E Quick Start](./AUDIO_E2E_QUICK_START.md)
- [Test Organization](./TEST_ORGANIZATION.md)
- [Dev Workflow](./DEV_WORKFLOW.md)

## Summary

**Total Tests: 23**

- 4 Rewind/Previous logic tests
- 3 Next track logic tests
- 1 Loop off test
- 2 Loop all tests
- 3 Loop one tests
- 3 Shuffle tests
- 7 Edge case & combination tests

**Coverage:**
- ✅ 3-second rewind threshold
- ✅ Next/previous navigation
- ✅ All loop modes (Off, All, One)
- ✅ All shuffle modes (Off, Random, Smart)
- ✅ Empty queue handling
- ✅ Single track queue
- ✅ Pause/resume preservation
- ✅ Rapid command handling
- ✅ Bug reproduction scenarios

**Test Quality:**
- Real audio files (no mocks)
- Timing-sensitive validation
- Edge case coverage
- Combination scenarios
- Bug reproduction tests
