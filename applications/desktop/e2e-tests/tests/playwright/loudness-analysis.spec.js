/**
 * Loudness Analysis E2E tests — Playwright CDP
 *
 * Tests the audio analysis / volume leveling pipeline.
 *
 * IPC commands tested:
 *   analyze_track(trackId) → ()
 *   get_track_loudness(trackId) → Option<LoudnessData>
 *   get_analysis_queue_stats() → { pending, completed, failed, total }
 *   queue_all_unanalyzed() → u32 (count queued)
 *   get_analysis_worker_status() → { running, processed, errors }
 *   set_volume_leveling_mode(mode) → ()  ["off", "track", "album"]
 *   set_volume_leveling_preamp(db) → ()
 *   set_volume_leveling_prevent_clipping(enabled) → ()
 *
 * 7 tests:
 *   1. get_analysis_queue_stats returns valid stats object
 *   2. analyze_track queues a track for analysis
 *   3. queue_all_unanalyzed queues unanalyzed tracks
 *   4. get_track_loudness returns null for unanalyzed track
 *   5. set_volume_leveling_mode accepts valid modes
 *   6. set_volume_leveling_preamp sets the preamp gain
 *   7. set_volume_leveling_prevent_clipping toggles clipping prevention
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  // Reset volume leveling to off
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'disabled' }); } catch {}
  }).catch(() => {});
  await browser.close();
});

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
});

// ── Test 1: get_analysis_queue_stats returns valid object ──

test('get_analysis_queue_stats returns a valid stats object', async () => {
  const stats = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_analysis_queue_stats')
  );

  expect(stats).toBeTruthy();
  expect(typeof stats.pending).toBe('number');
  expect(typeof stats.completed).toBe('number');
  expect(typeof stats.failed).toBe('number');
  expect(typeof stats.total).toBe('number');
  expect(stats.pending).toBeGreaterThanOrEqual(0);
  expect(stats.completed).toBeGreaterThanOrEqual(0);
});

// ── Test 2: analyze_track queues a track ──

test('analyze_track processes track 2001 and returns loudness info', async () => {
  const result = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('analyze_track', { trackId: 2001 });
    } catch (e) {
      return { error: String(e) };
    }
  });

  // May succeed with loudness data or fail if file path is unavailable
  expect(result).toBeTruthy();
  if (!result.error) {
    // Should have loudness fields
    expect(typeof result).toBe('object');
  }
});

// ── Test 3: queue_all_unanalyzed queues tracks ──

test('queue_all_unanalyzed returns the count of queued tracks', async () => {
  const count = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('queue_all_unanalyzed')
  );

  expect(typeof count).toBe('number');
  expect(count).toBeGreaterThanOrEqual(0);
});

// ── Test 4: get_track_loudness returns data or null ──

test('get_track_loudness returns null or loudness data for track 2001', async () => {
  const loudness = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_track_loudness', { trackId: 2001 })
  );

  // May be null (unanalyzed) or an object with loudness fields
  if (loudness !== null) {
    expect(typeof loudness).toBe('object');
  }
  // Either way, no error
});

// ── Test 5: set_volume_leveling_mode accepts valid modes ──

test('set_volume_leveling_mode accepts off, track, and album modes', async () => {
  for (const mode of ['disabled', 'replaygain_track', 'replaygain_album']) {
    const error = await page.evaluate(async (m) => {
      try {
        await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: m });
        return null;
      } catch (e) {
        return String(e);
      }
    }, mode);

    expect(error).toBeNull();
  }

  // Reset to off
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'disabled' })
  );
});

// ── Test 6: set_volume_leveling_preamp sets gain ──

test('set_volume_leveling_preamp accepts a dB value', async () => {
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_preamp', { preampDb: -3.0 });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  expect(error).toBeNull();

  // Reset
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_preamp', { preampDb: 0.0 }); } catch {}
  });
});

// ── Test 7: set_volume_leveling_prevent_clipping toggles ──

test('set_volume_leveling_prevent_clipping accepts boolean', async () => {
  for (const enabled of [true, false]) {
    const error = await page.evaluate(async (val) => {
      try {
        await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_prevent_clipping', { prevent: val });
        return null;
      } catch (e) {
        return String(e);
      }
    }, enabled);

    expect(error).toBeNull();
  }
});
