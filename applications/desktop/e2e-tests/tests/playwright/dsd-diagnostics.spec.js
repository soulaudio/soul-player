/**
 * DSD Diagnostics IPC — Playwright CDP E2E tests
 *
 * Verifies the `get_dsd_diagnostics` Tauri IPC command that exposes
 * real-time DSD ring-buffer health metrics: underrun count, buffer fill,
 * and decoder running state.
 *
 * Tests use Track 5003 (DSF, ~60s) seeded in playwright-global-setup.js
 * under Album 5002 "DSD Long Album".  Track 5004 is the DFF variant.
 *
 * IPC commands used:
 *   play_queue({ queue, startIndex })   — start playback
 *   stop_playback()                     — stop and reset
 *   seek_to({ position })               — seek to absolute position (seconds)
 *   get_playback_state()                — "Playing" | "Paused" | "Stopped"
 *   get_position()                      — current position (f64 seconds)
 *   get_track_by_id({ id })             — fetch single track record
 *   get_dsd_diagnostics()               — DSD buffer health snapshot
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

// ── helpers ──────────────────────────────────────────────────────────────────

const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(
    ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params },
  );

async function playTrack5003(page) {
  const track = await invoke(page, 'get_track_by_id', { id: 5003 });
  await invoke(page, 'play_queue', {
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
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 10_000 },
  );
}

async function getDiag(pg) {
  return invoke(pg, 'get_dsd_diagnostics');
}

// ── suite setup ──────────────────────────────────────────────────────────────

test.beforeAll(async () => {
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
  await browser.close();
});

test.beforeEach(async () => {
  await invoke(page, 'stop_playback').catch(() => {});
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await invoke(page, 'stop_playback').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Test 1: Buffer fill ≥ 20% throughout 5s ──────────────────────────────────

test.describe('DSD Diagnostics IPC', () => {

  test('buffer fill remains ≥ 20% throughout 5s of playback', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    // Allow ~1s for the ring buffer to pre-fill before asserting
    await page.waitForTimeout(1_000);

    const pollCount = 10; // 10 polls × 500ms = 5s
    for (let i = 0; i < pollCount; i++) {
      const diag = await getDiag(page);
      expect(diag).not.toBeNull();
      expect(diag.buffer_fill_percent).toBeGreaterThanOrEqual(20);
      await page.waitForTimeout(500);
    }
  });

  // ── Test 2: Zero underruns during 5s ───────────────────────────────────────

  test('zero underruns during 5s of steady playback', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    // Let it play for 5s
    await page.waitForTimeout(5_000);

    const diag = await getDiag(page);
    expect(diag).not.toBeNull();
    expect(diag.underrun_count).toBe(0);
  });

  // ── Test 3: Seek accuracy at [15, 30, 45]s ─────────────────────────────────

  test('seek accuracy within 0.5s at positions [15, 30, 45]s', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    const targets = [15, 30, 45];
    for (const target of targets) {
      await invoke(page, 'seek_to', { position: target });

      // Wait up to 1s for position to settle within 0.5s of the target
      await page.waitForFunction(
        async (t) => {
          const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
          return Math.abs(pos - t) < 0.5;
        },
        target,
        { timeout: 1_000 },
      );

      const pos = await invoke(page, 'get_position');
      expect(Math.abs(pos - target)).toBeLessThan(0.5);
    }
  });

  // ── Test 4: ≤2 underruns after 5 rapid seeks ──────────────────────────────

  test('≤2 underruns after 5 rapid seeks', async () => {
    test.setTimeout(45_000);

    await playTrack5003(page);

    // Allow pre-fill before stressing with seeks
    await page.waitForTimeout(1_000);

    const seekTargets = [5, 20, 40, 10, 50];
    for (const target of seekTargets) {
      await invoke(page, 'seek_to', { position: target });
      await page.waitForTimeout(500);
    }

    // Wait 2s for ring buffer to stabilise after the seek storm
    await page.waitForTimeout(2_000);

    const diag = await getDiag(page);
    expect(diag).not.toBeNull();
    // Seeks flush the ring buffer and may cause brief underruns — allow up to 2
    expect(diag.underrun_count).toBeLessThanOrEqual(2);
  });

  // ── Test 5: Position monotonically advances for 10s ───────────────────────

  test('position advances monotonically for 10s', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    let previousPos = await invoke(page, 'get_position');
    const pollCount = 100; // 100 polls × 100ms = 10s

    for (let i = 0; i < pollCount; i++) {
      await page.waitForTimeout(100);
      const currentPos = await invoke(page, 'get_position');
      expect(currentPos).toBeGreaterThanOrEqual(previousPos);
      previousPos = currentPos;
    }
  });

  // ── Test 6: Playback resumes within 500ms after seek ──────────────────────

  test('playback resumes within 500ms after seek to 30s', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    await invoke(page, 'seek_to', { position: 30 });

    // Wait for seek to land
    await page.waitForFunction(
      async () => {
        const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
        return Math.abs(pos - 30) < 1.0;
      },
      { timeout: 5_000 },
    );

    const posA = await invoke(page, 'get_position');
    await page.waitForTimeout(500);
    const posB = await invoke(page, 'get_position');

    // Position must have advanced within the 500ms window
    expect(posB).toBeGreaterThan(posA);
  });

  // ── Test 7: Decoder stays running for 30s ─────────────────────────────────

  test('decoder_running stays true for 30s of playback', async () => {
    test.setTimeout(60_000);

    await playTrack5003(page);

    const pollCount = 30; // 30 polls × 1s = 30s
    for (let i = 0; i < pollCount; i++) {
      const diag = await getDiag(page);
      expect(diag).not.toBeNull();
      expect(diag.decoder_running).toBe(true);
      await page.waitForTimeout(1_000);
    }
  });

  // ── Test 8: Underrun counter resets on new play_queue ─────────────────────

  test('underrun_count resets to 0 after stop and new play_queue', async () => {
    test.setTimeout(30_000);

    await playTrack5003(page);

    // Let it play for 3s to accumulate a baseline
    await page.waitForTimeout(3_000);

    await invoke(page, 'stop_playback');
    await page.waitForTimeout(200);

    // Start playback again
    await playTrack5003(page);

    // Immediately after starting the new queue, the counter should be reset
    const diag = await getDiag(page);
    expect(diag).not.toBeNull();
    expect(diag.underrun_count).toBe(0);
  });

  // ── Test 9: DFF format (Track 5004) — fill ≥ 20% and zero underruns ───────

  test('DFF format (track 5004): buffer fill ≥ 20% and zero underruns after 5s', async () => {
    test.setTimeout(30_000);

    // Play DFF track 5004
    const track = await invoke(page, 'get_track_by_id', { id: 5004 });
    await invoke(page, 'play_queue', {
      queue: [{
        trackId:         String(track.id),
        filePath:        track.file_path,
        title:           track.title,
        artist:          track.artist,
        durationSeconds: track.duration_seconds,  // camelCase
        coverArtPath:    null,
      }],
      startIndex: 0,
    });
    await page.waitForFunction(
      async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
      { timeout: 10_000 },
    );

    // Allow ~1s for ring buffer to pre-fill
    await page.waitForTimeout(1_000);

    // Assert fill is healthy
    const diagFill = await getDiag(page);
    expect(diagFill).not.toBeNull();
    expect(diagFill.buffer_fill_percent).toBeGreaterThanOrEqual(20);

    // Play for 5s total then check underruns
    await page.waitForTimeout(4_000);

    const diagFinal = await getDiag(page);
    expect(diagFinal).not.toBeNull();
    expect(diagFinal.underrun_count).toBe(0);
  });

});
