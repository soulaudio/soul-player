# Circuit Breaker for Track Loading

## Overview

The circuit breaker prevents playback from getting stuck on repeatedly failing tracks. It implements a Spotify-inspired fault tolerance pattern with exponential backoff and automatic recovery.

## Strategy

The circuit breaker operates in three states:

### 1. Closed (Normal Operation)
- Track loading is allowed
- Failures are monitored

### 2. Open (Too Many Failures)
- Track loading is blocked
- Playback is paused
- System waits 30 seconds before attempting recovery

### 3. HalfOpen (Testing Recovery)
- One track load attempt is allowed
- Success → transition to Closed
- Failure → return to Open

## Failure Thresholds

### Consecutive Failures
- **3 consecutive failures on same track** → skip to next track
- Prevents infinite retries on corrupted/missing files

### Window Failures
- **10 failures in 60 seconds** → open circuit, pause playback
- Prevents thrashing when many tracks are broken
- Protects against filesystem/network issues

## Exponential Backoff

Retries use increasing delays:
1. 1st retry: immediate
2. 2nd retry: 1 second
3. 3rd retry: 2 seconds
4. 4th+ attempt: skip to next track

## Usage

### Platform Integration

When track loading fails:

```rust
// In platform code (e.g., desktop/src-tauri/src/playback.rs)
let should_skip = manager.record_track_load_failure();

if should_skip {
    // Too many consecutive failures - skip to next track
    manager.next()?;
} else {
    // Wait for backoff delay, then retry loading
    // The backoff delay is already applied internally
    tokio::time::sleep(Duration::from_millis(100)).await;
    retry_load_track();
}
```

### Successful Load

When track loads successfully:

```rust
// In PlaybackManager::set_audio_source (already integrated)
manager.set_audio_source(source);
// Circuit breaker automatically records success
```

### User-Initiated Actions

Reset circuit breaker when user manually selects a track:

```rust
// Give previously failing tracks a fresh chance
manager.reset_circuit_breaker();
manager.play()?;
```

### Recovery from Open Circuit

Periodically check if circuit should attempt recovery:

```rust
// In a background task (e.g., every second)
if manager.should_retry_after_circuit_open() {
    // Circuit breaker is now in HalfOpen state
    // Attempt to resume playback
    manager.play()?;
}
```

## Events

The circuit breaker emits three event types:

### CircuitOpened
```rust
PlaybackEvent::CircuitOpened {
    failures: 10,      // Number of failures in window
    window_secs: 60,   // Window duration
}
```

Emitted when circuit opens due to too many failures. UI should:
- Display error message to user
- Suggest checking file paths / network connection
- Provide manual retry button

### CircuitClosed
```rust
PlaybackEvent::CircuitClosed
```

Emitted when circuit closes after successful recovery. UI should:
- Clear error messages
- Resume normal playback display

### TrackSkippedDueToFailures
```rust
PlaybackEvent::TrackSkippedDueToFailures {
    track_id: "track_123",
    consecutive_failures: 3,
}
```

Emitted when a track is skipped after consecutive failures. UI should:
- Show notification (e.g., "Skipped unplayable track")
- Optionally log track ID for debugging

## Configuration

Constants defined in `circuit_breaker.rs`:

```rust
const CONSECUTIVE_FAILURE_THRESHOLD: u32 = 3;      // Skip after N failures
const WINDOW_FAILURE_THRESHOLD: u32 = 10;          // Open circuit after N failures
const FAILURE_WINDOW: Duration = Duration::from_secs(60);  // Failure window
const OPEN_TO_HALFOPEN_TIMEOUT: Duration = Duration::from_secs(30);  // Recovery wait

const BACKOFF_DELAYS: [Duration; 3] = [
    Duration::from_secs(0),  // 1st retry
    Duration::from_secs(1),  // 2nd retry
    Duration::from_secs(2),  // 3rd retry
];
```

## Testing

Run circuit breaker tests:

```bash
cd libraries/soul-playback
cargo test --lib circuit_breaker
```

Test coverage:
- ✅ Consecutive failure skip logic
- ✅ Window failure circuit opening
- ✅ Success resets consecutive counter
- ✅ Track change resets consecutive counter
- ✅ HalfOpen → Closed transition
- ✅ Backoff delays
- ✅ State resets
- ✅ Window expiry

## Example Scenarios

### Scenario 1: Single Corrupted Track
1. Track fails to load (1st attempt) → retry immediately
2. Track fails again (2nd attempt) → retry after 1s
3. Track fails again (3rd attempt) → skip to next track
4. Next track loads successfully → circuit stays Closed

### Scenario 2: Filesystem Issue
1. Multiple tracks fail rapidly (different tracks)
2. After 10 failures in 60s → circuit opens
3. Playback stops, user sees error
4. After 30s → circuit transitions to HalfOpen
5. Platform attempts to load one track
6. If successful → circuit closes, playback resumes
7. If failed → circuit reopens for another 30s

### Scenario 3: User Manual Intervention
1. Circuit is Open due to many failures
2. User clicks retry button
3. Platform calls `reset_circuit_breaker()`
4. Circuit state → Closed
5. Playback resumes with fresh failure counters

## Implementation Notes

- Circuit breaker state is NOT persisted across app restarts
- Failure counters reset on successful track load
- Track ID changes reset consecutive failure counter
- Window expiry (60s) resets window failure counter
- Circuit state transitions are logged via `tracing` crate

## Future Enhancements

Potential improvements (not currently implemented):

1. **Configurable thresholds** - allow users to adjust sensitivity
2. **Per-source tracking** - different thresholds for local vs streaming
3. **Failure reason tracking** - different strategies for different error types
4. **Persistent state** - remember problematic tracks across restarts
5. **Adaptive backoff** - adjust delays based on failure patterns
