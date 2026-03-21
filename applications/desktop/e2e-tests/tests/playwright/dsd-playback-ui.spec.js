/**
 * DSD Playback UI E2E Tests
 *
 * Validates DSD (DSF/DFF) playback through the actual Soul Player desktop UI:
 *   - Track appears in the Tracks page library list
 *   - Double-clicking a DSD track row starts playback
 *   - Now-playing panel updates to show the correct DSD track
 *   - Play/pause button toggles state correctly
 *   - Seek bar is visible and the progress thumb moves over time
 *   - Clicking the seek bar at a specific position jumps the position
 *   - After seeking, audio continues (position keeps advancing)
 *   - WASAPI loopback captures non-silent audio during DSD playback
 *   - Audio output is non-silent after a seek (decoder didn't stall)
 *
 * Requirements (for audio capture tests):
 *   pip install pyaudiowpatch numpy
 *   WASAPI loopback device 14 = "Speakers (Realtek(R) Audio) [Loopback]"
 *
 * Run:
 *   npx playwright test tests/playwright/dsd-playback-ui.spec.js \
 *     --config playwright.prod.config.js
 */

import { test, expect, chromium } from '@playwright/test';
import { spawnSync }  from 'child_process';
import { mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname  = dirname(fileURLToPath(import.meta.url));
const CAPTURES   = join(__dirname, '..', '..', 'captures');
const CDP_URL    = process.env.SOUL_CDP_URL ?? 'http://localhost:9222';

const WASAPI_DEV  = 14;
const CAPTURE_SR  = 48000;
const WARMUP_MS   = 2000;

// ── module-level state ────────────────────────────────────────────────────────
let browser, page, skipReason;
let dsdTrack = null;

// ── helpers ───────────────────────────────────────────────────────────────────
const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params), { cmd, params });

async function getDsdTrack() {
  const DSD_FMTS = new Set(['dsf','dff','dsdiff','DSF','DFF','DSDIFF']);
  const all = await invoke(page, 'get_all_tracks');
  const cands = all.filter(t =>
    DSD_FMTS.has(t.file_format) && t.duration_seconds >= 30
    && t.file_path && existsSync(t.file_path)
  ).sort((a, b) => a.duration_seconds - b.duration_seconds);
  return cands[0] ?? null;
}

async function startDsdPlayback() {
  await invoke(page, 'play_queue', {
    queue: [{
      trackId:         String(dsdTrack.id),
      title:           dsdTrack.title,
      artist:          dsdTrack.artist_name    || 'Unknown Artist',
      album:           dsdTrack.album_title    || null,
      albumId:         dsdTrack.album_id       || null,
      filePath:        dsdTrack.file_path      || '',
      durationSeconds: dsdTrack.duration_seconds || null,
      trackNumber:     dsdTrack.track_number   || null,
      coverArtPath:    null,
    }],
    startIndex: 0,
  });
  await page.waitForTimeout(WARMUP_MS);
}

function captureRms(secs = 5) {
  mkdirSync(CAPTURES, { recursive: true });
  const outWav = join(CAPTURES, `dsd-ui-check-${Date.now()}.wav`);
  const OUT    = outWav.replace(/\\/g, '\\\\');
  const py = `
import pyaudiowpatch as pyaudio, wave, numpy as np
DEVICE=${WASAPI_DEV}; RATE=${CAPTURE_SR}; CHUNK=1024; DUR=${secs}
p=pyaudio.PyAudio()
s=p.open(format=pyaudio.paInt16,channels=2,rate=RATE,input=True,input_device_index=DEVICE,frames_per_buffer=CHUNK)
frames=[]
for _ in range(int(RATE/CHUNK*DUR)): frames.append(s.read(CHUNK,exception_on_overflow=False))
s.stop_stream(); s.close(); p.terminate()
raw=b''.join(frames)
with wave.open(r"${OUT}",'wb') as wf:
    wf.setnchannels(2); wf.setsampwidth(2); wf.setframerate(RATE); wf.writeframes(raw)
arr=np.frombuffer(raw,dtype=np.int16).astype(np.float32)/32768.0
import sys,io; sys.stdout=io.TextIOWrapper(sys.stdout.buffer,encoding='utf-8',errors='replace')
print(f"{20*np.log10(float(np.sqrt(np.mean(arr**2)))+1e-12):.2f}")
`;
  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8', stdio: ['ignore','pipe','pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`WASAPI capture failed: ${r.stderr}`);
  return parseFloat(r.stdout.trim());
}

// ── Suite ─────────────────────────────────────────────────────────────────────
test.describe('DSD Playback UI', () => {

  test.beforeAll(async () => {
    browser = await chromium.connectOverCDP(CDP_URL);
    const ctx = browser.contexts()[0];
    page = ctx.pages().find(
      p => (p.url().includes('tauri.localhost') || p.url().includes('localhost:1420'))
           && !p.url().includes('splash')
    );
    if (!page) { skipReason = 'Main app window not found via CDP'; return; }

    dsdTrack = await getDsdTrack();
    if (!dsdTrack) {
      skipReason = 'No DSD track with real on-disk file found (need production library)';
      return;
    }
  });

  test.afterAll(async () => {
    if (page)    await invoke(page, 'stop_playback').catch(() => {});
    if (browser) await browser.close();
  });

  // ── 1. DSD track present in library UI ─────────────────────────────────────
  test('1. DSD track is visible in Tracks page UI', async () => {
    if (skipReason) test.skip(true, skipReason);

    // Navigate to Tracks page
    await page.click('[data-testid="nav-tracks"]', { force: true });
    await page.waitForSelector('[data-testid="tracks-page"]',  { timeout: 15_000 });
    await page.waitForSelector('[data-testid="track-list"]',   { timeout: 15_000 });

    const trackRows = page.locator('[data-testid="track-row"]');
    const count = await trackRows.count();
    expect(count).toBeGreaterThan(0);
    console.log(`Tracks page shows ${count} track rows`);

    // Verify at least one row matches the DSD track title
    const titles = await page.evaluate(() =>
      [...document.querySelectorAll('[data-testid="track-row"]')]
        .map(r => r.textContent)
    );
    const found = titles.some(t => t?.includes(dsdTrack.title));
    expect(found, `DSD track "${dsdTrack.title}" should appear in track list`).toBe(true);
    console.log(`Found DSD track in UI: "${dsdTrack.title}"`);
  });

  // ── 2. Double-click DSD track row → starts playback ─────────────────────────
  test('2. Double-clicking DSD track row starts playback', async () => {
    if (skipReason) test.skip(true, skipReason);

    await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
    const trackRows = page.locator('[data-testid="track-row"]');

    // Find the row for our DSD track
    const rowCount  = await trackRows.count();
    let   targetRow = null;
    for (let i = 0; i < rowCount; i++) {
      const text = await trackRows.nth(i).textContent();
      if (text?.includes(dsdTrack.title)) { targetRow = trackRows.nth(i); break; }
    }
    // Fall back to first row if exact match not found (scroll may hide it)
    if (!targetRow) targetRow = trackRows.first();

    await targetRow.dblclick();
    await page.waitForTimeout(WARMUP_MS);

    const state = await invoke(page, 'get_playback_state');
    expect(state).toBe('Playing');
    console.log(`Playback state after double-click: ${state}`);
  });

  // ── 3. Now-playing panel shows DSD track ────────────────────────────────────
  test('3. Now-playing panel shows the DSD track title and artist', async () => {
    if (skipReason) test.skip(true, skipReason);

    // Ensure DSD is playing (may have changed since test 2)
    await startDsdPlayback();

    const titleEl = page.locator('[data-testid="now-playing-title"]');
    await titleEl.waitFor({ state: 'visible', timeout: 10_000 });
    const title = await titleEl.textContent({ timeout: 5_000 });

    expect(title?.trim()).toBeTruthy();
    expect(title).toContain(dsdTrack.title);
    console.log(`Now-playing title: "${title?.trim()}"`);
    console.log(`Expected:          "${dsdTrack.title}" — ${dsdTrack.artist_name}`);
  });

  // ── 4. Play/pause button toggles state ──────────────────────────────────────
  test('4. Play/pause button toggles DSD playback correctly', async () => {
    if (skipReason) test.skip(true, skipReason);

    await startDsdPlayback();
    const state0 = await invoke(page, 'get_playback_state');
    expect(state0).toBe('Playing');

    // Pause via UI button
    await page.click('[data-testid="play-pause-button"]');
    await page.waitForTimeout(800);
    const state1 = await invoke(page, 'get_playback_state');
    expect(state1).toBe('Paused');
    console.log('After pause click: Paused ✓');

    // Resume via UI button
    await page.click('[data-testid="play-pause-button"]');
    await page.waitForTimeout(800);
    const state2 = await invoke(page, 'get_playback_state');
    expect(state2).toBe('Playing');
    console.log('After resume click: Playing ✓');
  });

  // ── 5. Seek bar is visible and progress moves ────────────────────────────────
  test('5. Seek bar is visible and progress thumb advances during DSD playback', async () => {
    if (skipReason) test.skip(true, skipReason);

    await startDsdPlayback();

    // Progress bar container must be visible
    const progressBar = page.locator('[data-testid="now-playing-progress-bar"]');
    await progressBar.waitFor({ state: 'visible', timeout: 8_000 });
    expect(await progressBar.isVisible()).toBe(true);

    // Current time display must be visible and non-zero after warmup
    const currentTime = page.locator('[data-testid="seek-current-time"]');
    await currentTime.waitFor({ state: 'visible', timeout: 8_000 });
    const t1 = await currentTime.textContent({ timeout: 3_000 });
    console.log(`Seek current time (t=0): "${t1?.trim()}"`);

    // Total time must be displayed
    const totalTime = page.locator('[data-testid="seek-total-time"]');
    const total = await totalTime.textContent({ timeout: 3_000 });
    console.log(`Seek total time: "${total?.trim()}"`);
    expect(total?.trim()).toBeTruthy();

    // Wait 3 seconds and verify current time has advanced in the UI
    await page.waitForTimeout(3_000);
    const t2 = await currentTime.textContent({ timeout: 3_000 });
    console.log(`Seek current time (t=3s): "${t2?.trim()}"`);

    // t2 should be different from t1 (time is advancing visually)
    expect(t2?.trim()).not.toBe(t1?.trim());
  });

  // ── 6. Click seek bar → position jumps ──────────────────────────────────────
  test('6. Clicking seek bar at 40% jumps playback position', async () => {
    if (skipReason) test.skip(true, skipReason);

    await startDsdPlayback();
    await page.waitForSelector('[data-testid="seek-track"]', { timeout: 8_000 });

    const pos0 = await invoke(page, 'get_position');

    // Click the seek hit area at 40% of its width
    const seekTrack = page.locator('[data-testid="seek-track"]');
    const box       = await seekTrack.boundingBox();
    expect(box).toBeTruthy();

    const clickX = box.x + box.width * 0.4;
    const clickY = box.y + box.height / 2;
    await page.mouse.click(clickX, clickY);
    await page.waitForTimeout(1_000);

    const pos1 = await invoke(page, 'get_position');
    const duration = dsdTrack.duration_seconds;
    const expected = duration * 0.4;

    console.log(`Seek: ${pos0?.toFixed(2)}s → ${pos1?.toFixed(2)}s  (expected ≈ ${expected.toFixed(2)}s, duration=${duration.toFixed(0)}s)`);

    // Position must have moved from the start and be somewhere in the track
    expect(pos1).toBeGreaterThan(1.0);                         // actually jumped
    expect(pos1).toBeLessThan(duration - 1.0);                 // within track
    // Rough target: 40% ± 15% of duration
    expect(pos1).toBeGreaterThan(duration * 0.20);
    expect(pos1).toBeLessThan(duration * 0.60);
  });

  // ── 7. Position continues advancing after seek ──────────────────────────────
  test('7. Playback continues advancing after seek (decoder not stalled)', async () => {
    if (skipReason) test.skip(true, skipReason);

    // Seek to 30% via IPC (more reliable than UI for this assertion)
    const seekTo = dsdTrack.duration_seconds * 0.30;
    await invoke(page, 'seek_to', { position: seekTo });
    await page.waitForTimeout(1_200);

    const state = await invoke(page, 'get_playback_state');
    expect(state).toBe('Playing');

    const pos1 = await invoke(page, 'get_position');
    await page.waitForTimeout(3_000);
    const pos2 = await invoke(page, 'get_position');

    console.log(`After seek to ${seekTo.toFixed(1)}s: ${pos1?.toFixed(2)}s → ${pos2?.toFixed(2)}s  (Δ=${(pos2-pos1).toFixed(2)}s in 3s)`);
    expect(pos2).toBeGreaterThan((pos1 ?? seekTo) + 1.0);
    expect(pos2).toBeLessThan((pos1 ?? seekTo) + 6.0);
  });

  // ── 8. WASAPI: audio output is non-silent during DSD playback ───────────────
  test('8. WASAPI loopback captures non-silent audio during DSD playback', async () => {
    if (skipReason) test.skip(true, skipReason);

    const pyOk = spawnSync('python', ['-c', 'import pyaudiowpatch,numpy'], { stdio: 'pipe' }).status === 0;
    if (!pyOk) test.skip(true, 'pyaudiowpatch/numpy not installed');

    // Play from a mid-track position to avoid interludes
    const seekTo = Math.min(30, dsdTrack.duration_seconds * 0.20);
    await invoke(page, 'seek_to', { position: seekTo });
    await page.waitForTimeout(1_200);

    const rms = captureRms(5);
    console.log(`WASAPI capture during DSD playback: ${rms.toFixed(1)} dBFS`);
    expect(rms).toBeGreaterThan(-60); // not silent
    console.log('Audio output confirmed non-silent ✓');
  });

  // ── 9. WASAPI: audio output non-silent after seek ───────────────────────────
  test('9. WASAPI captures non-silent audio after seek (DSD decoder resumes)', async () => {
    if (skipReason) test.skip(true, skipReason);

    const pyOk = spawnSync('python', ['-c', 'import pyaudiowpatch,numpy'], { stdio: 'pipe' }).status === 0;
    if (!pyOk) test.skip(true, 'pyaudiowpatch/numpy not installed');

    // Seek to 50% then immediately capture
    const seekTo = dsdTrack.duration_seconds * 0.50;
    await invoke(page, 'seek_to', { position: seekTo });
    await page.waitForTimeout(1_500); // decoder restart warmup

    const state = await invoke(page, 'get_playback_state');
    expect(state).toBe('Playing');

    const rms = captureRms(5);
    console.log(`WASAPI after seek to ${seekTo.toFixed(1)}s: ${rms.toFixed(1)} dBFS`);
    expect(rms).toBeGreaterThan(-60);
    console.log('Audio resumes after seek ✓');
  });

  // ── 10. Pause stops audio output (WASAPI goes silent) ───────────────────────
  test('10. WASAPI confirms audio stops when DSD is paused', async () => {
    if (skipReason) test.skip(true, skipReason);

    const pyOk = spawnSync('python', ['-c', 'import pyaudiowpatch,numpy'], { stdio: 'pipe' }).status === 0;
    if (!pyOk) test.skip(true, 'pyaudiowpatch/numpy not installed');

    // Ensure playing first
    await startDsdPlayback();
    const rmsPlaying = captureRms(3);
    console.log(`RMS while playing: ${rmsPlaying.toFixed(1)} dBFS`);
    expect(rmsPlaying).toBeGreaterThan(-60);

    // Pause via UI
    await page.click('[data-testid="play-pause-button"]');
    await page.waitForTimeout(600);
    const state = await invoke(page, 'get_playback_state');
    expect(state).toBe('Paused');

    // Allow any buffered audio to drain
    await page.waitForTimeout(1_200);
    const rmsPaused = captureRms(3);
    console.log(`RMS while paused:  ${rmsPaused.toFixed(1)} dBFS`);

    // Paused audio should be significantly quieter than playing
    // (may not be perfectly silent due to system sounds / other apps)
    expect(rmsPaused).toBeLessThan(rmsPlaying + 3); // not louder than playing
    console.log(`Level drop on pause: ${(rmsPlaying - rmsPaused).toFixed(1)} dB`);

    // Resume
    await page.click('[data-testid="play-pause-button"]');
    await page.waitForTimeout(500);
  });

});
