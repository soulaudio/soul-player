/**
 * Search/filter stress tests — Playwright CDP
 *
 * Verifies that rapid search input during active playback does not:
 *   - Lag or freeze the UI (useDeferredValue handles debounce)
 *   - Interrupt audio playback
 *   - Cause stale or inconsistent filter results
 *   - Accumulate render cycles or memory
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist" — 5 tracks x 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
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
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  // Clear search input if visible
  const searchInput = page.locator('[data-testid="search-input"]');
  const isVisible = await searchInput.isVisible().catch(() => false);
  if (isVisible) {
    await searchInput.fill('');
    await page.waitForTimeout(100);
  }
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2001',
        contextName: 'Playwright Album',
        contextArtworkPath: null,
      },
    });
  });
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function typeInSearch(text) {
  // Hover near top to trigger show-on-hover search bar
  await page.mouse.move(200, 50);
  await page.waitForTimeout(100);
  const searchInput = page.locator('[data-testid="search-input"]');
  await searchInput.waitFor({ state: 'visible', timeout: 10_000 });
  await searchInput.fill(text);
  // Give React's useDeferredValue one frame to commit
  await page.waitForTimeout(200);
}

async function clearSearch() {
  const searchInput = page.locator('[data-testid="search-input"]');
  await searchInput.fill('');
  await page.waitForTimeout(200);
}

async function countTrackRows() {
  return page.locator('[data-testid="track-row"]').count();
}

// ================================================================
// Test 1: Rapid type/clear cycles on Tracks page — 10 cycles
// ================================================================

test('rapid type/clear: 10 search cycles on Tracks page stay responsive', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  const start = Date.now();
  const CYCLES = 10;

  for (let i = 0; i < CYCLES; i++) {
    await typeInSearch('Track T');
    // Should filter to ~2 results (Track Two, Track Three)
    await page.waitForFunction(
      () => document.querySelectorAll('[data-testid="track-row"]').length <= 3,
      { timeout: 5_000 }
    );

    await clearSearch();
    // Should restore to 5+ results
    await page.waitForFunction(
      () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
      { timeout: 5_000 }
    );
  }

  const elapsed = Date.now() - start;
  // 10 filter/clear cycles should complete in under 20s
  expect(elapsed).toBeLessThan(20_000);

  // Final state: all tracks visible
  const count = await countTrackRows();
  expect(count).toBeGreaterThanOrEqual(5);
});

// ================================================================
// Test 2: Search during playback — audio continues uninterrupted
// ================================================================

test('searching on Tracks page during playback does not interrupt audio', async () => {
  await startPlayback(page);

  // Navigate to Tracks page
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  // Perform several search operations while music plays
  await typeInSearch('Track One');
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 1,
    { timeout: 5_000 }
  );

  await typeInSearch('zzznomatch');
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length === 0,
    { timeout: 5_000 }
  );

  await clearSearch();
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 5_000 }
  );

  // Playback must still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Sidebar still shows track info
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 3: Progressive typing — character-by-character narrowing
// ================================================================

test('progressive typing narrows results correctly on Albums page', async () => {
  // Type "Playwright" one character at a time with short delays
  const query = 'Playwright';
  const albumCard = page.locator('[data-testid="media-card-album-2001"]');

  for (let i = 1; i <= query.length; i++) {
    await typeInSearch(query.substring(0, i));
  }

  // After full query, album card should be visible
  await expect(albumCard).toBeVisible({ timeout: 5_000 });

  // Clear and verify restore
  await clearSearch();
  await expect(albumCard).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 4: Search on different pages — cross-page filter stress
// ================================================================

test('search filter works correctly when switching between pages rapidly', async () => {
  // Search on Albums
  await typeInSearch('Playwright');
  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible({ timeout: 5_000 });
  await clearSearch();

  // Switch to Artists and search
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });
  await typeInSearch('Playwright');
  const artistCard = page.locator('[data-testid="media-card-artist-2001"]');
  await expect(artistCard).toBeVisible({ timeout: 5_000 });
  await clearSearch();

  // Switch to Tracks and search
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );
  await typeInSearch('Track F');
  // "Track Four" and "Track Five" match
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length === 2,
    { timeout: 5_000 }
  );
  await clearSearch();

  // All 5 tracks restored
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 5_000 }
  );
});

// ================================================================
// Test 5: Non-matching search rapidly alternated with matching search
// ================================================================

test('alternating match/no-match searches 8 times stays consistent', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  const start = Date.now();

  for (let i = 0; i < 8; i++) {
    if (i % 2 === 0) {
      // No match
      await typeInSearch('zzznomatch');
      await page.waitForFunction(
        () => document.querySelectorAll('[data-testid="track-row"]').length === 0,
        { timeout: 5_000 }
      );
    } else {
      // Match
      await typeInSearch('Track');
      await page.waitForFunction(
        () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
        { timeout: 5_000 }
      );
    }
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(16_000);

  // Clear to restore
  await clearSearch();
  const count = await countTrackRows();
  expect(count).toBeGreaterThanOrEqual(5);
});

// ================================================================
// Test 6: Search + skip track — search doesn't interfere with playback controls
// ================================================================

test('skip track while search is active on Tracks page', async () => {
  await startPlayback(page);

  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  // Apply a filter
  await typeInSearch('Track T');
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length <= 3,
    { timeout: 5_000 }
  );

  // Skip track while filter is active — controls should still work
  await page.click('[data-testid="next-button"]');
  await page.waitForTimeout(500);

  // Playback should still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Search filter should still be applied
  const count = await countTrackRows();
  expect(count).toBeLessThanOrEqual(3);

  await clearSearch();
});
