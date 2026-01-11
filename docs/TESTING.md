# Soul Player - Testing Strategy

## Core Philosophy

**Quality Over Quantity**
- Deep, meaningful tests that verify actual behavior
- NO shallow tests (getters, setters, trivial constructors)
- Focus on edge cases, error paths, and integration points
- Target: 50-60% coverage (meaningful coverage, not line-counting)

---

## Test Pyramid

Unit tests should comprise approximately 70% of the test suite, covering business logic and edge cases. Integration tests make up 20%, testing component interactions. E2E tests are 10%, focusing only on critical user flows.

---

## Unit Tests

### What to Test
✅ **Test these:**
- Business logic with multiple paths
- Error handling and edge cases
- Algorithms and calculations
- State machines and transitions
- Parser/serialization logic

❌ **DON'T test these:**
- Simple getters/setters
- Trivial constructors
- Framework boilerplate
- Generated code

### Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Use descriptive test names
    #[test]
    fn decode_returns_error_for_corrupted_mp3() {
        // Arrange
        let corrupted_data = create_corrupted_mp3();

        // Act
        let result = AudioDecoder::decode(&corrupted_data);

        // Assert
        assert!(matches!(result, Err(AudioError::DecodeFailed(_))));
    }

    // Test edge cases
    #[test]
    fn playlist_handles_duplicate_track_ids() {
        let mut playlist = Playlist::new();
        let track_id = TrackId::new("track-1");

        playlist.add(track_id);
        playlist.add(track_id); // Duplicate

        assert_eq!(playlist.len(), 1); // Should deduplicate
    }
}
```

### Property-Based Testing
Use `proptest` for complex invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn eq_output_never_exceeds_input_plus_gain(
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000),
        gain_db in -20.0f32..20.0
    ) {
        let mut eq = Equalizer::new();
        eq.set_gain(gain_db);

        let output = eq.process(&samples);
        let max_expected = gain_db.db_to_linear();

        for &sample in &output {
            prop_assert!(sample.abs() <= max_expected * 1.01); // 1% tolerance
        }
    }
}
```

---

## Integration Tests

### Scope
Test interactions between components/crates:
- Storage ↔ Metadata
- Audio ↔ Effects chain
- Server ↔ Sync protocol
- Desktop UI ↔ Tauri commands

### Testcontainers Usage
Use testcontainers for **real** database integration:

```rust
use testcontainers::*;

#[tokio::test]
async fn test_multi_user_playlist_isolation() {
    // Start real SQLite container (NOT in-memory)
    let docker = clients::Cli::default();
    let container = docker.run(/* SQLite image */);

    let db_url = format!("sqlite://{}:5432", container.get_host_port_ipv4(5432));
    let storage = Storage::connect(&db_url).await.unwrap();

    // Create two users
    let user1 = storage.create_user("alice").await.unwrap();
    let user2 = storage.create_user("bob").await.unwrap();

    // Each creates a playlist
    let playlist1 = storage.create_playlist(user1.id, "Alice's Favorites").await.unwrap();
    let playlist2 = storage.create_playlist(user2.id, "Bob's Jams").await.unwrap();

    // Verify isolation
    let alice_playlists = storage.get_user_playlists(user1.id).await.unwrap();
    assert_eq!(alice_playlists.len(), 1);
    assert_eq!(alice_playlists[0].id, playlist1.id);

    // Cleanup happens automatically when container drops
}
```

### Why Real Databases?
- ✅ Catches SQL errors that in-memory DBs miss
- ✅ Tests actual constraints and triggers
- ✅ Validates migrations work correctly
- ✅ Realistic performance characteristics
- ❌ In-memory SQLite has different behavior (too permissive)

---

## Tauri Testing

### Command Testing (Unit-Level)
Test commands without running the full webview:

```rust
#[cfg(test)]
mod tests {
    use tauri::test::{mock_builder, mock_context, MockRuntime};

    #[test]
    fn test_play_track_command() {
        let app = mock_builder()
            .build(mock_context())
            .expect("failed to build app");

        let result = play_track(
            app.state(),
            TrackId::new("track-123")
        );

        assert!(result.is_ok());
    }
}
```

### Integration Testing (WebDriver)
For critical user flows only (~10% of tests):

```rust
// tests/ui_integration.rs
use tauri_driver::Driver;

#[tokio::test]
async fn test_complete_playback_flow() {
    let driver = Driver::new().await.unwrap();

    // Navigate and interact
    driver.goto("http://localhost:1420").await.unwrap();
    driver.find_element("#library").await?.click().await?;
    driver.find_element("#play-button").await?.click().await?;

    // Verify state change via IPC
    let is_playing = driver.eval("window.__TAURI__.invoke('is_playing')").await?;
    assert!(is_playing);
}
```

### Frontend Mocking
Use `@tauri-apps/api/mocks` for frontend unit tests:

```typescript
// vitest.config.ts
import { mockIPC } from '@tauri-apps/api/mocks'

beforeAll(() => {
  mockIPC((cmd, args) => {
    if (cmd === 'get_track_list') {
      return Promise.resolve([
        { id: '1', title: 'Test Track' }
      ])
    }
  })
})
```

---

## Server Testing

### Multi-User Scenarios
```rust
#[tokio::test]
async fn test_concurrent_playlist_modifications() {
    let server = TestServer::spawn().await;

    // Create shared playlist
    let playlist = server.create_shared_playlist("Collaborative").await?;

    // Simulate concurrent users
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let server = server.clone();
            let playlist_id = playlist.id;
            tokio::spawn(async move {
                server.add_track_to_playlist(playlist_id, TrackId::new(&format!("track-{}", i))).await
            })
        })
        .collect();

    // Wait for all
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all tracks added
    let final_playlist = server.get_playlist(playlist.id).await?;
    assert_eq!(final_playlist.tracks.len(), 10);
}
```

### Authentication Flow
```rust
#[tokio::test]
async fn test_jwt_expiration_and_refresh() {
    let server = TestServer::spawn().await;

    // Login
    let token = server.login("user", "pass").await?;

    // Use token successfully
    let playlists = server.get_playlists(&token).await?;
    assert!(!playlists.is_empty());

    // Wait for expiration (fast-forward time in test)
    server.advance_time(Duration::hours(2)).await;

    // Should fail
    let result = server.get_playlists(&token).await;
    assert!(matches!(result, Err(AuthError::TokenExpired)));

    // Refresh token
    let new_token = server.refresh_token(&token).await?;
    let playlists = server.get_playlists(&new_token).await?;
    assert!(!playlists.is_empty());
}
```

---

## Audio Testing

Soul Player has comprehensive audio testing organized into three tiers based on environment requirements.

### Test Categories

| Category | Environment | CI Support | Description |
|----------|-------------|------------|-------------|
| **Unit Tests** | Any | ✅ Full | DSP algorithms, effect processing, resampling |
| **Integration Tests** | Docker | ✅ Full | Effect chains, pipeline quality, glitch detection |
| **Device Tests** | Hardware | ❌ Skip | Real audio output, device switching, latency |

### Tier 1: Unit Tests (Any Environment)

These tests run everywhere - local dev, CI, Docker. No audio device required.

```rust
#[test]
fn test_eq_frequency_response() {
    let eq = ThreeBandEq::new(44100);
    let freq = 1000.0; // 1kHz
    let samples = generate_sine_wave(freq, 44100, 1.0);

    eq.set_mid_gain(6.0); // +6dB at 1kHz
    let output = eq.process(&samples);

    let output_rms = calculate_rms(&output);
    let expected_rms = 1.0 * (6.0f32).db_to_linear();
    assert!((output_rms - expected_rms).abs() < 0.1);
}
```

**Test files in this tier:**
- `libraries/soul-audio/tests/*_precision_test.rs` - DSP precision tests
- `libraries/soul-audio/tests/*_conformance_test.rs` - Standard compliance
- `libraries/soul-audio/tests/regression_test.rs` - Edge case coverage
- `libraries/soul-audio/tests/integration_test.rs` - Effect chain behavior

### Tier 2: Docker-Based Integration Tests

These tests use PulseAudio virtual devices in Docker containers. They verify:
- Buffer underrun detection
- Playback gap detection (important for Windows issues)
- Long-running stability
- Sample rate transitions

#### Running with Docker

```bash
# Start the audio test container
cd docker/audio-test
docker-compose up -d

# Run tests inside container
docker-compose exec audio-test bash
cd /workspace
cargo test --package soul-audio --test playback_glitch_test

# Alternative: Run directly (testcontainers)
cargo test --package soul-audio --test playback_glitch_test --features testcontainers
```

#### Docker Audio Test Files

| Test File | Purpose |
|-----------|---------|
| `playback_glitch_test.rs` | Buffer underruns, gaps, batch processing stress |
| `memory_stability_test.rs` | Memory leaks, long-running stability |
| `pipeline_quality_test.rs` | Full pipeline quality verification |

#### PulseAudio Virtual Devices

The Docker container (`docker/audio-test/Dockerfile`) provides:
- `virtual_output_1` - 44.1kHz stereo output
- `virtual_output_2` - 48kHz stereo output
- `virtual_output_3` - 96kHz stereo output
- `virtual_output_hires` - 192kHz high-resolution output
- Virtual input devices (for recording/loopback testing)

```bash
# Verify devices are available
docker-compose exec audio-test pactl list sinks short
```

### Tier 3: Hardware-Specific Tests (Manual/Local Only)

These tests require real audio hardware and **cannot run in CI**:

```rust
#[test]
#[ignore] // Skip in CI - requires real audio hardware
fn test_real_audio_output() {
    // This test is IGNORED by default
    // Run manually: cargo test test_real_audio_output -- --ignored
}
```

**Hardware-dependent behaviors:**
- Real audio device enumeration
- Actual audio output quality (subjective)
- Hardware-specific latency measurements
- Device hot-plugging behavior
- Exclusive mode / WASAPI shared mode

**Running hardware tests locally:**
```bash
# Run all ignored tests (requires audio hardware)
cargo test --package soul-audio -- --ignored

# Run specific hardware test
cargo test test_real_audio_output -- --ignored
```

### CI Configuration

```yaml
# .github/workflows/audio-tests.yml
name: Audio Tests

jobs:
  # Tier 1 & 2: Run in CI
  unit-and-integration:
    runs-on: ubuntu-latest
    services:
      audio:
        image: soul-audio-test:latest
        options: --health-cmd="pactl info"
    steps:
      - uses: actions/checkout@v4
      - name: Run audio unit tests
        run: cargo test --package soul-audio --lib

      - name: Run audio integration tests
        run: cargo test --package soul-audio --test integration_test

      - name: Run glitch detection tests
        run: cargo test --package soul-audio --test playback_glitch_test

  # Tier 3: NOT run in CI (no audio hardware)
  # These are run manually before releases
```

### Performance Testing
```rust
#[bench]
fn bench_decode_mp3_realtime(b: &mut Bencher) {
    let mp3_data = load_test_file("test.mp3");

    b.iter(|| {
        let decoder = AudioDecoder::new();
        let start = Instant::now();
        let buffer = decoder.decode(&mp3_data).unwrap();
        let elapsed = start.elapsed();

        // Must decode faster than playback duration
        let playback_duration = buffer.duration();
        assert!(elapsed < playback_duration);
    });
}
```

### Nightly Extended Tests

Some tests are designed for nightly CI runs (resource-intensive):

```rust
#[test]
#[ignore] // Run nightly: cargo test -- --ignored
fn test_one_hour_actual_playback() {
    // 1-hour sustained load test
    // Too long for regular CI
}
```

**Nightly test files:**
- `memory_stability_test.rs` - Extended memory leak detection
- `playback_glitch_test.rs` (ignored tests) - 1-hour playback tests

### Troubleshooting Audio Tests

**"No audio device found" errors:**
```bash
# Ensure PulseAudio is running (Docker)
docker-compose exec audio-test pulseaudio --check

# Restart PulseAudio
docker-compose exec audio-test pulseaudio --kill
docker-compose exec audio-test pulseaudio --start
```

**High timing variance in tests:**
- Expected in CI environments (shared resources)
- Tests use relaxed thresholds for CI compatibility
- Run locally for precise timing measurements

**Windows playback gaps:**
- These tests specifically target the Windows batch processing issue
- `test_windows_batch_processing_gap_simulation` simulates the scenario
- `test_inter_buffer_gap_detection` catches discontinuities

---

## Embedded Testing (ESP32-S3)

### Testing Strategy (Phase 3)

Soul Player uses a **pragmatic multi-tier testing approach** for ESP32-S3:

1. **Cross-Compilation Checks** (CI - Phase 1) ✅
   - Verify code compiles for `xtensa-esp32s3-espidf` target
   - Catch incompatible dependencies early
   - Runs on every PR in GitHub Actions

2. **Host-Based Unit Tests** (Phase 1-2)
   - Test core logic on development machine
   - 80% of test coverage happens here
   - Fast feedback loop for shared crates

3. **QEMU Basic Smoke Tests** (Phase 3)
   - Verify firmware boots and runs core logic
   - Limited peripheral emulation (no I2S, SD card, WiFi)
   - Good for testing Rust logic execution

4. **Manual Hardware Testing** (Phase 3)
   - Audio pipeline, I2S output, SD card, WiFi sync
   - Documented test procedures
   - Run before releases

### Cross-Compilation Verification (Current)

CI automatically checks ESP32-S3 compatibility:

```yaml
# .github/workflows/ci.yml
check-esp32:
  name: ESP32-S3 Cross-Compilation Check
  runs-on: ubuntu-latest
  steps:
    - name: Install ESP32 toolchain
      run: |
        cargo install espup
        espup install
        . $HOME/export-esp.sh

    - name: Check ESP32-S3 compilation
      run: |
        . $HOME/export-esp.sh
        cargo check -p soul-player-esp32 --target xtensa-esp32s3-espidf

    - name: Check core crates ESP32 compatibility
      run: |
        . $HOME/export-esp.sh
        cargo check -p soul-core --target xtensa-esp32s3-espidf
        cargo check -p soul-storage --target xtensa-esp32s3-espidf --no-default-features
```

This ensures dependencies added to core crates remain compatible with ESP32.

### QEMU Emulation (Phase 3)

Official Espressif QEMU support for ESP32-S3:

```bash
# Install ESP-IDF with QEMU
espup install

# Run firmware in QEMU
cd crates/soul-player-esp32
idf.py qemu flash

# Run with GDB for debugging
idf.py qemu gdb
```

**What QEMU Can Test:**
- ✅ Core Rust logic execution
- ✅ Basic firmware boot sequence
- ✅ Memory management
- ✅ Database operations (if using SD card emulation)
- ❌ I2S audio output (no hardware peripheral)
- ❌ Real SD card operations
- ❌ WiFi connectivity

**Use Cases:**
- Verify firmware builds and boots
- Test database migrations on ESP32 target
- Debug core logic issues without hardware
- CI smoke tests (optional)

### Wokwi Simulator (Optional, Premium)

If budget allows (~$20-50/month), Wokwi provides comprehensive ESP32-S3 simulation:

```yaml
# .github/workflows/esp32-test.yml (optional)
- name: Test with Wokwi
  uses: wokwi/wokwi-ci-action@v1
  with:
    token: ${{ secrets.WOKWI_CLI_TOKEN }}
    path: crates/soul-player-esp32
    expect_text: 'Firmware started'
```

**Advantages:**
- Real peripheral simulation (I2S, SD card, displays)
- Screenshot validation for UI testing
- Serial output monitoring
- GitHub Actions integration

**Decision:** Defer to Phase 3, evaluate based on need and budget.

### Platform-Specific Dependencies

Use conditional compilation for ESP32-specific code:

```rust
// In Cargo.toml
[target.'cfg(target_os = "espidf")'.dependencies]
esp-idf-hal = "0.43"
awedio_esp32 = "0.2"

[target.'cfg(not(target_os = "espidf"))'.dependencies]
cpal = "0.15"
```

### Hardware-in-Loop Testing (Phase 3)

When hardware available:

```rust
#[test]
#[cfg(target_os = "espidf")]
fn test_i2s_audio_output() {
    let mut output = EspI2sOutput::new();
    let test_tone = generate_sine_wave(440.0, 44100, 1.0);

    let result = output.play(&test_tone);
    assert!(result.is_ok());

    // Verify output via oscilloscope or loopback
}
```

### Manual Test Procedures (Phase 3)

Document hardware test checklist:

**Basic Functionality:**
- [ ] Firmware boots and displays UI on e-ink
- [ ] SD card mounts and reads database
- [ ] WiFi connects to configured network
- [ ] Plays MP3/FLAC files via I2S DAC
- [ ] Playlist navigation works

**Audio Quality:**
- [ ] No audible distortion at 80% volume
- [ ] EQ settings apply correctly
- [ ] Gapless playback between tracks
- [ ] Sample rate conversion (44.1kHz → 48kHz)

**Sync Protocol:**
- [ ] Authenticates with soul-server
- [ ] Downloads playlist updates
- [ ] Uploads playback stats
- [ ] Handles connection loss gracefully

**Power Management:**
- [ ] Idle power consumption <10mA
- [ ] Sleep mode <1mA
- [ ] Battery indicator accurate
- [ ] USB charging works

### Coverage Goals for ESP32

**Target: 40-50% coverage** (lower than core crates due to hardware glue)

**What to Test:**
- ✅ Business logic that runs on ESP32
- ✅ Error handling for hardware failures
- ✅ Database operations (use SQLite in QEMU)
- ❌ Hardware driver initialization (hard to test)
- ❌ I2S/SPI/I2C protocol details (tested by hardware libs)

### Resources

- [ESP-IDF QEMU Documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/tools/qemu.html)
- [Wokwi ESP32 Simulator](https://wokwi.com/esp32)
- [Wokwi CI for GitHub Actions](https://docs.wokwi.com/wokwi-ci/github-actions)
- [espup Toolchain Manager](https://github.com/esp-rs/espup)

---

## Snapshot Testing

For complex data structures:

```rust
use insta::assert_json_snapshot;

#[test]
fn test_playlist_serialization() {
    let playlist = create_test_playlist();

    assert_json_snapshot!(playlist, @r###"
    {
      "id": "playlist-123",
      "name": "Test Playlist",
      "tracks": [
        {"id": "track-1", "title": "Song 1"},
        {"id": "track-2", "title": "Song 2"}
      ]
    }
    "###);
}
```

---

## Test Data Management

### Fixtures
```rust
// tests/fixtures/mod.rs
pub fn create_test_track() -> Track {
    Track {
        id: TrackId::new("test-track"),
        title: "Test Song".into(),
        artist: "Test Artist".into(),
        duration: Duration::from_secs(180),
        ..Default::default()
    }
}

pub fn create_test_library() -> Library {
    let mut library = Library::new();
    library.add_track(create_test_track());
    library
}
```

### Test Audio Files
Store in `tests/data/`:
```
tests/
├── data/
│   ├── valid.mp3        # Valid MP3 file
│   ├── corrupted.mp3    # Intentionally broken
│   ├── silence.wav      # Pure silence
│   └── tone_440hz.flac  # Pure 440Hz tone
└── fixtures/
    └── mod.rs
```

---

## CI Integration

### GitHub Actions Workflow
```yaml
test:
  strategy:
    matrix:
      os: [ubuntu-latest, macos-latest, windows-latest]
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable

    - name: Run unit tests
      run: cargo test --all --lib

    - name: Run integration tests
      run: cargo test --all --test '*'

    - name: Run testcontainers tests
      run: cargo test --all --features testcontainers

    - name: Generate coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out Xml --output-dir coverage

    - name: Upload coverage
      uses: codecov/codecov-action@v3
      with:
        files: coverage/cobertura.xml
```

### Test Performance Tracking
```yaml
benchmark:
  runs-on: ubuntu-latest
  steps:
    - name: Run benchmarks
      run: cargo bench --all

    - name: Store benchmark results
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: target/criterion/benchmarks.json
        alert-threshold: '150%' # Alert if 50% regression
```

---

## Coverage Reporting

### Configuration
```toml
# .tarpaulin.toml
[tarpaulin]
command = "test"
out = ["Xml", "Html"]
exclude-files = [
    "*/tests/*",
    "*/benches/*",
    "**/main.rs",
]
target-dir = "target/tarpaulin"
```

### Guidelines
- **Target**: 50-60% coverage
- **Focus**: Business logic, not boilerplate
- **Non-blocking**: Report coverage, don't block PRs
- **Trend**: Monitor coverage trend, not absolute number

---

## Test Maintenance

### Regular Review
- Remove obsolete tests
- Update tests when requirements change
- Refactor duplicated test code
- Keep test data up to date

### Test Smells to Avoid
- ❌ Tests that test the test framework
- ❌ Tests with sleeps (`tokio::time::sleep`)
- ❌ Tests that depend on external services
- ❌ Tests with random inputs (unless property-based)
- ❌ Tests that modify global state

---

## Quick Reference

### Run Commands
```bash
# All tests
cargo test --all

# Specific crate
cargo test -p soul-storage

# Integration tests only
cargo test --test '*'

# With testcontainers
cargo test --features testcontainers

# Benchmarks
cargo bench --all

# Coverage
cargo tarpaulin --all

# ESP32 tests (simulator)
cargo test --target xtensa-esp32s3-espidf
```

### Key Dependencies
```toml
[dev-dependencies]
proptest = "1.4"
testcontainers = "0.15"
insta = "1.34"
criterion = "0.5"
mockall = "0.12"  # Use sparingly, prefer real implementations
```

---

## Resources

- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Proptest Documentation](https://proptest-rs.github.io/proptest/)
- [Testcontainers Rust](https://docs.rs/testcontainers/)
- [Tauri Testing Guide](https://v2.tauri.app/develop/tests/)
