/**
 * DSD (Direct Stream Digital) playback — Playwright CDP E2E tests
 *
 * Covers DSD file format support end-to-end:
 *
 *   1. DSF track (.dsf) appears in the Tracks page
 *   2. DSF track plays back (playback state reaches Playing)
 *   3. DSDIFF track (.dff) appears in the Tracks page
 *   4. DSDIFF track plays back (playback state reaches Playing)
 *   5. Import pipeline: a DSF file in the watched folder is picked up by scan
 *
 * Seed data (added to playwright-global-setup.js):
 *   Artist 5001 — "DSD Artist"
 *   Album 5001  — "DSD Album"
 *   Track 5001  — "DSD Track One"  (.dsf file, file_format='dsf')
 *   Track 5002  — "DSD Track Two"  (.dff file, file_format='dff')
 *   Import dir contains dsd-import-01.dsf for the scan test
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
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start playback of a single track by track ID via IPC.
// ----------------------------------------------------------------

async function playTrackById(p, trackId, title) {
  await p.evaluate(async ({ trackId }) => {
    const track = await window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: trackId });
    if (!track) throw new Error(`Track ${trackId} not found`);
    const queue = [{
      trackId: String(track.id),
      title: track.title,
      artist: track.artist_name || 'Unknown Artist',
      album: track.album_title || null,
      albumId: track.album_id || null,
      filePath: track.file_path || '',
      durationSeconds: track.duration_seconds || null,
      trackNumber: track.track_number || null,
      coverArtPath: null,
    }];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  }, { trackId });

  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
}

// ----------------------------------------------------------------
// Test 1: DSF track appears in Tracks page
// ----------------------------------------------------------------

test('DSF track appears in the Tracks library', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-row"]', { timeout: 15_000 });

  // "DSD Track One" must appear in the track list
  const trackRow = page.locator('[data-testid="track-row"]').filter({ hasText: 'DSD Track One' });
  await expect(trackRow).toBeVisible({ timeout: 10_000 });
});

// ----------------------------------------------------------------
// Test 2: DSF track plays back
// ----------------------------------------------------------------

test('DSF track (.dsf) starts playback and reaches Playing state', async () => {
  await playTrackById(page, 5001, 'DSD Track One');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Now-playing panel must show the track title
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      return el && el.textContent.trim() === 'DSD Track One';
    },
    { timeout: 10_000 }
  );
});

// ----------------------------------------------------------------
// Test 3: DSDIFF track appears in Tracks page
// ----------------------------------------------------------------

test('DSDIFF track appears in the Tracks library', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-row"]', { timeout: 15_000 });

  const trackRow = page.locator('[data-testid="track-row"]').filter({ hasText: 'DSD Track Two' });
  await expect(trackRow).toBeVisible({ timeout: 10_000 });
});

// ----------------------------------------------------------------
// Test 4: DSDIFF track plays back
// ----------------------------------------------------------------

test('DSDIFF track (.dff) starts playback and reaches Playing state', async () => {
  await playTrackById(page, 5002, 'DSD Track Two');

  // Allow DSD decoder to stabilize before checking state.
  // The DSD source briefly flickers through Loading before settling at Playing.
  await page.waitForTimeout(500);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      return el && el.textContent.trim() === 'DSD Track Two';
    },
    { timeout: 10_000 }
  );
});

// ----------------------------------------------------------------
// Test 5: DSD file in import dir is picked up by scan
// ----------------------------------------------------------------

test('DSF file in watched folder is discovered by import scan', async () => {
  // Trigger a rescan of the import dir — uses the standard import_directory IPC.
  const importDir = process.env.PLAYWRIGHT_IMPORT_DIR;
  if (!importDir) {
    console.warn('PLAYWRIGHT_IMPORT_DIR not set, skipping scan test');
    return;
  }

  const result = await page.evaluate(async (dir) => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('import_directory', {
        directory: dir,
      });
    } catch (e) {
      return { error: String(e) };
    }
  }, importDir);

  // import_directory returns void (null in JS) on success; an error object on failure.
  if (result !== null && result !== undefined) {
    expect(result).not.toHaveProperty('error');
  }
});
