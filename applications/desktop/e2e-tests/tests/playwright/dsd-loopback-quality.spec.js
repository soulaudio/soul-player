/**
 * DSD Loopback Quality Suite — Playwright CDP E2E tests
 *
 * Captures WASAPI loopback audio while Track 5003 (DSF, ~60s) is playing and
 * checks for silence gaps and RMS level drops using analyze_dsd_audio.py.
 *
 * Prerequisites (auto-detected in beforeAll — suite skipped if any absent):
 *   - python in PATH
 *   - pyaudiowpatch importable
 *   - At least one WASAPI loopback device available
 *
 * IPC commands used:
 *   play_queue({ queue, startIndex })   — start playback
 *   stop_playback()                     — stop and reset
 *   seek_to({ position })               — seek to absolute position (seconds)
 *   get_playback_state()                — "Playing" | "Paused" | "Stopped"
 *   get_track_by_id({ id })             — fetch single track record
 */

import { test, expect, chromium } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CDP_URL } from '../../playwright-global-setup.js';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const SCRIPT = resolve(__dirname, '../../scripts/analyze_dsd_audio.py');

/** Milliseconds to wait after play_queue before starting capture. */
const WARMUP_MS = 1500;

let browser;
let page;

// ── helpers ──────────────────────────────────────────────────────────────────

const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(
    ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params },
  );

async function playTrack5003(pg) {
  const track = await invoke(pg, 'get_track_by_id', { id: 5003 });
  await invoke(pg, 'play_queue', {
    queue: [{
      trackId:         String(track.id),
      filePath:        track.file_path,
      title:           track.title,
      artist:          track.artist,
      durationSeconds: track.duration_seconds,  // camelCase — matches TrackData serde field
      coverArtPath:    null,
    }],
    startIndex: 0,
  });
  await pg.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 10_000 },
  );
}

/**
 * Run analyze_dsd_audio.py with the given extra arguments.
 * Returns the spawnSync result.
 */
function runAnalyzer(extraArgs) {
  return spawnSync('python', [SCRIPT, ...extraArgs], {
    encoding: 'utf8',
    timeout:  60_000,
  });
}

// ── suite setup ──────────────────────────────────────────────────────────────

test.beforeAll(async () => {
  // ── Prerequisites check ────────────────────────────────────────────────────

  // 1. python in PATH
  const pyVersion = spawnSync('python', ['--version'], { encoding: 'utf8', timeout: 5_000 });
  if (pyVersion.status !== 0 && pyVersion.error) {
    test.skip(true, 'python not found in PATH — loopback tests skipped');
    return;
  }

  // 2. pyaudiowpatch importable
  const pyImport = spawnSync(
    'python', ['-c', 'import pyaudiowpatch'],
    { encoding: 'utf8', timeout: 5_000 },
  );
  if (pyImport.status !== 0) {
    test.skip(true, 'pyaudiowpatch not installed — loopback tests skipped');
    return;
  }

  // 3. At least one loopback device
  const pyDevice = spawnSync(
    'python',
    ['-c', 'import pyaudiowpatch; assert pyaudiowpatch.get_loopback_device_info_list()'],
    { encoding: 'utf8', timeout: 5_000 },
  );
  if (pyDevice.status !== 0) {
    test.skip(true, 'No WASAPI loopback devices found — loopback tests skipped');
    return;
  }

  // ── CDP browser connection ─────────────────────────────────────────────────
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  if (browser) await browser.close();
});

test.beforeEach(async () => {
  if (!page) return;
  await invoke(page, 'stop_playback').catch(() => {});
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  if (!page) return;
  await invoke(page, 'stop_playback').catch(() => {});
  await page.waitForTimeout(200);
});

// ── tests ─────────────────────────────────────────────────────────────────────

test.describe('DSD Loopback Quality', () => {

  // ── Test 1: No silence gap > 50ms during 10s ─────────────────────────────

  test('no silence gap > 50ms during 10s steady playback', async () => {
    test.setTimeout(60_000);

    await playTrack5003(page);
    await page.waitForTimeout(WARMUP_MS);

    const result = runAnalyzer([
      '--duration', '10',
      '--mode', 'silence',
      '--silence-threshold-ms', '50',
    ]);

    expect(
      result.status,
      `analyzer stdout: ${result.stdout}\nanalyzer stderr: ${result.stderr}`,
    ).toBe(0);
  });

  // ── Test 2: RMS level consistent during 10s ───────────────────────────────

  test('RMS level consistent (≤12 dB drop) during 10s steady playback', async () => {
    test.setTimeout(60_000);

    await playTrack5003(page);
    await page.waitForTimeout(WARMUP_MS);

    const result = runAnalyzer([
      '--duration', '10',
      '--mode', 'rms',
      '--rms-window-ms', '100',
      '--rms-max-drop-db', '12',
    ]);

    expect(
      result.status,
      `analyzer stdout: ${result.stdout}\nanalyzer stderr: ${result.stderr}`,
    ).toBe(0);
  });

  // ── Test 3: No silence gap > 100ms after seek to 30s ─────────────────────

  test('no silence gap > 100ms after seek to 30s', async () => {
    test.setTimeout(60_000);

    await playTrack5003(page);
    await page.waitForTimeout(WARMUP_MS);

    await invoke(page, 'seek_to', { position: 30 });
    // Wait 1s for the ring buffer to refill after the seek flush
    await page.waitForTimeout(1_000);

    const result = runAnalyzer([
      '--duration', '5',
      '--mode', 'silence',
      '--silence-threshold-ms', '100',
    ]);

    expect(
      result.status,
      `analyzer stdout: ${result.stdout}\nanalyzer stderr: ${result.stderr}`,
    ).toBe(0);
  });

  // ── Test 4: Seek stress — 5 seeks, each followed by a 3s capture ─────────

  test('seek stress: no silence gap > 100ms at each of [5, 15, 25, 35, 45]s', async () => {
    test.setTimeout(120_000);

    await playTrack5003(page);
    await page.waitForTimeout(WARMUP_MS);

    const seekTargets = [5, 15, 25, 35, 45];

    for (const target of seekTargets) {
      await invoke(page, 'seek_to', { position: target });
      // Wait 1s for the ring buffer to refill after the seek flush
      await page.waitForTimeout(1_000);

      const result = runAnalyzer([
        '--duration', '3',
        '--mode', 'silence',
        '--silence-threshold-ms', '100',
      ]);

      expect(
        result.status,
        `seek to ${target}s — analyzer stdout: ${result.stdout}\nanalyzer stderr: ${result.stderr}`,
      ).toBe(0);
    }
  });

  // ── Test 5: 30s continuous playback — zero dropout windows ───────────────

  test('30s continuous playback — no silence gap > 50ms', async () => {
    test.setTimeout(90_000);

    await playTrack5003(page);
    await page.waitForTimeout(WARMUP_MS);

    const result = runAnalyzer([
      '--duration', '30',
      '--mode', 'silence',
      '--silence-threshold-ms', '50',
    ]);

    expect(
      result.status,
      `analyzer stdout: ${result.stdout}\nanalyzer stderr: ${result.stderr}`,
    ).toBe(0);
  });

});
