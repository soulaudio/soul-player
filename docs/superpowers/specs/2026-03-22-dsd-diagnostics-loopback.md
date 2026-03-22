# Spec: DSD Diagnostics IPC + Loopback Quality Suite

**Date**: 2026-03-22
**Status**: Draft

---

## Summary

Three interconnected areas:

1. **Bug fix** — `BUFFER_SIZE_SECONDS: 1 → 5` in `DsdAudioSource`. Reduces the ring buffer from 1s to 5s caused audio underruns on Windows because `sleep(1ms)` in the decode loop can sleep up to ~15ms (system timer resolution), leaving the audio callback with no samples. Restoring 5s absorbs up to 15 consecutive worst-case sleeps before dropout.

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

The decode loop sleeps 1ms when the ring buffer has fewer than `channels * CHUNK_FRAMES = 32768` free slots. On Windows, `std::thread::sleep(Duration::from_millis(1))` actually sleeps 10–15ms due to the default 15.6ms timer resolution. With a 1-second ring buffer (96,000 samples at 48kHz stereo), the buffer holds ~2 seconds of audio at the decode throttle point. Any two consecutive 15ms sleeps (30ms of no decoding) can drain 1,440 samples from the audio callback, and if the decode thread then wakes late the ring hits 0 → silence.

With a 5-second ring buffer (480,000 samples), the decoder can sleep for 15 consecutive 15ms intervals (225ms) before the ring drains below the minimum — far more than any realistic scheduling jitter.

### Fix

```rust
const BUFFER_SIZE_SECONDS: usize = 5;
```

`CHUNK_FRAMES: 16384` and `RESAMPLER_CHUNK: 4096` are unchanged (these are correct).

---

## Area 2 — Diagnostics IPC

### Data Type (soul-playback)

Add to `libraries/soul-playback/src/lib.rs`:

```rust
/// Runtime health metrics for a DSD audio source.
/// Returned by `AudioSource::dsd_diagnostics()`.
#[derive(Debug, Clone, Default)]
pub struct DsdDiagnostics {
    /// Number of times the audio callback requested samples but the ring
    /// buffer was empty (i.e. silence was output instead of audio).
    pub underrun_count: u64,
    /// Current number of PCM samples in the ring buffer (interleaved).
    pub buffer_fill_samples: usize,
    /// Total ring buffer capacity in samples (interleaved).
    pub buffer_capacity_samples: usize,
    /// Fill level as a percentage (0.0–100.0).
    pub buffer_fill_percent: f32,
    /// True while the decoder background thread is still producing samples
    /// (false after EOF or decode error).
    pub decoder_running: bool,
}
```

Add default method to `AudioSource` trait:
```rust
/// Returns DSD-specific diagnostics if this source is a DSD source.
/// Default implementation returns `None` (non-DSD sources).
fn dsd_diagnostics(&self) -> Option<DsdDiagnostics> {
    None
}
```

### DsdAudioSource Changes

**`SharedState`** — add one new atomic:
```rust
underrun_count: AtomicU64,  // incremented each time read_samples returns short
```
Initialised to 0 in `SharedState::new()`.

Also add `buffer_capacity: usize` as a plain (immutable) field on `DsdAudioSource` struct, set in `new()` to `ring_capacity`.

**`read_samples()`** — after computing `available`:
```rust
let available = self.buffer_consumer.slots();
let to_read = buffer.len().min(available);

// Count each callback invocation that couldn't fill the full request.
if to_read < buffer.len() {
    self.shared.underrun_count.fetch_add(1, Ordering::Relaxed);
}
```

**`dsd_diagnostics()` impl on `DsdAudioSource`**:
```rust
fn dsd_diagnostics(&self) -> Option<DsdDiagnostics> {
    let fill = self.buffer_consumer.slots();
    let cap  = self.buffer_capacity;
    Some(DsdDiagnostics {
        underrun_count:          self.shared.underrun_count.load(Ordering::Relaxed),
        buffer_fill_samples:     fill,
        buffer_capacity_samples: cap,
        buffer_fill_percent:     if cap == 0 { 0.0 } else { fill as f32 / cap as f32 * 100.0 },
        decoder_running:         !self.shared.is_eof.load(Ordering::Acquire),
    })
}
```

### PlaybackManager

Add to `libraries/soul-playback/src/manager.rs`:
```rust
/// Returns DSD diagnostics for the currently active source, or None if
/// the active source is not a DSD source or no source is active.
pub fn current_source_diagnostics(&self) -> Option<DsdDiagnostics> {
    self.sources.current_source()?.dsd_diagnostics()
}
```

### Tauri IPC Command

Add to `applications/desktop/src-tauri/src/playback.rs`:

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
    state: tauri::State<'_, AppState>,
) -> Result<Option<DsdDiagnosticsDto>, String> {
    let playback = state.playback.lock().await;
    let diag = playback.manager().current_source_diagnostics();
    Ok(diag.map(|d| DsdDiagnosticsDto {
        underrun_count:          d.underrun_count,
        buffer_fill_samples:     d.buffer_fill_samples,
        buffer_capacity_samples: d.buffer_capacity_samples,
        buffer_fill_percent:     d.buffer_fill_percent,
        decoder_running:         d.decoder_running,
    }))
}
```

Register command in `tauri::Builder` handler list alongside existing playback commands.

---

## Area 3 — Test Fixtures

Add to `applications/desktop/e2e-tests/playwright-global-setup.js`:

### `buildDsfLong(durationSecs)`

Generates a valid DSF binary with `ceil(durationSecs × 352800 / 4096)` data blocks, each block filled with `0x55` bytes (valid DSD signal: alternating 0/1 bits ≈ ~90Hz tone at DSD64). Uses the same sequential offset tracking as the existing `buildDsf()` helper.

Duration formula: `nBlocks × 4096 / 352800` seconds per channel.

### Seeded Records

**Album 5002** — `title: 'DSD Stress Album'`, `artist_id: 5001`

**Track 5003** — 60-second DSF:
```js
{ id: 5003, title: 'DSD Stress Track DSF', duration_seconds: 60.0,
  sample_rate: 2822400, file_format: 'DSF', album_id: 5002, artist_id: 5001 }
```
File: `dsd-stress-track.dsf` (~42MB, written to `audioDir`)

**Track 5004** — 60-second DFF (DSDIFF format):
```js
{ id: 5004, title: 'DSD Stress Track DFF', duration_seconds: 60.0,
  sample_rate: 2822400, file_format: 'DFF', album_id: 5002, artist_id: 5001 }
```
File: `dsd-stress-track.dff` (~42MB). Use the existing `DsdiffContainer` format: DSD chunk (28 bytes) → FVER (12 bytes) → PROP chunk with COMT, ABSS, CHNL, CMPR, LSCO → DST/DSD chunk with interleaved data.

Both files contain `track_sources` rows and `track_artists` junction rows.

---

## Area 4 — IPC Diagnostic E2E Suite

**File**: `applications/desktop/e2e-tests/tests/playwright/dsd-diagnostics.spec.js`

All tests use Track 5003 (60s DSF) unless noted. `test.setTimeout(45_000)`.

**Helpers**:
```js
const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params), { cmd, params });

async function playTrack5003(page) {
  const track = await invoke(page, 'get_track_by_id', { id: 5003 });
  await invoke(page, 'play_queue', {
    queue: [{ trackId: String(track.id), filePath: track.file_path,
              title: track.title, artist: track.artist,
              duration: track.duration_seconds, coverArtPath: null }],
    startIndex: 0,
  });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 10_000 },
  );
}

async function getDiag(page) {
  return invoke(page, 'get_dsd_diagnostics');
}
```

### Tests

**Test 1: Buffer fill ≥ 20% throughout 5s of playback**
- Call `playTrack5003`, poll `getDiag` every 500ms for 5s
- Assert each poll: `diag.buffer_fill_percent >= 20`

**Test 2: Zero underruns during 5s of playback**
- Call `playTrack5003`, wait 5s (polling state stays Playing)
- Assert `diag.underrun_count === 0`

**Test 3: Seek accuracy — three positions**
- Play track, for each target in `[15, 30, 45]` seconds:
  - Call `seek_to({ position: target })`
  - Wait up to 1s for `get_position` to settle within 0.5s of target
  - Assert `Math.abs(pos - target) < 0.5`

**Test 4: Zero underruns after 5 rapid seeks**
- Play track, fire 5 seeks to `[5, 20, 40, 10, 50]` seconds with 500ms between each
- After all seeks, wait 2s for playback to stabilise
- Assert `diag.underrun_count === 0`

**Test 5: Position monotonically advances for 10s**
- Play track, capture `get_position` every 100ms for 10s
- Assert each sample ≥ previous sample (allow ≤ 1 tolerance sample for seek/stutter detection; zero tolerance means genuine stutter)

**Test 6: Playback resumes within 500ms after seek**
- Play track, seek to 30s
- Capture `posA = get_position` immediately after seek
- Wait 500ms, capture `posB = get_position`
- Assert `posB > posA` (position advanced — decoder resumed)

**Test 7: Decoder stays running for 30s**
- Play track, poll `diag.decoder_running` every 1s for 30s
- Assert all polls return `true`

**Test 8: Underrun counter is fresh per play_queue**
- Play track 5003, wait 3s (may accumulate underruns if bug regresses)
- Issue `stop_playback`, then `play_queue` again
- Assert `diag.underrun_count === 0` (counter reset for new session)

**Test 9: DFF format — same health profile**
- Same as Tests 1+2 but with Track 5004 (DFF)
- Assert fill ≥ 20% and underrun_count = 0

---

## Area 5 — WASAPI Loopback Quality Suite

**File**: `applications/desktop/e2e-tests/tests/playwright/dsd-loopback-quality.spec.js`
**Script**: `applications/desktop/e2e-tests/scripts/analyze_dsd_audio.py`

### Prerequisites Check

At `beforeAll`, skip all tests with `test.skip()` if:
- `python` is not in PATH
- `import pyaudiowpatch` fails (Python subprocess probe)
- No loopback device found via `pyaudiowpatch.get_loopback_device_info_list()`

### Python Script: `analyze_dsd_audio.py`

```
Usage: python analyze_dsd_audio.py --duration <secs> --output <wav> [--mode silence|rms]
       --silence-threshold-ms <ms>   (default 50)
       --rms-window-ms <ms>          (default 100)
       --rms-max-drop-db <db>        (default 12)

Exit code:
  0  = pass
  1  = fail (prints description of first violation to stdout)
  2  = error (missing deps, no device)
```

**Silence detection**: 20ms RMS windows. A "gap" is a contiguous run of windows where `RMS < -60 dBFS`. Report gap durations.

**RMS drop detection**: sliding 100ms window. Flag if any window drops >12dB below the median of the first 1s.

**Loopback device**: `pyaudiowpatch.get_loopback_device_info_list()[0]` — uses the first available loopback device automatically. If none, exit code 2.

### Tests

Each test:
1. Calls `playTrack5003(page)` to start DSD playback
2. Waits `WARMUP_MS = 1500` for steady-state
3. Runs `analyze_dsd_audio.py` as a subprocess with the appropriate flags
4. Asserts exit code 0

**Test 1: No silence gap > 50ms during 10s playback**
```
--duration 10 --mode silence --silence-threshold-ms 50
```

**Test 2: RMS level consistent during 10s**
```
--duration 10 --mode rms --rms-window-ms 100 --rms-max-drop-db 12
```

**Test 3: No silence gap > 100ms after seek to 30s**
- Seek to 30s, wait 1s, then capture 5s:
```
--duration 5 --mode silence --silence-threshold-ms 100
```

**Test 4: Seek stress — 5 seeks, no silence per capture**
- For each seek target in `[5, 15, 25, 35, 45]`:
  - Seek, wait 1s, capture 3s with `--mode silence --silence-threshold-ms 100`
  - Assert exit code 0

**Test 5: 30s continuous playback — zero dropout windows**
```
--duration 30 --mode silence --silence-threshold-ms 50
```

---

## File Change Map

| File | Change | Area |
|------|--------|------|
| `libraries/soul-audio-desktop/src/sources/dsd/source.rs` | `BUFFER_SIZE_SECONDS` 1→5; `underrun_count` atomic; `DsdDiagnostics` struct; `dsd_diagnostics()` impl | 1+2 |
| `libraries/soul-playback/src/lib.rs` | `DsdDiagnostics` type; default `dsd_diagnostics()` on `AudioSource` trait | 2 |
| `libraries/soul-playback/src/manager.rs` | `current_source_diagnostics()` method | 2 |
| `applications/desktop/src-tauri/src/playback.rs` | `get_dsd_diagnostics` Tauri command + DTO + registration | 2 |
| `applications/desktop/e2e-tests/playwright-global-setup.js` | `buildDsfLong()`; Album 5002; Track 5003 (DSF 60s); Track 5004 (DFF 60s) | 3 |
| `applications/desktop/e2e-tests/tests/playwright/dsd-diagnostics.spec.js` | NEW — 9 IPC tests | 4 |
| `applications/desktop/e2e-tests/tests/playwright/dsd-loopback-quality.spec.js` | NEW — 5 loopback tests | 5 |
| `applications/desktop/e2e-tests/scripts/analyze_dsd_audio.py` | NEW — Python silence + RMS analysis | 5 |

---

## Implementation Order (TDD throughout)

1. **Area 1**: Revert `BUFFER_SIZE_SECONDS` → verify app plays without stutter
2. **Area 2**: Write failing Rust test for underrun counting → add `underrun_count` + `dsd_diagnostics()` → green; add Tauri command
3. **Area 3**: Add 60s fixtures to global setup → verify setup completes
4. **Area 4**: Write `dsd-diagnostics.spec.js` (9 tests) → run against app → all green
5. **Area 5**: Write `analyze_dsd_audio.py` + `dsd-loopback-quality.spec.js` → run → all green
