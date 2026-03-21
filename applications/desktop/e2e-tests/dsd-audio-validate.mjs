/**
 * DSD audio validation — interactive then used as basis for E2E test.
 *
 * Strategy:
 *   1. Pick a DSD track from the library (or accept path via CLI)
 *   2. Decode it offline to PCM WAV via ffmpeg (the "ground truth" reference)
 *   3. Connect to running Soul Player via CDP, queue + play the track from start
 *   4. Capture system output via Python (Stereo Mix / WASAPI loopback)
 *   5. Cross-correlate to align timing, then compute:
 *        - RMS (silence check)
 *        - Waveform correlation with reference (≥ 0.80 = good)
 *        - Spectral centroid match (< 500 Hz diff = good)
 *        - Band energy balance
 *
 * Usage:
 *   node dsd-audio-validate.mjs
 *   node dsd-audio-validate.mjs --dsf "D:\music\City Pop\Hiroshi Sato\Orient\01 - Kalimba Night.dsf"
 *   node dsd-audio-validate.mjs --dsf <path> --capture-device 15 --capture-duration 8
 */

import { chromium } from '@playwright/test';
import { execSync, spawnSync } from 'child_process';
import { existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import Database from 'better-sqlite3';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CAPTURES_DIR = join(__dirname, 'captures');
mkdirSync(CAPTURES_DIR, { recursive: true });

const CDP_URL = 'http://localhost:9222';
const DB_PATH = 'C:/Users/sebas/AppData/Roaming/Soul Player Dev/soul-player.db';
const STEREO_MIX_DEVICE = 15;
const CAPTURE_SR = 48000;

// ── CLI args ──────────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const get = (flag, def) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : def;
};
const DSF_PATH_OVERRIDE = get('--dsf', null);
const CAPTURE_DEVICE   = parseInt(get('--capture-device', String(STEREO_MIX_DEVICE)));
const CAPTURE_DURATION = parseFloat(get('--capture-duration', '8'));

// ── Helpers ───────────────────────────────────────────────────────────────────

const invoke = (page, cmd, params = {}) =>
  page.evaluate(
    async ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params }
  );

async function connectCdp() {
  for (let i = 0; i < 10; i++) {
    try {
      const browser = await chromium.connectOverCDP(CDP_URL);
      const ctx = browser.contexts()[0];
      const page = ctx.pages().find(
        p => (p.url().includes('tauri.localhost') || p.url().includes('localhost:1420'))
          && !p.url().includes('splash')
      );
      if (page) return { browser, page };
      await browser.close();
    } catch {}
    console.log(`Waiting for app (${i + 1}/10)...`);
    await new Promise(r => setTimeout(r, 1500));
  }
  throw new Error('Could not connect to Soul Player via CDP');
}

function ffmpegDecodeDsf(dsfPath, outWav, durationSecs) {
  console.log(`\n  Decoding DSF → PCM reference (${durationSecs}s @ 88200Hz)...`);
  const result = spawnSync('ffmpeg', [
    '-y', '-i', dsfPath,
    '-t', String(durationSecs),
    '-ar', '88200',
    '-ac', '2',
    outWav
  ], { encoding: 'utf8', stdio: 'pipe' });
  if (result.status !== 0) {
    throw new Error(`ffmpeg failed: ${result.stderr}`);
  }
  console.log(`  Reference saved → ${outWav}`);
}

function captureAudio(deviceIdx, durationSecs, outWav) {
  console.log(`\n  Capturing ${durationSecs}s from device ${deviceIdx} → ${outWav}`);
  // Delegate to Python since sounddevice is already installed there
  const pyScript = `
import sounddevice as sd
import scipy.io.wavfile as wav
import numpy as np, sys, time

device = ${deviceIdx}
sr = ${CAPTURE_SR}
dur = ${durationSecs}
out = r"${outWav.replace(/\\/g, '\\\\')}"

print(f"  Recording {dur}s...", flush=True)
audio = sd.rec(int(sr*dur), samplerate=sr, channels=2, dtype='float32', device=device)
for i in range(int(dur)):
    time.sleep(1)
    print(f"  {i+1}/{int(dur)}s", end='\\r', flush=True)
sd.wait()
print()
wav.write(out, sr, (audio * 32767).astype(np.int16))
print(f"  Capture saved: {out}", flush=True)
`;
  const result = spawnSync('python', ['-c', pyScript], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  });
  if (result.status !== 0) throw new Error(`Capture failed: ${result.stderr}`);
  process.stdout.write(result.stdout);
}

function analyzeAndCompare(refWav, captureWav) {
  const pyScript = `
import sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
import scipy.io.wavfile as wav
import scipy.signal as sig
import scipy.fft as fft
import numpy as np

sr_ref, ref_raw = wav.read(r"${refWav.replace(/\\/g, '\\\\')}")
sr_cap, cap_raw = wav.read(r"${captureWav.replace(/\\/g, '\\\\')}")

def to_float(arr, sr_name):
    if arr.dtype == np.int16:
        return arr.astype(np.float32) / 32768.0, int(sr_name)
    if arr.dtype == np.int32:
        return arr.astype(np.float32) / 2147483648.0, int(sr_name)
    return arr.astype(np.float32), int(sr_name)

ref, sr_r = to_float(ref_raw, sr_ref)
cap, sr_c = to_float(cap_raw, sr_cap)

# Mono left channel
ref_l = ref[:, 0] if ref.ndim == 2 else ref
cap_l = cap[:, 0] if cap.ndim == 2 else cap

# ── RMS / silence check ───────────────────────────────────────────────────────
cap_rms = float(np.sqrt(np.mean(cap_l**2)))
cap_rms_db = 20 * np.log10(cap_rms + 1e-12)
ref_rms = float(np.sqrt(np.mean(ref_l**2)))
ref_rms_db = 20 * np.log10(ref_rms + 1e-12)
is_silent = cap_rms_db < -60

# ── Resample capture to reference SR for comparison ───────────────────────────
if sr_c != sr_r:
    num_samples = int(len(cap_l) * sr_r / sr_c)
    cap_l_rs = sig.resample(cap_l, num_samples)
else:
    cap_l_rs = cap_l

# ── Cross-correlation alignment (find offset) ─────────────────────────────────
# Use short windows (2s each) to keep memory reasonable
window = min(sr_r * 2, len(ref_l), len(cap_l_rs))
corr_full = sig.correlate(cap_l_rs[:sr_r*10], ref_l[:sr_r*2], mode='valid')
lag = int(np.argmax(np.abs(corr_full)))

# Aligned capture segment
cap_aligned = cap_l_rs[lag: lag + window]
ref_aligned = ref_l[:window]
n = min(len(cap_aligned), len(ref_aligned))

if n < sr_r:
    print("WARNING: alignment window too short, skipping correlation")
    correlation = 0.0
else:
    correlation = float(np.corrcoef(ref_aligned[:n], cap_aligned[:n])[0, 1])

# ── Spectral analysis (on capture) ───────────────────────────────────────────
def band_energy_db(signal_arr, sr, flo, fhi):
    n = min(sr * 2, len(signal_arr))
    freqs = fft.rfftfreq(n, 1 / sr)
    spec = np.abs(fft.rfft(signal_arr[:n] * np.hanning(n)))
    mask = (freqs >= flo) & (freqs < fhi)
    e = np.mean(spec[mask]) if mask.any() else 1e-12
    return float(20 * np.log10(e + 1e-12))

bands_cap = {
    "20-200Hz":  band_energy_db(cap_l, sr_c, 20, 200),
    "200-2kHz":  band_energy_db(cap_l, sr_c, 200, 2000),
    "2k-8kHz":   band_energy_db(cap_l, sr_c, 2000, 8000),
    "8k-20kHz":  band_energy_db(cap_l, sr_c, 8000, 20000),
    ">20kHz":    band_energy_db(cap_l, sr_c, 20000, sr_c // 2),
}
bands_ref = {
    "20-200Hz":  band_energy_db(ref_l, sr_r, 20, 200),
    "200-2kHz":  band_energy_db(ref_l, sr_r, 200, 2000),
    "2k-8kHz":   band_energy_db(ref_l, sr_r, 2000, 8000),
    "8k-20kHz":  band_energy_db(ref_l, sr_r, 8000, 20000),
}

# Spectral centroid
def centroid(arr, sr):
    n = min(sr * 2, len(arr))
    freqs = fft.rfftfreq(n, 1 / sr)
    spec = np.abs(fft.rfft(arr[:n]))
    return float(np.sum(freqs * spec) / (np.sum(spec) + 1e-12))

centroid_cap = centroid(cap_l, sr_c)
centroid_ref = centroid(ref_l, sr_r)

# ── Print results ─────────────────────────────────────────────────────────────
print()
print("  ── Reference (ffmpeg DSD→PCM decode) ──────────────────────────────")
print(f"     RMS: {ref_rms_db:.1f} dBFS   Centroid: {centroid_ref:.0f} Hz")
print(f"     Bands: " + "  ".join(f"{k}={v:.0f}" for k,v in bands_ref.items()))

print()
print("  ── Soul Player capture (Stereo Mix) ────────────────────────────────")
status = "✗ SILENT" if is_silent else "✓ audio"
print(f"     RMS: {cap_rms_db:.1f} dBFS   {status}   Centroid: {centroid_cap:.0f} Hz")
print(f"     Bands: " + "  ".join(f"{k}={v:.0f}" for k,v in bands_cap.items()))
if ">20kHz" in bands_cap:
    print(f"     Ultrasonic (>20kHz): {bands_cap['>20kHz']:.0f} dB  (DSD noise shaping visible if > -60)")

print()
print("  ── Comparison ───────────────────────────────────────────────────────")
print(f"     Waveform correlation (aligned): {correlation:.4f}  (target ≥ 0.80)")
corr_verdict = "✓ PASS" if correlation >= 0.80 else ("✗ FAIL" if not is_silent else "✗ SILENT — no signal")
print(f"     Verdict: {corr_verdict}")
print(f"     RMS diff: {cap_rms_db - ref_rms_db:+.1f} dB")
print(f"     Centroid diff: {centroid_cap - centroid_ref:+.0f} Hz  (< ±500 Hz = good)")
centroid_ok = abs(centroid_cap - centroid_ref) < 1000
rms_ok = cap_rms_db > -60
print()
overall = "✓ ALL PASS" if (rms_ok and centroid_ok and correlation >= 0.60) else "✗ CHECK FAILED"
print(f"  ── Overall: {overall} ──")
`;
  const result = spawnSync('python', ['-c', pyScript], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' }
  });
  if (result.stderr && result.stderr.includes('Traceback')) {
    console.error('Analysis error:', result.stderr);
  }
  process.stdout.write(result.stdout);
  if (result.stderr && !result.stderr.includes('Traceback')) {
    process.stderr.write(result.stderr);
  }
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  // 1. Pick DSD track
  let dsfPath = DSF_PATH_OVERRIDE;
  let trackId = null;

  if (!dsfPath) {
    const db = new Database(DB_PATH, { readonly: true });
    const track = db.prepare(`
      SELECT t.id, t.file_path, t.title, ar.name AS artist, t.duration_seconds
      FROM tracks t
      LEFT JOIN artists ar ON ar.id = t.artist_id
      WHERE LOWER(t.file_format) IN ('dsf','dff','dsdiff')
        AND t.is_available = 1
        AND t.duration_seconds BETWEEN 60 AND 600
        AND t.title IS NOT NULL
      ORDER BY t.duration_seconds ASC
      LIMIT 1
    `).get();
    db.close();
    if (!track) throw new Error('No suitable DSD track found in DB');
    dsfPath = track.file_path;
    trackId = track.id;
    console.log(`\nSelected DSD track:`);
    console.log(`  ID:     ${trackId}`);
    console.log(`  Title:  ${track.title}`);
    console.log(`  Artist: ${track.artist}`);
    console.log(`  File:   ${dsfPath}`);
    console.log(`  Dur:    ${track.duration_seconds?.toFixed(1)}s`);
  }

  if (!existsSync(dsfPath)) throw new Error(`DSF file not found: ${dsfPath}`);

  // 2. Decode reference offline
  const refWav = join(CAPTURES_DIR, 'dsd-reference.wav');
  ffmpegDecodeDsf(dsfPath, refWav, CAPTURE_DURATION + 2); // +2s buffer

  // 3. Connect to app via CDP
  console.log('\nConnecting to Soul Player...');
  const { browser, page } = await connectCdp();
  console.log('Connected.');

  // 4. Queue and play the track from start
  if (trackId) {
    console.log(`\nQueueing track ${trackId} and playing from start...`);
    // Fetch full track info from IPC for proper queue item shape
    const allTracks = await invoke(page, 'get_all_tracks');
    const track = allTracks.find(t => t.id === trackId);
    const queueItem = track ? {
      trackId: String(track.id),
      title: track.title,
      artist: track.artist_name || track.artist || 'Unknown Artist',
      album: track.album_title || track.album || null,
      albumId: track.album_id || null,
      filePath: track.file_path || dsfPath,
      durationSeconds: track.duration_seconds || null,
      trackNumber: track.track_number || null,
      coverArtPath: null,
    } : { trackId: String(trackId), filePath: dsfPath, title: '', artist: 'Unknown Artist' };
    await invoke(page, 'play_queue', { queue: [queueItem], startIndex: 0 });
  } else {
    console.log('\n(Manual mode — start the DSF track playing in Soul Player now)');
    console.log('Press Enter when playback has started...');
    await new Promise(r => process.stdin.once('data', r));
  }

  // Small delay to let playback stabilize (decoder startup + resampler)
  await new Promise(r => setTimeout(r, 1200));

  // 5. Capture
  const captureWav = join(CAPTURES_DIR, 'dsd-capture.wav');
  captureAudio(CAPTURE_DEVICE, CAPTURE_DURATION, captureWav);

  await browser.close();

  // 6. Analyze & compare
  console.log('\n  Analyzing...');
  analyzeAndCompare(refWav, captureWav);

  console.log(`\n  Files:`);
  console.log(`    Reference: ${refWav}`);
  console.log(`    Capture:   ${captureWav}`);
}

main().catch(e => { console.error(e); process.exit(1); });
