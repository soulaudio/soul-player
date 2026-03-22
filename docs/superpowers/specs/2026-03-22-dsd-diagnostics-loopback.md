# Spec: DSD Diagnostics IPC + Loopback Quality Suite

**Date**: 2026-03-22
**Status**: Draft v2

---

## Summary

Three interconnected areas:

1. **Bug fix** — `BUFFER_SIZE_SECONDS: 1 → 5` in `DsdAudioSource`. Reducing the ring buffer from 5s to 1s caused audio underruns on Windows because `sleep(1ms)` in the decode loop can sleep up to ~15ms (system timer resolution), leaving the audio callback with no samples. Restoring 5s absorbs up to 15 consecutive worst-case sleeps before dropout.

2. **Diagnostics IPC** — New `get_dsd_diagnostics` Tauri command exposing underrun count, buffer fill, and decoder liveness. Required for the IPC test suite to detect regressions without external capture infrastructure.

3. **Test suites** — Two new Playwright CDP spec files:
   - `dsd-diagnostics.spec.js` — 9 IPC-based tests (fast, no external deps)
   - `dsd-loopback-quality.spec.js` — 5 WASAPI loopback tests (actual audio output)

---

## Area 1 — Bug Fix: `BUFFER_SIZE_SECONDS`

### Root Cause

`libraries/soul-audio-desktop/src/sources/dsd/source.rs` line 24:
```rust
const BUFFER_SIZE_SECONDS: usize = 1;  // was 5 before last fix
```

The decode loop sleeps 1ms when the ring buffer has fewer than `channels * CHUNK_FRAMES = 32768` free slots. On Windows, `std::thread::sleep(Duration::from_millis(1))` actually sleeps 10–15ms due to the default 15.6ms timer resolution. With a 1-second ring buffer (96,000 samples at 48kHz stereo), any two consecutive 15ms sleeps (30ms of no decoding) can drain enough samples to underrun the audio callback → silence.

With a 5-second ring buffer (480,000 samples), the decoder can sleep for ~15 consecutive 15ms intervals (225ms) before the ring drains to zero — far more than any realistic scheduling jitter.

### Fix

```rust
const BUFFER_SIZE_SECONDS: usize = 5;
```

`CHUNK_FRAMES: 16384` and `RESAMPLER_CHUNK: 4096` are unchanged (these are correct).

---

## Area 2 — Diagnostics IPC

### Architecture Note

`soul-playback` is platform-agnostic and must not gain DSD-specific types (CLAUDE.md Rule 3). `DsdDiagnostics` lives in `soul-audio-desktop`. **No changes to the `AudioSource` trait or any existing `AudioSource` implementor.**

Instead, use a handle pattern: `DsdAudioSource::new()` exposes a `diagnostics_handle()` method returning a cheap `DsdDiagnosticsHandle` (an `Arc` into the source's `SharedState`). `DesktopPlayback` stores this handle as an `Option` whenever it creates a DSD source, and clears it when activating any non-DSD source. The Tauri IPC reads from the stored handle directly.

### `DsdDiagnosticsHandle` (new type in `soul-audio-desktop/src/sources/dsd/source.rs`)

A cheap, cloneable read handle into the source's shared state:

```rust
#[derive(Clone)]
pub struct DsdDiagnosticsHandle {
    shared: Arc<SharedState>,
    capacity: usize,
}

impl DsdDiagnosticsHandle {
    pub fn read(&self) -> DsdDiagnostics {
        let fill = self.shared.buffer_fill.load(Ordering::Relaxed);
        let cap  = self.capacity;
        DsdDiagnostics {
            underrun_count:          self.shared.underrun_count.load(Ordering::Relaxed),
            buffer_fill_samples:     fill,
            buffer_capacity_samples: cap,
            buffer_fill_percent:     if cap == 0 { 0.0 } else { fill as f32 / cap as f32 * 100.0 },
            decoder_running:         !self.shared.is_eof.load(Ordering::Acquire),
        }
    }
}
```

`buffer_consumer.slots()` (the consumer's current readable count) is not accessible from the handle — `buffer_consumer` is owned by `DsdAudioSource`. Add a new atomic to `SharedState` that `read_samples()` keeps current:

```rust
buffer_fill: AtomicUsize,  // samples currently readable in ring; updated by read_samples()
```

Initialised to 0 in `SharedState::new()`. Updated at the end of `read_samples()`, after committing the read chunk:
```rust
// Store updated fill level (slots remaining after this read)
self.shared.buffer_fill.store(self.buffer_consumer.slots(), Ordering::Relaxed);
```

`DsdAudioSource` gains:
```rust
pub fn diagnostics_handle(&self) -> DsdDiagnosticsHandle {
    DsdDiagnosticsHandle { shared: Arc::clone(&self.shared), capacity: self.buffer_capacity }
}
```

### `DsdDiagnostics` struct (`soul-audio-desktop/src/sources/dsd/source.rs`)

```rust
#[derive(Debug, Clone)]
pub struct DsdDiagnostics {
    /// Times the audio callback requested samples but the ring buffer was empty.
    pub underrun_count: u64,
    /// Current PCM samples in ring buffer (interleaved).
    pub buffer_fill_samples: usize,
    /// Total ring buffer capacity in interleaved samples.
    pub buffer_capacity_samples: usize,
    /// Fill level 0.0–100.0.
    pub buffer_fill_percent: f32,
    /// True while the decoder background thread is still producing (false after EOF).
    pub decoder_running: bool,
}
```

### `SharedState` changes

Add one new atomic field:
```rust
underrun_count: AtomicU64,
```
Initialised to 0 in `SharedState::new()`.

### `DsdAudioSource` struct changes

Add one immutable field:
```rust
buffer_capacity: usize,
```
Set in `new()` to `ring_capacity` (the value computed from `target_sample_rate * BUFFER_SIZE_SECONDS * channels`). Must be included in `Ok(Self { ..., buffer_capacity: ring_capacity, ... })`.

### `read_samples()` underrun counting

In `read_samples()`, underruns happen at two points — both must be counted:

```rust
// Point 1: ring completely empty — true underrun (silence output, no samples available)
if to_read == 0 {
    self.shared.underrun_count.fetch_add(1, Ordering::Relaxed);
    buffer.fill(0.0);
    return Ok(0);
}

// Point 2: partial fill (ring has fewer samples than callback requested)
if to_read < buffer.len() {
    self.shared.underrun_count.fetch_add(1, Ordering::Relaxed);
}
```

(The existing early-return at `to_read == 0` is on line 213–215; insert the `fetch_add` before the `buffer.fill(0.0)` line.)

### `DsdAudioSource::diagnostics()` (public, not trait-based)

```rust
pub fn diagnostics(&self) -> DsdDiagnostics {
    let fill = self.buffer_consumer.slots();
    let cap  = self.buffer_capacity;
    DsdDiagnostics {
        underrun_count:          self.shared.underrun_count.load(Ordering::Relaxed),
        buffer_fill_samples:     fill,
        buffer_capacity_samples: cap,
        buffer_fill_percent:     if cap == 0 { 0.0 } else { fill as f32 / cap as f32 * 100.0 },
        decoder_running:         !self.shared.is_eof.load(Ordering::Acquire),
    }
}
```

### `DesktopPlayback` integration (`soul-audio-desktop/src/playback.rs`)

`DesktopPlayback` is where DSD sources are constructed (the `load_dsd_source` or equivalent path that calls `DsdAudioSource::new()`). Locate that call site and:

1. Add a new field to `DesktopPlayback`:
   ```rust
   dsd_diagnostics_handle: Option<DsdDiagnosticsHandle>,
   ```
   Initialised to `None`.

2. After `DsdAudioSource::new(...)` succeeds and before `activate_source` is called, store the handle:
   ```rust
   self.dsd_diagnostics_handle = Some(source.diagnostics_handle());
   ```

3. When a non-DSD source is activated (e.g. `LocalAudioSource`), clear it:
   ```rust
   self.dsd_diagnostics_handle = None;
   ```

4. Add a public method:
   ```rust
   pub fn get_dsd_diagnostics(&self) -> Option<DsdDiagnostics> {
       self.dsd_diagnostics_handle.as_ref().map(|h| h.read())
   }
   ```

### Tauri IPC Command (`applications/desktop/src-tauri/src/playback.rs`)

Follow the exact same state access pattern as `get_position()`. Add a helper to the Tauri-layer `PlaybackManager` that calls through to `DesktopPlayback::get_dsd_diagnostics()`, then expose it as a command:

```rust
#[derive(serde::Serialize)]
pub struct DsdDiagnosticsDto {
    pub underrun_count: u64,
    pub buffer_fill_samples: usize,
    pub buffer_capacity_samples: usize,
    pub buffer_fill_percent: f32,
    pub decoder_running: bool,
}

#[tauri::command]
pub async fn get_dsd_diagnostics(
    playback: tauri::State<'_, LazyPlaybackManager>,
) -> Result<Option<DsdDiagnosticsDto>, String> {
    let pb = playback.get().await.map_err(|e| e.to_string())?;
    let diag = pb.get_dsd_diagnostics().await;
    Ok(diag.map(|d| DsdDiagnosticsDto {
        underrun_count:          d.underrun_count,
        buffer_fill_samples:     d.buffer_fill_samples,
        buffer_capacity_samples: d.buffer_capacity_samples,
        buffer_fill_percent:     d.buffer_fill_percent,
        decoder_running:         d.decoder_running,
    }))
}
```

Register `get_dsd_diagnostics` in `tauri::Builder`'s `invoke_handler` alongside existing playback commands in `main.rs`.

---

## Area 3 — Test Fixtures

Add to `applications/desktop/e2e-tests/playwright-global-setup.js`.

### Duration and file size calculation

DSF block = 4096 bytes per channel. Each byte = 8 DSD bits = 8 samples.
Samples per block per channel: `4096 × 8 = 32768`.
At DSD64 (2,822,400 Hz): one block = `32768 / 2822400 = 11.61ms`.
Blocks for 60s: `ceil(60000 / 11.61) = 5169 blocks`.
File size: `5169 × 4096 × 2 channels ≈ 42.4MB`.

### `buildDsfLong(durationSecs)`

Generates a valid DSF binary for `durationSecs` seconds of DSD64 stereo audio.

```js
function buildDsfLong(durationSecs) {
  const BLOCK_SIZE = 4096;
  const CHANNELS = 2;
  const DSD_RATE = 2_822_400;
  const SAMPLES_PER_BLOCK = BLOCK_SIZE * 8;
  const nBlocks = Math.ceil(durationSecs * DSD_RATE / SAMPLES_PER_BLOCK);
  // Re-use the existing buildDsf(nBlocks) helper — it already handles DSF
  // header construction correctly. Just call it with the computed block count.
  return buildDsf(nBlocks);
}
```

`buildDsf` already exists in `playwright-global-setup.js`. `buildDsfLong` is a thin wrapper that computes the correct block count.

### `buildDffLong(durationSecs)`

Similarly wraps the existing `buildDff(numSamplesPerChannel)` helper:

```js
function buildDffLong(durationSecs) {
  const DSD_RATE = 2_822_400;
  const numSamplesPerChannel = Math.ceil(durationSecs * DSD_RATE);
  return buildDff(numSamplesPerChannel);
}
```

### Seeded Records

**Album 5002** — `title: 'DSD Stress Album'`, `artist_id: 5001`

**Track 5003** — 60-second DSF:
```js
{ id: 5003, title: 'DSD Stress Track DSF', duration_seconds: 60.0,
  sample_rate: 2822400, file_format: 'DSF', album_id: 5002, artist_id: 5001 }
```
File: `dsd-stress-track.dsf` (~42MB), written to `audioDir`.

**Track 5004** — 60-second DFF:
```js
{ id: 5004, title: 'DSD Stress Track DFF', duration_seconds: 60.0,
  sample_rate: 2822400, file_format: 'DFF', album_id: 5002, artist_id: 5001 }
```
File: `dsd-stress-track.dff` (~42MB), written to `audioDir`.

Both need `track_sources` and `track_artists` rows (same pattern as Tracks 5001/5002).

---

## Area 4 — IPC Diagnostic E2E Suite

**File**: `applications/desktop/e2e-tests/tests/playwright/dsd-diagnostics.spec.js`

All tests use Track 5003 (60s DSF) unless noted. `test.setTimeout(45_000)`.

### Helpers

```js
const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params), { cmd, params });

async function playTrack5003(page) {
  const track = await invoke(page, 'get_track_by_id', { id: 5003 });
  await invoke(page, 'play_queue', {
    queue: [{
      trackId:      String(track.id),
      filePath:     track.file_path,
      title:        track.title,
      artist:       track.artist,
      durationSeconds: track.duration_seconds,  // camelCase — matches TrackData serde
      coverArtPath: null,
    }],
    startIndex: 0,
  });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 10_000 },
  );
}

const getDiag = (page) => invoke(page, 'get_dsd_diagnostics');
```

### Tests

**Test 1: Buffer fill ≥ 20% throughout 5s of playback**
Poll `getDiag` every 500ms for 5s. Assert each poll: `diag.buffer_fill_percent >= 20`.

**Test 2: Zero underruns during 5s of playback**
Play for 5s (polling state stays Playing). Assert final `diag.underrun_count === 0`.

**Test 3: Seek accuracy — three positions**
For each target in `[15, 30, 45]` seconds: `seek_to({ position: target })`, wait up to 1s for `get_position` to settle. Assert `Math.abs(pos - target) < 0.5`.

**Test 4: Zero underruns after 5 rapid seeks**
Play track. Fire 5 seeks to `[5, 20, 40, 10, 50]` seconds with 500ms between each. After all seeks, wait 2s for playback to stabilise. Assert `diag.underrun_count === 0`.

**Test 5: Position monotonically advances for 10s**
Poll `get_position` every 100ms for 10s while Playing. Assert each sample ≥ previous.

**Test 6: Playback resumes within 500ms after seek**
Play track, seek to 30s. Capture `posA` immediately. Wait 500ms, capture `posB`. Assert `posB > posA`.

**Test 7: Decoder stays running for 30s**
Poll `diag.decoder_running` every 1s for 30s. Assert all polls return `true`.

**Test 8: Underrun counter is fresh per new play_queue**
Play track 5003. Wait 3s. Issue `stop_playback`. Issue `play_queue` again with Track 5003 (a new `DsdAudioSource` is constructed with a fresh `SharedState`, resetting the counter to 0). Assert `diag.underrun_count === 0` immediately after the second play starts.

Note: the counter resets because each `play_queue` creates a new `DsdAudioSource` with a new `SharedState`. If the counter were moved to a long-lived shared store, this test would need to change.

**Test 9: DFF format — same health profile**
Same sequence as Tests 1+2 but with Track 5004 (DFF). Assert fill ≥ 20% and underrun_count = 0.

---

## Area 5 — WASAPI Loopback Quality Suite

**File**: `applications/desktop/e2e-tests/tests/playwright/dsd-loopback-quality.spec.js`
**Script**: `applications/desktop/e2e-tests/scripts/analyze_dsd_audio.py`

### Prerequisites Check

In `beforeAll`, detect prerequisites and call `test.skip()` for all tests if any are absent:
- `python` in PATH (probe with `spawnSync('python', ['--version'])`)
- `pyaudiowpatch` importable (probe with `spawnSync('python', ['-c', 'import pyaudiowpatch'])`)
- At least one loopback device (probe with `spawnSync('python', ['-c', 'import pyaudiowpatch; assert pyaudiowpatch.get_loopback_device_info_list()'])`)

### Python Script: `analyze_dsd_audio.py`

```
Usage:
  python analyze_dsd_audio.py
    --duration <secs>
    --output <wav_path>
    [--mode silence|rms]
    [--silence-threshold-ms <ms>]     default 50
    [--rms-window-ms <ms>]            default 100
    [--rms-max-drop-db <db>]          default 12

Exit codes:
  0  pass
  1  fail  (prints description of first violation to stdout)
  2  error (no loopback device, missing deps, capture failure)
```

**Loopback device**: `pyaudiowpatch.get_loopback_device_info_list()[0]` — uses first available loopback device automatically (no hardcoded device index).

**Silence detection** (`--mode silence`):
- Compute RMS in 20ms windows over the captured WAV.
- A "gap" is a contiguous run of windows where `RMS < -60 dBFS`.
- Fail if any gap ≥ `--silence-threshold-ms`.

**RMS drop detection** (`--mode rms`):
- Compute median RMS of the first 1s as baseline.
- Fail if any `--rms-window-ms` window drops > `--rms-max-drop-db` below baseline.

### Tests

Shared constant: `WARMUP_MS = 1500` (wait after play_queue before starting capture).

Each test: play Track 5003, wait `WARMUP_MS`, run `analyze_dsd_audio.py` as a subprocess, assert `exitCode === 0`.

**Test 1: No silence gap > 50ms during 10s playback**
```
--duration 10 --mode silence --silence-threshold-ms 50
```

**Test 2: RMS level consistent during 10s**
```
--duration 10 --mode rms --rms-window-ms 100 --rms-max-drop-db 12
```

**Test 3: No silence gap > 100ms after seek to 30s**
Seek to 30s, wait 1s, then run:
```
--duration 5 --mode silence --silence-threshold-ms 100
```

**Test 4: Seek stress — 5 seeks, no silence per capture**
For each target in `[5, 15, 25, 35, 45]` seconds:
- Seek, wait 1s, run:
```
--duration 3 --mode silence --silence-threshold-ms 100
```
Assert exit code 0 for each.

**Test 5: 30s continuous playback — zero dropout windows**
```
--duration 30 --mode silence --silence-threshold-ms 50
```

---

## File Change Map

| File | Change | Area |
|------|--------|------|
| `libraries/soul-audio-desktop/src/sources/dsd/source.rs` | `BUFFER_SIZE_SECONDS` 1→5; `underrun_count` + `buffer_fill` atomics in `SharedState`; `buffer_capacity` field on struct; underrun counting + fill tracking in `read_samples()`; `DsdDiagnostics` struct; `DsdDiagnosticsHandle` type; `diagnostics_handle()` method | 1+2 |
| `libraries/soul-audio-desktop/src/playback.rs` | `dsd_diagnostics_handle` field; set on DSD source creation; clear on non-DSD; `get_dsd_diagnostics()` method | 2 |
| `applications/desktop/src-tauri/src/playback.rs` | `DsdDiagnosticsDto`; `get_dsd_diagnostics` Tauri command; Tauri-layer helper method | 2 |
| `applications/desktop/src-tauri/src/main.rs` | Register `get_dsd_diagnostics` in `invoke_handler` | 2 |
| `applications/desktop/e2e-tests/playwright-global-setup.js` | `buildDsfLong()`, `buildDffLong()`; Album 5002; Track 5003 (DSF 60s); Track 5004 (DFF 60s); `track_sources` + `track_artists` rows | 3 |
| `applications/desktop/e2e-tests/tests/playwright/dsd-diagnostics.spec.js` | NEW — 9 IPC tests | 4 |
| `applications/desktop/e2e-tests/tests/playwright/dsd-loopback-quality.spec.js` | NEW — 5 loopback tests | 5 |
| `applications/desktop/e2e-tests/scripts/analyze_dsd_audio.py` | NEW — silence + RMS analysis Python script | 5 |

---

## Implementation Order (TDD throughout)

1. **Area 1**: Change `BUFFER_SIZE_SECONDS: 1 → 5`; verify app plays without stutter
2. **Area 2**: Write failing Rust test that an underrun increments the counter → add `underrun_count` atomic, `buffer_capacity` field, both counting sites in `read_samples()`, `diagnostics()` method → green. Add `as_any()` to trait + all implementors. Add Tauri command + register it. Verify `get_dsd_diagnostics` returns data via the running app.
3. **Area 3**: Add `buildDsfLong`/`buildDffLong` wrappers and seed records to global setup; run setup and verify files written.
4. **Area 4**: Write `dsd-diagnostics.spec.js` (9 tests) → run against app → all green
5. **Area 5**: Write `analyze_dsd_audio.py` + `dsd-loopback-quality.spec.js` → run → all green (or skipped cleanly if pyaudiowpatch absent)
