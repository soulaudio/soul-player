# Performance Benchmarking Guide

Comprehensive performance benchmarks for the Soul Player playback system using Criterion.rs.

## Overview

This benchmark suite measures critical hot paths in the playback system to ensure optimal performance and detect regressions. All benchmarks use real playback logic (no mocks) to provide accurate measurements.

## Quick Start

```bash
# Run all benchmarks
cd libraries/soul-playback
cargo bench

# Run specific benchmark group
cargo bench -- audio_callback
cargo bench -- queue_operations
cargo bench -- volume

# View HTML reports
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
start target/criterion/report/index.html  # Windows
```

## Benchmark Groups

### 1. Audio Callback Latency (`audio_callback`)

**What it measures**: Time to process a single audio buffer (the most critical hot path).

**Why it matters**: Audio callbacks run on a real-time thread and must complete in <1ms to prevent dropouts.

**Variants**:
- Buffer sizes: 256, 512, 1024, 2048 samples (stereo)
- Simulates realistic audio processing workload

**Performance targets**:
- **p50**: <200μs (typical)
- **p99**: <1ms (critical - must not exceed)

**Run**:
```bash
cargo bench -- audio_callback
```

**Interpreting results**:
- If p99 exceeds 1ms: Audio dropouts likely on slower systems
- Compare across different buffer sizes to verify scaling

### 2. Queue Operations (`queue_operations`)

**What it measures**: Speed of queue manipulation operations.

**Why it matters**: These are called frequently during playback and from UI thread.

**Benchmarks**:
- `add_to_queue_next`: Add track to play next (LIFO)
- `add_to_queue_end`: Add track to queue end (FIFO)
- `skip_to_index`: Jump to specific queue position
- `get_queue_state`: Retrieve current queue state (UI polling)

**Performance targets**:
- **All operations**: <10μs per call
- **get_queue_state**: <1μs (lock-free read)

**Run**:
```bash
cargo bench -- queue_operations
```

### 3. State Transitions (`state_transitions`)

**What it measures**: Time to change playback state (play, pause, next, previous).

**Why it matters**: These are user-initiated actions that should feel instant.

**Benchmarks**:
- `play`: Start playback from stopped
- `pause`: Pause active playback
- `next`: Skip to next track
- `previous`: Go to previous track

**Performance targets**:
- **All transitions**: <100μs
- User perception: <16ms (one frame) for instant feel

**Run**:
```bash
cargo bench -- state_transitions
```

### 4. Lock-Free Queries (`lock_free_queries`)

**What it measures**: Speed of state queries (called from UI thread).

**Why it matters**: UI thread polls these frequently for responsive updates.

**Benchmarks**:
- `state`: Get current playback state
- `current_track`: Get current track info
- `volume`: Get volume level
- `is_muted`: Get mute state
- `shuffle_mode`: Get shuffle mode
- `repeat_mode`: Get repeat mode

**Performance targets**:
- **All queries**: <1μs (these should be simple atomic reads)

**Run**:
```bash
cargo bench -- lock_free_queries
```

### 5. Crossfade Performance (`crossfade`)

**What it measures**: Time to mix two audio buffers during crossfade.

**Why it matters**: Crossfades happen at track boundaries and must be smooth.

**Variants**:
- Crossfade durations: 1s, 3s, 5s, 10s
- 10 seconds of audio at 48kHz (worst case)

**Performance targets**:
- **1s crossfade**: <2ms
- **10s crossfade**: <10ms
- All processing happens at startup, not in audio callback

**Run**:
```bash
cargo bench -- crossfade
```

### 6. Crossfade Curves (`crossfade_curves`)

**What it measures**: Speed of gain calculation for different fade curves.

**Why it matters**: Gain is calculated per-sample during crossfade.

**Curves tested**:
- Linear
- SquareRoot
- SCurve
- EqualPower (default)
- Exponential

**Performance targets**:
- **1000 calculations**: <10μs
- Ensures no bottleneck during buffer mixing

**Run**:
```bash
cargo bench -- crossfade_curves
```

### 7. Volume Operations (`volume`)

**What it measures**: Volume control and ramping performance.

**Why it matters**: Volume changes are frequent and must be click-free.

**Benchmarks**:
- `set_volume`: Initiate volume change (triggers ramp)
- `toggle_mute`: Toggle mute state
- `apply_volume`: Apply volume to buffer (in audio callback)

**Performance targets**:
- **set_volume**: <1μs (just updates state)
- **apply_volume**: <100μs for 1024 samples (must fit in audio callback)

**Run**:
```bash
cargo bench -- volume
```

### 8. Shuffle Algorithms (`shuffle`)

**What it measures**: Time to shuffle/unshuffle queue.

**Why it matters**: Users toggle shuffle frequently.

**Variants**:
- Queue sizes: 10, 50, 100, 500 tracks
- `enable_shuffle`: Shuffle entire queue
- `disable_shuffle`: Restore original order

**Performance targets**:
- **100 tracks**: <50μs (should feel instant)
- **500 tracks**: <200μs

**Run**:
```bash
cargo bench -- shuffle
```

### 9. Allocation Frequency (`allocations`)

**What it measures**: Memory allocations during critical operations.

**Why it matters**: Allocations in audio callback cause jitter and dropouts.

**Benchmarks**:
- `audio_callback_no_alloc`: Verify ZERO allocations in audio callback
- `queue_add_allocations`: Measure allocations when adding to queue

**Performance targets**:
- **Audio callback**: 0 allocations (strict requirement)
- **Queue operations**: Minimal allocations (pre-allocated capacity)

**Run**:
```bash
cargo bench -- allocations
```

**Verification**:
Use `cargo-flamegraph` or `heaptrack` to verify zero allocations:
```bash
cargo test --test pause_during_startup_e2e_test -- --include-ignored
# Check with memory profiler
```

### 10. Seek Operations (`seek`)

**What it measures**: Time to seek within a track.

**Why it matters**: Seeking should feel instant when scrubbing.

**Benchmarks**:
- `seek_time`: Seek to specific time position
- `seek_percentage`: Seek to percentage of track

**Performance targets**:
- **Both operations**: <100μs (excluding audio decoder seek time)

**Run**:
```bash
cargo bench -- seek
```

## Continuous Integration

Benchmarks can be run in CI to detect performance regressions:

```yaml
# .github/workflows/benchmarks.yml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benchmarks
        run: cargo bench -p soul-playback -- --output-format bencher | tee output.txt
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
```

## Baseline Comparison

Compare current performance against a baseline:

```bash
# Save current performance as baseline
cargo bench -- --save-baseline main

# Make changes...

# Compare against baseline
cargo bench -- --baseline main
```

Criterion will show percentage changes:
```
audio_callback/process_audio/512
                        time:   [98.234 µs 98.456 µs 98.789 µs]
                        change: [-2.3421% -1.9876% -1.5432%] (p = 0.00 < 0.05)
                        Performance has improved.
```

## Performance Targets Summary

| Operation | Target | Critical |
|-----------|--------|----------|
| Audio callback (1024 samples) | <200μs p50, <1ms p99 | ⚠️ Critical |
| Queue operations | <10μs | Important |
| State transitions | <100μs | Important |
| Lock-free queries | <1μs | Important |
| Crossfade (10s) | <10ms | Normal |
| Volume apply (1024 samples) | <100μs | ⚠️ Critical |
| Shuffle (100 tracks) | <50μs | Normal |
| Allocations in audio callback | 0 | ⚠️ Critical |

**Critical** = Directly affects real-time audio quality
**Important** = Affects user experience responsiveness
**Normal** = Happens infrequently or during startup

## Troubleshooting

### Benchmarks fail to compile

**Issue**: Missing dependencies or features

**Fix**:
```bash
cd libraries/soul-playback
cargo bench --no-fail-fast
```

### Inconsistent results

**Issue**: CPU throttling or background processes

**Fix**:
1. Close background applications
2. Disable CPU frequency scaling:
   ```bash
   # Linux
   sudo cpupower frequency-set --governor performance

   # macOS - disable Turbo Boost
   sudo sysctl -w machdep.xcpm.turbo_boost=0
   ```
3. Run benchmarks multiple times and average

### High variance

**Issue**: System load or thermal throttling

**Fix**:
1. Increase sample size in benchmark:
   ```rust
   group.sample_size(1000);  // Default is 100
   ```
2. Check CPU temperature
3. Run on dedicated benchmark machine

## Advanced Usage

### Custom benchmark configuration

Create `.cargo/config.toml`:
```toml
[profile.bench]
opt-level = 3
lto = true
codegen-units = 1
```

### Profiling a specific benchmark

```bash
# Generate flamegraph
cargo flamegraph --bench playback_benchmarks -- audio_callback --profile-time 60

# Use perf
cargo bench --bench playback_benchmarks -- audio_callback --profile-time 10
perf record -g target/release/deps/playback_benchmarks-*
perf report
```

### Memory profiling

```bash
# Linux - Valgrind
valgrind --tool=massif cargo bench -- audio_callback

# macOS - Instruments
instruments -t "Allocations" cargo bench -- audio_callback
```

## Interpreting Results

### Good performance indicators:
- ✅ Flat scaling with buffer size (O(n))
- ✅ Low variance (<5% standard deviation)
- ✅ No outliers in p99/p100
- ✅ Stable over multiple runs

### Warning signs:
- ⚠️ High variance (>10% std dev) - indicates jitter
- ⚠️ Non-linear scaling - algorithm complexity issue
- ⚠️ Large p99-p50 gap - indicates worst-case problems
- ⚠️ Regressions >5% - investigate immediately

### Example output:
```
audio_callback/process_audio/1024
                        time:   [187.32 µs 189.45 µs 191.78 µs]
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe
```

**Analysis**:
- Mean: ~189μs (good - well below 1ms target)
- Variance: ~2.3% (good - low jitter)
- Outliers: 2% (acceptable - likely OS scheduling)

## Next Steps

After running benchmarks:

1. **Review results**: Check all targets are met
2. **Identify bottlenecks**: Focus on slowest operations
3. **Profile hot paths**: Use flamegraphs to find exact bottlenecks
4. **Optimize**: Make targeted improvements
5. **Re-benchmark**: Verify improvements with baseline comparison
6. **Commit baseline**: Save as new baseline for future comparisons

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Real-time Audio Programming 101](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing)
- [Soul Player Audio Architecture](../../docs/AUDIO_E2E_TESTING.md)
