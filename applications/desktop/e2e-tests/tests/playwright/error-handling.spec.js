/**
 * Error Handling — Playwright CDP tests
 *
 * Covers graceful handling of playback errors:
 *   1. Playing a track with a missing audio file → state transitions to Stopped or Error
 *   2. App does not crash (UI still responsive after error)
 *   3. After an error, playing a valid track works normally
 *   4. play_queue with empty list doesn't crash the app
 *   5. stop_playback while already stopped doesn't crash
 *   6. Rapid start/stop cycles leave app functional
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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── tests ─────────────────────────────────────────────────────────────────────

test('playing a track with missing file path eventually stops or errors without crash', async () => {
  // Build a queue with a non-existent file path
  await page.evaluate(async () => {
    const queue = [{
      trackId: 'error-test-999',
      title: 'Missing File Track',
      artist: 'Error Test',
      album: null,
      albumId: null,
      filePath: 'C:\\nonexistent\\path\\missing.flac',
      durationSeconds: 3,
      trackNumber: 1,
      coverArtPath: null,
    }];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  // State must settle to either Stopped or Error within 10 seconds
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped' || state === 'Error';
    },
    { timeout: 10_000 }
  );

  // UI must still be interactive — nav links must be visible
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
  await expect(page.locator('[data-testid="nav-tracks"]')).toBeVisible();
});

test('after a playback error, playing a valid album works normally', async () => {
  // Trigger error first
  await page.evaluate(async () => {
    const queue = [{
      trackId: 'error-test-998',
      title: 'Missing File',
      artist: 'Test',
      album: null,
      albumId: null,
      filePath: '/does/not/exist.mp3',
      durationSeconds: 1,
      trackNumber: 1,
      coverArtPath: null,
    }];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 }).catch(() => {});
  });

  // Wait for error/stop
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped' || state === 'Error';
    },
    { timeout: 10_000 }
  );

  // Now play album 2001 normally
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  // Must reach Playing state — waitForFunction resolves only when Playing is seen.
  // We do NOT re-query after this because the 2-second test tracks can complete before
  // the second IPC round-trip, causing a false "Stopped" result.
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // If waitForFunction resolved without throwing, Playing state was confirmed.
  // App is still functional.
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

test('play_queue with empty array does not crash the app', async () => {
  await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue: [], startIndex: 0 });
    } catch {
      // Expected: error may be thrown for empty queue
    }
  });

  // App still functional
  await page.waitForTimeout(500);
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Stopped', 'Error']).toContain(state);
});

test('stop_playback while already stopped does not crash', async () => {
  // Already stopped (beforeEach called stop_playback)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Stopped');

  // Call stop again — must not throw/crash
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('stop_playback');
  });

  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

test('rapid start/stop does not leave app in broken state', async () => {
  // Fire play + stop in quick succession 5 times
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    }));
    for (let i = 0; i < 5; i++) {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 }).catch(() => {});
      await window.__TAURI_INTERNALS__.invoke('stop_playback').catch(() => {});
    }
  });

  await page.waitForTimeout(500);

  // Nav must still work
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="track-list"]')).toBeVisible();
});
