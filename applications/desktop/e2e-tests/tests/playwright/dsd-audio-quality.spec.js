/**
 * DSD Audio Quality Tests
 *
 * Validates that DSD (DSF/DFF) tracks produce valid audio output by:
 *   1. Decoding the DSF file to a PCM WAV reference via ffmpeg (ground truth)
 *   2. Playing the same track through Soul Player
 *   3. Capturing system audio via WASAPI loopback (pyaudiowpatch, device 14)
 *   4. Comparing spectral centroid + RMS + band-energy fractions (20kHz-limited)
 *
 * Why waveform correlation is NOT used:
 *   Soul Player and ffmpeg use different DSD-to-PCM resampling filters, so phase
 *   alignment will not match. Algorithm-agnostic metrics (centroid, band fractions)
 *   are used instead — they reflect perceived content without depending on filter phase.
 *
 * Requirements:
 *   pip install pyaudiowpatch scipy numpy
 *   ffmpeg in PATH
 *   WASAPI loopback device 14 = "Speakers (Realtek(R) Audio) [Loopback]"
 */

import { test, expect, chromium } from '@playwright/test';

// SOUL_CDP_URL is set by both global setups (standard port 9222, prod port 9223).
const CDP_URL = process.env.SOUL_CDP_URL ?? 'http://localhost:9222';
import { spawnSync } from 'child_process';
import { mkdirSync, existsSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── Config ────────────────────────────────────────────────────────────────────
const CAPTURES_DIR    = join(__dirname, '..', '..', 'captures');
const WASAPI_DEVICE   = 14;     // "Speakers (Realtek(R) Audio) [Loopback]"
const CAPTURE_SR      = 48000;
const CAPTURE_SECS    = 8;
const WARMUP_MS       = 1500;   // wait after play_queue before capturing

// Thresholds — intentionally loose to survive DAC chain variance and
// track-to-track spectral variation (interludes vs full tracks, etc.)
const CENTROID_MAX_DIFF_HZ = 2500;
const RMS_MAX_DIFF_DB      = 12;
const CAPTURE_MIN_RMS_DB   = -60;

// ── Module-level state ────────────────────────────────────────────────────────

let browser, page;
let dsdTrack   = null;
let refWav     = null;
let captureWav = null;
let analysis   = null;
let skipReason = null;

// ── Helpers ───────────────────────────────────────────────────────────────────

const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(
    ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params }
  );

/** Find a suitable DSD track via the app's IPC (works with any DB). */
async function getDsdTrackFromIpc() {
  const DSD_FORMATS = new Set(['dsf','dff','dsdiff','DSF','DFF','DSDIFF']);
  const allTracks = await invoke(page, 'get_all_tracks');
  const candidates = allTracks.filter(t =>
    DSD_FORMATS.has(t.file_format) &&
    t.file_path &&
    t.duration_seconds >= 30 &&
    t.title &&
    existsSync(t.file_path)
  );
  if (candidates.length === 0) return null;
  // Pick shortest qualifying track to minimise capture time
  candidates.sort((a, b) => a.duration_seconds - b.duration_seconds);
  return candidates[0];
}

function ffmpegDecodeDsf(dsfPath, outWav) {
  const r = spawnSync('ffmpeg', [
    '-y', '-i', dsfPath,
    '-t', String(CAPTURE_SECS + 2),
    '-ar', '88200', '-ac', '2', outWav,
  ], { encoding: 'utf8', stdio: 'pipe' });
  if (r.status !== 0) throw new Error(`ffmpeg failed: ${r.stderr?.slice(-400)}`);
}

function captureAudio(outWav) {
  const py = `
import pyaudiowpatch as pyaudio, wave, numpy as np
DEVICE=${WASAPI_DEVICE}; RATE=${CAPTURE_SR}; CHUNK=1024; DUR=${CAPTURE_SECS}
OUT=r"${outWav.replace(/\\/g, '\\\\')}"
p=pyaudio.PyAudio()
frames=[]
s=p.open(format=pyaudio.paInt16,channels=2,rate=RATE,input=True,input_device_index=DEVICE,frames_per_buffer=CHUNK)
for _ in range(int(RATE/CHUNK*DUR)): frames.append(s.read(CHUNK,exception_on_overflow=False))
s.stop_stream(); s.close(); p.terminate()
raw=b''.join(frames)
with wave.open(OUT,'wb') as wf:
    wf.setnchannels(2); wf.setsampwidth(2); wf.setframerate(RATE); wf.writeframes(raw)
arr=np.frombuffer(raw,dtype=np.int16).astype(np.float32)/32768.0
print(f"{20*np.log10(float(np.sqrt(np.mean(arr**2)))+1e-12):.2f}")
`;
  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`Capture failed: ${r.stderr}`);
  return parseFloat(r.stdout.trim());
}

function analyzeSpectral(refPath, capPath) {
  const py = `
import sys,io; sys.stdout=io.TextIOWrapper(sys.stdout.buffer,encoding='utf-8',errors='replace')
import scipy.io.wavfile as wav,scipy.signal as sg,scipy.fft as fft,numpy as np,json
CUTOFF=20000
def analyze(path):
    sr,raw=wav.read(path)
    ch=raw[:,0].astype(np.float32)/32768.0 if raw.ndim==2 else raw.astype(np.float32)/32768.0
    rms_db=float(20*np.log10(np.sqrt(np.mean(ch**2))+1e-12))
    nyq=sr/2.0
    if CUTOFF<nyq:
        sos=sg.butter(8,CUTOFF/nyq,btype='low',output='sos'); ch=sg.sosfilt(sos,ch)
    n=min(sr*4,len(ch)); w=ch[:n]*np.hanning(n)
    freqs=fft.rfftfreq(n,1/sr); spec=np.abs(fft.rfft(w))
    mask=freqs<=CUTOFF; f2=freqs[mask]; s2=spec[mask]
    centroid=float(np.sum(f2*s2)/(np.sum(s2)+1e-12))
    total=float(np.sum(s2**2)+1e-12)
    def frac(lo,hi):
        m=(f2>=lo)&(f2<hi); return float(np.sum(s2[m]**2)/total) if m.any() else 0.0
    return dict(rms_db=rms_db,centroid=centroid,
        sub=frac(20,200),bass=frac(200,2000),mid=frac(2000,8000),hi=frac(8000,20000))
ref=analyze(r"${refPath.replace(/\\/g, '\\\\')}"); cap=analyze(r"${capPath.replace(/\\/g, '\\\\')}")
print(json.dumps(dict(
    ref_rms_db=ref['rms_db'],cap_rms_db=cap['rms_db'],
    rms_diff_db=abs(ref['rms_db']-cap['rms_db']),
    ref_centroid=ref['centroid'],cap_centroid=cap['centroid'],
    centroid_diff_hz=abs(ref['centroid']-cap['centroid']),
    band_ratios={b:cap[b]/(ref[b]+1e-9) for b in ['sub','bass','mid','hi']},
    ref_band_fracs={b:ref[b] for b in ['sub','bass','mid','hi']},
)))
`;
  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`Spectral analysis failed: ${r.stderr}`);
  return JSON.parse(r.stdout.trim());
}

async function ensurePlaying() {
  const state = await invoke(page, 'get_playback_state');
  if (state === 'Playing') return;
  const allTracks = await invoke(page, 'get_all_tracks');
  const t = allTracks.find(tr => tr.id === dsdTrack.id);
  await invoke(page, 'play_queue', {
    queue: [{
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: t.coverArtPath || null,
    }],
    startIndex: 0,
  });
  await page.waitForTimeout(WARMUP_MS);
}

// ── Suite ─────────────────────────────────────────────────────────────────────

test.describe('DSD Audio Quality', () => {

  test.beforeAll(async () => {
    mkdirSync(CAPTURES_DIR, { recursive: true });

    // Check toolchain
    const ffmpegOk = spawnSync('ffmpeg', ['-version'], { stdio: 'pipe' }).status === 0;
    const pyOk     = spawnSync('python', ['-c', 'import pyaudiowpatch,scipy,numpy'], { stdio: 'pipe' }).status === 0;
    if (!ffmpegOk) { skipReason = 'ffmpeg not found in PATH'; return; }
    if (!pyOk)     { skipReason = 'pyaudiowpatch/scipy/numpy not installed'; return; }

    // CDP connection first (needed for IPC track lookup)
    browser = await chromium.connectOverCDP(CDP_URL);
    const ctx = browser.contexts()[0];
    page = ctx.pages().find(
      p => (p.url().includes('tauri.localhost') || p.url().includes('localhost:1420'))
           && !p.url().includes('splash')
    );
    if (!page) { skipReason = 'Could not find main app window via CDP'; return; }

    // Find a real DSD track via IPC (works with test DB or production DB)
    dsdTrack = await getDsdTrackFromIpc();
    if (!dsdTrack) {
      skipReason = 'No DSD track with a real on-disk file found (need production library)';
      return;
    }

    refWav     = join(CAPTURES_DIR, `dsd-ref-${dsdTrack.id}.wav`);
    captureWav = join(CAPTURES_DIR, `dsd-cap-${dsdTrack.id}.wav`);

    // Decode reference (idempotent)
    if (!existsSync(refWav)) ffmpegDecodeDsf(dsdTrack.file_path, refWav);
  });

  test.afterAll(async () => {
    if (page) await invoke(page, 'stop_playback').catch(() => {});
    if (browser) await browser.close();
  });

  // ── 1. Library presence ────────────────────────────────────────────────────

  test('1. DSD track present in library with valid metadata', () => {
    if (skipReason) test.skip(true, skipReason);
    const DSD_FORMATS = new Set(['dsf','dff','dsdiff','DSF','DFF','DSDIFF']);
    expect(dsdTrack).toBeTruthy();
    expect(DSD_FORMATS.has(dsdTrack.file_format)).toBe(true);
    expect(dsdTrack.duration_seconds).toBeGreaterThan(0);
    expect(dsdTrack.sample_rate).toBeGreaterThan(1_000_000); // DSD64 = 2 822 400 Hz
    expect(dsdTrack.title).toBeTruthy();
    expect(existsSync(dsdTrack.file_path)).toBe(true);
    console.log(`Track: "${dsdTrack.title}" — ${dsdTrack.artist}`);
    console.log(`Format: ${dsdTrack.file_format} @ ${dsdTrack.sample_rate?.toLocaleString()} Hz`);
  });

  // ── 2. ffmpeg reference decode ─────────────────────────────────────────────

  test('2. ffmpeg decodes DSF to PCM reference without error', () => {
    if (skipReason) test.skip(true, skipReason);
    expect(existsSync(refWav)).toBe(true);
    const size = statSync(refWav).size;
    expect(size).toBeGreaterThan(100_000); // >100 KB for 8s of audio
    console.log(`Reference: ${(size / 1024 / 1024).toFixed(1)} MB at 88 200 Hz`);
  });

  // ── 3. Playback state ──────────────────────────────────────────────────────

  test('3. Soul Player reaches Playing state for DSD track', async () => {
    if (skipReason) test.skip(true, skipReason);
    await ensurePlaying();
    const state = await invoke(page, 'get_playback_state');
    expect(state).toBe('Playing');
    const title = await page.locator('[data-testid="now-playing-title"]').textContent({ timeout: 5000 });
    console.log(`Now playing: "${title}"  state=${state}`);
  });

  // ── 4. Non-silent audio output ─────────────────────────────────────────────

  test('4. WASAPI loopback captures non-silent audio during DSD playback', async () => {
    if (skipReason) test.skip(true, skipReason);
    await ensurePlaying();
    const rms_db = captureAudio(captureWav);
    console.log(`Capture RMS: ${rms_db.toFixed(1)} dBFS (silent < ${CAPTURE_MIN_RMS_DB} dBFS)`);
    expect(rms_db).toBeGreaterThan(CAPTURE_MIN_RMS_DB);
    expect(existsSync(captureWav)).toBe(true);
  });

  // ── 5-7. Spectral analysis (run once, shared across tests) ────────────────

  test('5. Spectral analysis: centroid, RMS, and band ratios match reference', () => {
    if (skipReason) test.skip(true, skipReason);
    expect(existsSync(refWav)).toBe(true);
    expect(existsSync(captureWav)).toBe(true);

    analysis = analyzeSpectral(refWav, captureWav);

    console.log([
      `Centroid: ref=${analysis.ref_centroid.toFixed(0)} Hz  cap=${analysis.cap_centroid.toFixed(0)} Hz`,
      `diff=${analysis.centroid_diff_hz.toFixed(0)} Hz  (max ${CENTROID_MAX_DIFF_HZ} Hz)`,
    ].join('  '));
    console.log(`RMS: ref=${analysis.ref_rms_db.toFixed(1)} dBFS  cap=${analysis.cap_rms_db.toFixed(1)} dBFS  diff=${analysis.rms_diff_db.toFixed(1)} dB`);
    for (const [band, ratio] of Object.entries(analysis.band_ratios)) {
      console.log(`  ${band}: ratio=${ratio.toFixed(2)}x`);
    }

    // Centroid match — same perceptual content class
    expect(analysis.centroid_diff_hz).toBeLessThan(CENTROID_MAX_DIFF_HZ);

    // Loudness match — allow ReplayGain / volume headroom
    expect(analysis.rms_diff_db).toBeLessThan(RMS_MAX_DIFF_DB);

    // Band energy: only check bands where the reference has ≥5% of total energy.
    // Low-energy reference bands (e.g. a bass-only interlude with near-zero highs)
    // produce unreliable ratios due to noise floor / other system audio.
    const MIN_REF_FRAC = 0.05;
    for (const [band, ratio] of Object.entries(analysis.band_ratios)) {
      const refFrac = analysis.ref_band_fracs[band];
      if (refFrac < MIN_REF_FRAC) continue; // skip sparse bands
      console.log(`  ${band}: ratio=${ratio.toFixed(2)}x  (ref frac=${refFrac.toFixed(3)})`);
      expect(ratio).toBeGreaterThan(0.05);   // band present in capture
      expect(ratio).toBeLessThan(20);        // not wildly over-represented
    }
  });

  // ── 8. Position advance ────────────────────────────────────────────────────

  test('6. Playback position advances at correct rate', async () => {
    if (skipReason) test.skip(true, skipReason);
    await ensurePlaying();
    const pos1 = await invoke(page, 'get_position');
    await page.waitForTimeout(3000);
    const pos2 = await invoke(page, 'get_position');
    console.log(`Position: ${pos1?.toFixed(2)}s → ${pos2?.toFixed(2)}s  (Δ=${(pos2-pos1).toFixed(2)}s in 3s)`);
    expect(pos2).toBeGreaterThan((pos1 ?? 0) + 1.0);
    expect(pos2).toBeLessThan((pos1 ?? 0) + 6.0);
  });

});
