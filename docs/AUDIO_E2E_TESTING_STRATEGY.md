# Audio E2E Testing Strategy

## Overview

This document outlines a comprehensive E2E testing strategy for two critical audio playback issues without using mocks:

1. **Playback initialization delay** - First play request triggers 200-800ms initialization
2. **Audio stutter at song start** - Intermittent stutter or false-start (song starts, restarts from beginning)

## Current State Analysis

### Issue 1: Lazy Initialization Delay

**Location**: `applications/desktop/src-tauri/src/playback_lazy.rs`

**Root Cause**:
- `LazyPlaybackManager` defers audio engine initialization until first `play()` command
- Initialization includes device enumeration + stream creation (200-800ms on macOS CoreAudio)
- First playback experiences full latency hit

**Impact**:
- Poor first-click UX (user presses play, waits 500ms+ for sound)
- Settings restoration happens asynchronously AFTER initialization returns

### Issue 2: Audio Stutter/False Start

**Potential Causes** (from code analysis):
1. **Buffer underrun** during initial playback (`libraries/soul-playback/src/manager.rs:119-129`)
2. **Source readiness check** timing (`source_ready_verified` flag, line 124)
3. **Start fade envelope** interaction with prebuffering
4. **Prebuffer configuration** - Current: 250ms (commit b749236), was 1000ms

**Relevant Code**:
```rust
// PlaybackManager tracks source readiness before starting playback
source_ready_verified: bool,
source_ready_wait_samples: usize,
```

## Best Practices Research

### Industry Standards

**Sources**:
- [Audio loopback latency testing (Android AOSP)](https://source.android.com/docs/compatibility/cts/audio-loopback-latency)
- [LatencyMon - Real-time audio testing](https://www.resplendence.com/latencymon)
- [Tauri E2E Testing Guide](https://v2.tauri.app/develop/tests/)
- [Audio buffer bloat simulation (Meegle)](https://www.meegle.com/en_us/advanced-templates/audio_quality_testing/audio_network_buffer_bloat_simulation)

### Key Findings

1. **Audio Loopback Testing** - Gold standard for E2E validation
   - Measures round-trip latency (speaker → microphone)
   - Detects initialization delays, stutters, dropouts
   - Requires physical or virtual audio devices

2. **Virtual Audio Devices** - Enable CI/CD testing
   - Linux: `snd-aloop` kernel module
   - Windows: VB-Cable, Virtual Audio Cable
   - macOS: BlackHole, Loopback

3. **Timing Metrics** - What to measure
   - Cold start latency (app launch → first audio output)
   - Warm start latency (play button → first audio output)
   - Buffer underrun rate
   - Inter-buffer gap detection
   - Phase continuity across buffer boundaries

## Proposed Testing Strategy

### Phase 1: Initialization Delay Testing

#### 1.1 Cold Start Latency Test (E2E with Real Audio Device)

**Approach**: WebDriver-based Tauri test with audio output monitoring

**Setup**:
```rust
// tests/e2e/audio_initialization_latency.rs
use tauri_driver::TauriDriver;
use std::time::Instant;

#[test]
fn test_cold_start_audio_latency() {
    // Launch app
    let driver = TauriDriver::new().unwrap();
    let start = Instant::now();

    // Wait for app ready
    driver.wait_for_window();

    // Inject test track
    driver.execute_script("window.testAudioFile = '/test/assets/1khz-tone.wav'");

    // Click play button
    let play_start = Instant::now();
    driver.find_element(By::TestId("play-button")).click();

    // Monitor audio output (via virtual device loopback)
    let first_audio = monitor_audio_output_start(Duration::from_secs(2));

    let play_to_audio_latency = first_audio.duration_since(play_start);

    println!("Cold start latency: {:?}", play_to_audio_latency);
    assert!(play_to_audio_latency < Duration::from_millis(1000),
        "Cold start took {:?}, expected < 1s", play_to_audio_latency);
}
```

**Implementation Details**:
```rust
fn monitor_audio_output_start(timeout: Duration) -> Instant {
    // Use cpal to monitor default output device
    // Detect first non-silent sample above threshold
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();

    let (tx, rx) = std::sync::mpsc::channel();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &_| {
            if data.iter().any(|&s| s.abs() > 0.01) {
                tx.send(Instant::now()).ok();
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None
    ).unwrap();

    stream.play().unwrap();
    rx.recv_timeout(timeout).unwrap()
}
```

#### 1.2 Warm Start Test (Pre-initialized Engine)

**Test Scenario**: Measure play latency when engine is already initialized

```rust
#[test]
fn test_warm_start_audio_latency() {
    let driver = TauriDriver::new().unwrap();

    // Pre-warm: play once, then stop
    driver.find_element(By::TestId("play-button")).click();
    std::thread::sleep(Duration::from_millis(500));
    driver.find_element(By::TestId("pause-button")).click();

    // Now measure second play (engine already initialized)
    let play_start = Instant::now();
    driver.find_element(By::TestId("play-button")).click();

    let first_audio = monitor_audio_output_start(Duration::from_secs(1));
    let warm_latency = first_audio.duration_since(play_start);

    println!("Warm start latency: {:?}", warm_latency);
    assert!(warm_latency < Duration::from_millis(100),
        "Warm start took {:?}, expected < 100ms", warm_latency);
}
```

#### 1.3 Initialization Options Test

**Goal**: Compare lazy vs eager initialization impact

```rust
#[test]
fn test_eager_vs_lazy_initialization() {
    // Test 1: Current lazy initialization
    let lazy_latency = measure_initialization_latency(InitMode::Lazy);

    // Test 2: Eager initialization (modify config for test)
    let eager_latency = measure_initialization_latency(InitMode::Eager);

    println!("Lazy init: {:?}, Eager init: {:?}", lazy_latency, eager_latency);

    // Eager should be slower at startup but faster at first play
}

fn measure_initialization_latency(mode: InitMode) -> TestMetrics {
    let launch_start = Instant::now();
    let driver = TauriDriver::with_config(mode).unwrap();
    let app_ready = launch_start.elapsed();

    let play_start = Instant::now();
    driver.find_element(By::TestId("play-button")).click();
    let first_audio = monitor_audio_output_start(Duration::from_secs(2));
    let first_play_latency = first_audio.duration_since(play_start);

    TestMetrics {
        app_ready_time: app_ready,
        first_play_latency,
    }
}
```

### Phase 2: Audio Stutter Detection

#### 2.1 Loopback Phase Continuity Test

**Approach**: Generate known test signal, capture via loopback, verify continuity

```rust
// tests/e2e/audio_stutter_detection.rs

#[test]
fn test_no_stutter_at_playback_start() {
    // Setup virtual loopback device
    let loopback = VirtualAudioDevice::new_loopback().unwrap();

    // Configure app to use loopback output
    set_audio_device(&loopback.output_device());

    // Load test file: 1kHz sine wave (for easy discontinuity detection)
    load_test_track("tests/assets/1khz-sine-10s.wav");

    // Start recording loopback input
    let recorder = loopback.start_recording();

    // Click play
    click_play_button();

    // Record first 2 seconds
    std::thread::sleep(Duration::from_secs(2));
    let recorded_samples = recorder.stop();

    // Analyze for discontinuities
    let stutters = detect_phase_discontinuities(&recorded_samples, 1000.0, 44100);

    println!("Detected {} stutter events in first 2 seconds", stutters.len());

    // Allow 1 discontinuity at start (fade-in), but no more
    assert!(stutters.len() <= 1,
        "Found {} stutters, expected ≤ 1: {:?}", stutters.len(), stutters);
}

fn detect_phase_discontinuities(
    samples: &[f32],
    frequency: f32,
    sample_rate: u32
) -> Vec<StutterEvent> {
    // Similar to existing playback_glitch_test.rs:detect_sine_discontinuities
    let max_expected_derivative = 2.0 * PI * frequency / sample_rate as f32;
    let threshold = max_expected_derivative * 3.0; // Allow some tolerance

    let mut events = Vec::new();
    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i-1]).abs();
        if diff > threshold {
            events.push(StutterEvent {
                sample_index: i,
                magnitude: diff,
                timestamp_ms: (i as f64 / sample_rate as f64) * 1000.0,
            });
        }
    }
    events
}
```

#### 2.2 False Start Detection Test

**Approach**: Monitor for duplicate playback start patterns

```rust
#[test]
fn test_no_false_start_restart() {
    let loopback = VirtualAudioDevice::new_loopback().unwrap();
    set_audio_device(&loopback.output_device());

    // Load track with distinctive intro (e.g., kick drum at 0s, 0.5s, 1.0s)
    load_test_track("tests/assets/distinctive-intro.wav");

    let recorder = loopback.start_recording();
    click_play_button();

    // Record first 3 seconds
    std::thread::sleep(Duration::from_secs(3));
    let recorded = recorder.stop();

    // Detect if the intro pattern appears twice (indicating a restart)
    let intro_pattern = load_intro_pattern("tests/assets/distinctive-intro-first-500ms.wav");
    let matches = find_pattern_occurrences(&recorded, &intro_pattern);

    assert_eq!(matches.len(), 1,
        "Intro pattern appeared {} times (false start detected)", matches.len());
}
```

#### 2.3 Buffer Underrun Stress Test

**Approach**: Simulate heavy CPU load during playback start

```rust
#[test]
fn test_playback_start_under_cpu_load() {
    let loopback = VirtualAudioDevice::new_loopback().unwrap();
    set_audio_device(&loopback.output_device());

    // Start CPU-intensive background task
    let cpu_load = simulate_heavy_load();

    // Wait for load to stabilize
    std::thread::sleep(Duration::from_millis(500));

    // Start playback
    let recorder = loopback.start_recording();
    click_play_button();

    std::thread::sleep(Duration::from_secs(2));
    let recorded = recorder.stop();
    cpu_load.stop();

    // Check for gaps (buffer underruns)
    let gaps = detect_silence_gaps(&recorded, 0.01, 100);

    assert!(gaps.is_empty(),
        "Found {} silence gaps under CPU load: {:?}", gaps.len(), gaps);
}

fn simulate_heavy_load() -> CpuLoadHandle {
    // Spawn thread doing heavy computation (similar to batch processing)
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    std::thread::spawn(move || {
        while !stop_clone.load(Ordering::Relaxed) {
            // Heavy work simulation
            for i in 0..10000 {
                std::hint::black_box((i as f64).sin().cos().tan());
            }
        }
    });

    CpuLoadHandle { stop }
}
```

### Phase 3: CI/CD Integration

#### 3.1 Virtual Audio Device Setup

**Linux (GitHub Actions)**:
```yaml
# .github/workflows/audio-e2e-tests.yml
jobs:
  audio-e2e-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup virtual audio device
        run: |
          sudo modprobe snd-aloop
          # Verify loopback device
          aplay -l | grep Loopback

      - name: Run audio E2E tests
        run: cargo test --test audio_initialization_latency -- --test-threads=1
        env:
          AUDIO_TEST_DEVICE: "hw:Loopback,0,0"
```

**macOS (GitHub Actions)**:
```yaml
  audio-e2e-macos:
    runs-on: macos-latest
    steps:
      - name: Install BlackHole
        run: |
          brew install blackhole-2ch
          # BlackHole creates virtual audio device

      - name: Run audio E2E tests
        run: cargo test --test audio_initialization_latency
        env:
          AUDIO_TEST_DEVICE: "BlackHole 2ch"
```

**Windows**:
```yaml
  audio-e2e-windows:
    runs-on: windows-latest
    steps:
      - name: Install Virtual Audio Cable
        run: |
          # Download and install VB-CABLE (free version)
          choco install vb-cable

      - name: Run audio E2E tests
        run: cargo test --test audio_initialization_latency
        env:
          AUDIO_TEST_DEVICE: "CABLE Input"
```

#### 3.2 Test Execution Strategy

**Test Configuration**:
```rust
// tests/e2e/common.rs
pub struct AudioTestConfig {
    pub use_real_device: bool,
    pub device_name: Option<String>,
    pub timeout: Duration,
    pub silence_threshold: f32,
}

impl AudioTestConfig {
    pub fn from_env() -> Self {
        Self {
            use_real_device: std::env::var("AUDIO_E2E_REAL_DEVICE").is_ok(),
            device_name: std::env::var("AUDIO_TEST_DEVICE").ok(),
            timeout: Duration::from_secs(
                std::env::var("AUDIO_TEST_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5)
            ),
            silence_threshold: 0.01,
        }
    }
}
```

**Parallel Execution Guard**:
```rust
// Only one audio test at a time (exclusive device access)
use std::sync::Mutex;
static AUDIO_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_with_exclusive_audio_access() {
    let _lock = AUDIO_TEST_LOCK.lock().unwrap();
    // Run test...
}
```

### Phase 4: Metrics & Monitoring

#### 4.1 Key Metrics to Track

```rust
#[derive(Debug)]
pub struct AudioPlaybackMetrics {
    // Initialization
    pub app_launch_to_ready: Duration,
    pub first_play_click_to_audio: Duration,
    pub audio_engine_init_time: Duration,

    // Playback quality
    pub stutters_detected: usize,
    pub false_starts: usize,
    pub buffer_underruns: usize,
    pub max_gap_duration: Duration,

    // Latency
    pub p50_play_latency: Duration,
    pub p95_play_latency: Duration,
    pub p99_play_latency: Duration,
}

impl AudioPlaybackMetrics {
    pub fn assert_quality_thresholds(&self) {
        assert!(self.first_play_click_to_audio < Duration::from_millis(500),
            "First play latency {} exceeds 500ms threshold",
            self.first_play_click_to_audio.as_millis());

        assert_eq!(self.stutters_detected, 0,
            "Detected {} stutters, expected 0", self.stutters_detected);

        assert_eq!(self.false_starts, 0,
            "Detected {} false starts, expected 0", self.false_starts);

        assert!(self.buffer_underruns <= 1,
            "Detected {} buffer underruns, expected ≤ 1", self.buffer_underruns);
    }
}
```

#### 4.2 Continuous Monitoring Dashboard

```rust
// tests/e2e/metrics_export.rs
pub fn export_metrics_to_json(metrics: &AudioPlaybackMetrics, path: &Path) {
    let json = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "commit": std::env::var("GITHUB_SHA").ok(),
        "metrics": {
            "init": {
                "app_launch_ms": metrics.app_launch_to_ready.as_millis(),
                "first_play_ms": metrics.first_play_click_to_audio.as_millis(),
                "engine_init_ms": metrics.audio_engine_init_time.as_millis(),
            },
            "quality": {
                "stutters": metrics.stutters_detected,
                "false_starts": metrics.false_starts,
                "underruns": metrics.buffer_underruns,
                "max_gap_ms": metrics.max_gap_duration.as_millis(),
            },
            "latency": {
                "p50_ms": metrics.p50_play_latency.as_millis(),
                "p95_ms": metrics.p95_play_latency.as_millis(),
                "p99_ms": metrics.p99_play_latency.as_millis(),
            }
        }
    });

    std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}
```

## Implementation Roadmap

### Week 1: Foundation
- [ ] Set up virtual audio device testing infrastructure
- [ ] Create `AudioTestHarness` utility crate
- [ ] Implement audio output monitoring with `cpal`
- [ ] Add phase discontinuity detection algorithms

### Week 2: Initialization Tests
- [ ] Implement cold start latency test
- [ ] Implement warm start latency test
- [ ] Add eager initialization option for comparison
- [ ] Create baseline metrics on current implementation

### Week 3: Stutter Detection Tests
- [ ] Implement loopback phase continuity test
- [ ] Add false start detection test
- [ ] Create buffer underrun stress test
- [ ] Test with various prebuffer configurations

### Week 4: CI/CD Integration
- [ ] Set up Linux CI with `snd-aloop`
- [ ] Set up macOS CI with BlackHole
- [ ] Set up Windows CI with VB-Cable
- [ ] Configure test execution strategy (serial, exclusive access)

### Week 5: Metrics & Documentation
- [ ] Implement metrics collection and export
- [ ] Create performance dashboard
- [ ] Document test maintenance procedures
- [ ] Write troubleshooting guide

## Test Asset Requirements

### Required Test Files

```
tests/
  assets/
    1khz-sine-10s.wav           # Pure 1kHz sine wave for phase continuity testing
    1khz-sine-30s.wav           # Extended test
    distinctive-intro.wav        # Track with clear intro pattern for false-start detection
    distinctive-intro-first-500ms.wav  # Reference pattern
    silence-1s.wav              # 1 second silence
    mixed-content.wav           # Real music sample for realistic testing
```

### Test File Generation Script

```bash
#!/bin/bash
# scripts/generate-test-audio.sh

# 1kHz sine wave (10 seconds)
sox -n -r 44100 -c 2 tests/assets/1khz-sine-10s.wav synth 10 sine 1000

# 1kHz sine wave (30 seconds)
sox -n -r 44100 -c 2 tests/assets/1khz-sine-30s.wav synth 30 sine 1000

# Distinctive intro (kick drum pattern)
sox -n -r 44100 -c 2 tests/assets/distinctive-intro.wav \\
  synth 0.1 sine 60 : synth 0.4 sine 0 : \\
  synth 0.1 sine 60 : synth 0.4 sine 0 : \\
  synth 0.1 sine 60 : synth 9.4 sine 0

# Extract first 500ms for pattern matching
sox tests/assets/distinctive-intro.wav tests/assets/distinctive-intro-first-500ms.wav trim 0 0.5

# Silence
sox -n -r 44100 -c 2 tests/assets/silence-1s.wav trim 0 1
```

## Alternative Approaches (If Virtual Devices Fail)

### Fallback 1: Null Output Device with Instrumentation

```rust
// Inject test points in the audio callback to monitor actual output
pub struct InstrumentedAudioOutput {
    inner: Box<dyn AudioOutput>,
    metrics: Arc<Mutex<OutputMetrics>>,
}

impl AudioOutput for InstrumentedAudioOutput {
    fn write(&mut self, buffer: &[f32]) -> Result<()> {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.samples_written += buffer.len();
        metrics.last_write_time = Instant::now();

        // Detect discontinuities
        if let Some(prev) = metrics.last_sample {
            let diff = (buffer[0] - prev).abs();
            if diff > 0.3 {
                metrics.discontinuities.push(Discontinuity {
                    sample_offset: metrics.samples_written,
                    magnitude: diff,
                });
            }
        }
        metrics.last_sample = buffer.last().copied();

        self.inner.write(buffer)
    }
}
```

### Fallback 2: File Output Testing

```rust
// Instead of real-time playback, write to WAV file and analyze
#[test]
fn test_stutter_via_file_output() {
    // Configure app to write audio to file instead of device
    set_audio_output_mode(AudioOutputMode::File("test-output.wav"));

    click_play_button();
    std::thread::sleep(Duration::from_secs(10));
    stop_playback();

    // Analyze written file
    let samples = read_wav_file("test-output.wav");
    let stutters = detect_phase_discontinuities(&samples, 1000.0, 44100);

    assert_eq!(stutters.len(), 0);
}
```

## Expected Outcomes

### Success Criteria

1. **Initialization Delay**:
   - Cold start (first play after launch): < 500ms
   - Warm start (subsequent plays): < 100ms
   - Regressions caught automatically in CI

2. **Stutter Detection**:
   - Zero false starts detected in 100 consecutive runs
   - ≤ 1 buffer underrun per 10-minute playback session
   - No phase discontinuities > 0.3 magnitude

3. **CI/CD Integration**:
   - All platforms (Linux/macOS/Windows) running E2E audio tests
   - Test execution time < 5 minutes per platform
   - Metrics exported and tracked over time

### Maintenance & Monitoring

- **Weekly**: Review CI metrics dashboard for trends
- **Per-PR**: Automated checks fail if thresholds exceeded
- **Monthly**: Comprehensive audio quality audit on real hardware
- **Quarterly**: Update test assets and expand test coverage

## References

### External Resources
- [Audio loopback latency test | Android Open Source Project](https://source.android.com/docs/compatibility/cts/audio-loopback-latency)
- [LatencyMon: suitability checker for real-time audio](https://www.resplendence.com/latencymon)
- [Tauri Tests Documentation](https://v2.tauri.app/develop/tests/)
- [Audio Network Buffer Bloat Simulation | Meegle](https://www.meegle.com/en_us/advanced-templates/audio_quality_testing/audio_network_buffer_bloat_simulation)
- [Audio testing scripts for Linux](https://gtkc.net/audio-testing-scripts-for-linux)
- [Video playback issues testing guide | FastPix](https://www.fastpix.io/blog/video-playback-issues-how-to-identify-test-and-fix-streaming-problems)

### Internal Documentation
- `libraries/soul-audio/tests/playback_glitch_test.rs` - Existing glitch detection tests
- `applications/desktop/src-tauri/src/playback_lazy.rs` - Lazy initialization implementation
- `libraries/soul-playback/src/manager.rs` - Core playback manager

---

**Document Version**: 1.0
**Last Updated**: 2026-02-11
**Author**: Claude Sonnet 4.5
**Next Review**: After Phase 1 implementation
