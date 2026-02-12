# Comprehensive Test Suite

This directory contains stress tests, endurance tests, and edge case tests for the Soul Player audio playback system.

## Test Categories

### 1. Lock Contention Stress Tests (`lock_contention_stress_test.rs`)

Tests the playback system under extreme concurrent load:

- **test_extreme_command_flood**: 8 threads sending different commands simultaneously
  - Thread 0: Rapid play/pause cycles
  - Thread 1: Skip next/prev rapidly
  - Thread 2: Volume changes
  - Thread 3: Queue modifications
  - Threads 4-7: State queries
  - **Duration**: 5 seconds
  - **Success Criteria**: No deadlocks, >50% command success rate

- **test_concurrent_state_queries**: 16 threads querying state + 1 thread sending commands
  - **Duration**: 3 seconds
  - **Success Criteria**: >30,000 total queries (>10,000 queries/sec)

- **test_rapid_playlist_changes**: 4 threads modifying playlist concurrently
  - **Duration**: 3 seconds
  - **Success Criteria**: System remains stable

### 2. Endurance Stress Tests (`endurance_stress_test.rs`)

Tests long-running stability and memory management:

- **test_continuous_playback_1_hour**: Full 1-hour endurance test
  - **Duration**: 60 minutes
  - **Workload**: Track cycling, pause/resume, volume changes
  - **Success Criteria**: Memory growth <10MB/hour, system responsive

- **test_continuous_playback_5_min**: Quick endurance validation
  - **Duration**: 5 minutes
  - **Workload**: Rapid cycling to stress test in shorter time
  - **Success Criteria**: Memory growth <5MB, system responsive

- **test_rapid_track_cycling**: Decoder/buffer cleanup validation
  - **Duration**: 2 minutes
  - **Workload**: Skip track every 500ms (~240 skips)
  - **Success Criteria**: No decoder resource leaks, memory growth <5MB

### 3. Corrupted File Recovery Tests (`corrupted_file_recovery_test.rs`)

Tests error handling for invalid/corrupted audio files:

- **test_truncated_wav_file**: WAV with valid header but truncated data
- **test_zero_byte_file**: Completely empty file
- **test_invalid_header**: File with random garbage data
- **test_missing_file_during_playback**: File deleted between queue and playback
- **test_recovery_after_corrupted_file**: Skip to next valid track after error
- **test_playlist_of_corrupted_files**: Multiple corrupted files in sequence

**Success Criteria**: Graceful error handling, system remains responsive

## Performance Benchmarks

### Criterion Benchmarks (`benches/playback_latency_benchmark.rs`)

- **cold_start**: Initialization latency
- **command_latency**: Play, pause, volume, skip commands
- **event_poll**: Event polling latency
- **playlist_loading**: 10, 100, 1000 track playlists
- **command_burst_10**: Rapid command throughput
- **queue_operations**: Add/remove from queue

## Running Tests

### Via xtask (Recommended)

```bash
# Lock contention tests
cargo xtask test stress contention --verbose

# Endurance tests (5 min)
cargo xtask test stress endurance --duration 5 --verbose

# Endurance tests (1 hour)
cargo xtask test stress endurance --duration 60 --verbose

# Corrupted file tests
cargo xtask test stress corrupted --verbose

# Performance benchmarks
cargo xtask test stress bench --verbose

# Run all stress tests (quick suite, excludes 1-hour test)
cargo xtask test stress all --verbose
```

### Direct Cargo Commands

```bash
# Lock contention tests
cargo test --package soul-audio-desktop --test lock_contention_stress_test -- --include-ignored --nocapture

# Endurance tests (all)
cargo test --package soul-audio-desktop --test endurance_stress_test -- --include-ignored --nocapture

# Endurance tests (specific)
cargo test --package soul-audio-desktop --test endurance_stress_test test_continuous_playback_5_min -- --include-ignored --nocapture
cargo test --package soul-audio-desktop --test endurance_stress_test test_continuous_playback_1_hour -- --include-ignored --nocapture

# Corrupted file tests
cargo test --package soul-audio-desktop --test corrupted_file_recovery_test -- --include-ignored --nocapture

# Benchmarks
cargo bench --package soul-audio-desktop --bench playback_latency_benchmark
```

## CI Integration

Stress tests run automatically via GitHub Actions:

- **Schedule**: Every Sunday at 3 AM UTC
- **Workflow**: `.github/workflows/stress-tests.yml`
- **Platforms**: Ubuntu, Windows, macOS
- **Tests**: Lock contention, 5-min endurance, corrupted files, benchmarks
- **Artifacts**: Benchmark results uploaded for 30 days

### Manual CI Trigger

You can manually trigger the workflow with custom duration:
1. Go to GitHub Actions → Weekly Stress Tests
2. Click "Run workflow"
3. Select duration (5 or 60 minutes)

## Test Data

Tests use WAV files from `libraries/soul-audio-desktop/test_data/`:
- `track_1.wav`: 1 kHz sine wave (30 seconds)
- `track_2.wav`: 1 kHz sine wave (30 seconds)

Generate new test data:
```bash
cargo xtask test audio generate-assets
```

## Memory Monitoring

Endurance tests track memory usage:
- **Windows**: Uses `GetProcessMemoryInfo` (Working Set)
- **Linux**: Reads `/proc/self/statm` (RSS)
- **macOS**: Fallback to basic monitoring

**Thresholds**:
- 1-hour test: <10MB growth
- 5-minute test: <5MB growth
- Rapid cycling: <5MB growth

## Interpreting Results

### Lock Contention Tests
- **Good**: >90% success rate, no deadlocks
- **Warning**: 50-90% success rate (investigate contention)
- **Bad**: <50% success rate or deadlocks

### Endurance Tests
- **Good**: Memory stable, system responsive throughout
- **Warning**: Memory grows linearly but within limits
- **Bad**: Memory leak detected (exceeds thresholds)

### Corrupted File Tests
- **Good**: All errors caught, system remains stable
- **Warning**: Some errors not propagated correctly
- **Bad**: System crashes or locks up

### Benchmarks
- **Compare**: Against previous runs (Criterion tracks history)
- **Thresholds** (typical values):
  - Cold start: <200ms
  - Command latency: <1ms
  - Event poll: <0.1ms
  - Playlist load (100 tracks): <10ms

## Troubleshooting

### Windows: Memory monitoring fails
- Ensure `windows` crate is available in dev-dependencies
- Tests will continue with memory checks disabled

### Linux: Permission errors
- Install audio dependencies: `sudo apt-get install libasound2-dev`

### macOS: Code signing issues
- Tests don't require signing (dev builds)
- If issues persist, check `codesign -dv` output

### Tests time out
- Increase timeout in workflow (default: 75 min for 1-hour test)
- Check for actual deadlocks vs. slow hardware

## Future Enhancements

Potential additions:
- [ ] Property-based testing for queue operations
- [ ] Fuzzing for decoder input
- [ ] Network streaming stress tests
- [ ] GPU/hardware acceleration stress tests
- [ ] Multi-device simultaneous playback
