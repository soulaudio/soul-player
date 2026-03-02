/**
 * Album detail page — Playwright CDP E2E tests
 *
 * Covers the album detail page layout, track listing, and playback flows:
 *
 *   1. Album detail page loads correctly (title, artist, track count)
 *   2. Track list shows all 5 seeded tracks
 *   3. "Play All" button starts playback from the first track
 *   4. Double-clicking a specific track row starts playback from that track
 *   5. Next button works while the album detail page is displayed
 *   6. Album info matches seeded data (title + track count)
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — Artist "Playwright Artist"
 *   Track IDs 2001–2005, titles: Track One … Track Five (2-second WAV files)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  // Find the main window — it is already loaded by the time tests run.
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  // Short safety wait in case there is any residual animation or settling
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: stop any active playback, dismiss open overlays, navigate to Albums.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums list — use force:true so the click goes through even if a
  // backdrop overlay is still present from the previous test.
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

// After each test: stop playback and clean up any open overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start playback of album 2001 by invoking play_queue directly.
//
// This bypasses the MediaCard play button's resumePlayback() vs playQueue()
// branching logic. Invoking play_queue directly ensures we always start fresh
// from Track One regardless of any prior state.
// ----------------------------------------------------------------

async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    // Sort by track_number so Track One is first
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
  });

  // Wait until now-playing-title appears (UI received the TrackChanged event)
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Confirm Track One is loaded
  await p.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 }
  );

  // Wait for the React store to fully reflect the Playing state.
  await p.waitForFunction(
    () => {
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: navigate from the Albums list to the album detail page for album 2001.
//
// The MediaCard outer div (data-testid="media-card-album-2001") has no onClick.
// Navigation is triggered by either the artwork div (role="button") or the title <p>.
// We click the title text "Playwright Album" directly, which is the most reliable
// selector. The title <p> inside the card has cursor-pointer and calls handleClick().
// ----------------------------------------------------------------

async function navigateToAlbumDetail(p) {
  // The card must be visible first
  await p.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Click the title text inside the card — it is a <p> with onClick={handleClick}
  // We use locator scoping to target the title only within this specific card
  const card = p.locator('[data-testid="media-card-album-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();

  // Wait for the album detail page container to appear
  await p.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  // Also wait for the album title to be rendered
  await p.waitForSelector('[data-testid="album-title"]', { timeout: 10_000 });
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The now-playing-title container holds a TrackItem with nested spans;
// the title is in the first .text-sm element.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to become the expected value.
// ----------------------------------------------------------------

async function waitForTitle(p, expected, timeout = 15_000) {
  await p.waitForFunction(
    (exp) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === exp;
    },
    expected,
    { timeout }
  );
}

// ----------------------------------------------------------------
// Test 1: Album detail page loads correctly
// ----------------------------------------------------------------

test('album detail page loads with correct title, artist, and track count', async () => {
  await navigateToAlbumDetail(page);

  // Album title must match the seeded value
  const titleEl = page.locator('[data-testid="album-title"]');
  await expect(titleEl).toBeVisible();
  await expect(titleEl).toHaveText('Playwright Album');

  // Artist name must match — the artist link renders the name as text inside the paragraph
  const artistEl = page.locator('[data-testid="album-artist"]');
  await expect(artistEl).toBeVisible();
  await expect(artistEl).toContainText('Playwright Artist');

  // Track count paragraph must be visible and show 5 tracks
  const trackCountEl = page.locator('[data-testid="album-track-count"]');
  await expect(trackCountEl).toBeVisible();
  await expect(trackCountEl).toContainText('5');
});

// ----------------------------------------------------------------
// Test 2: Track list shows all 5 seeded tracks
// ----------------------------------------------------------------

test('track list shows all 5 tracks with correct titles', async () => {
  await navigateToAlbumDetail(page);

  // Wait for the track list container
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  const trackRows = page.locator('[data-testid="track-row"]');
  await expect(trackRows).toHaveCount(5, { timeout: 10_000 });

  // Verify each track title is visible somewhere on the page
  const expectedTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
  for (const title of expectedTitles) {
    // Each track row contains a span with the track title text
    const row = trackRows.filter({ hasText: title });
    await expect(row).toBeVisible({ timeout: 5_000 });
  }
});

// ----------------------------------------------------------------
// Test 3: Clicking "Play All" starts playback from the first track
// ----------------------------------------------------------------

test('clicking Play All starts playback from Track One', async () => {
  await navigateToAlbumDetail(page);

  // Wait for the Play All button
  const playAllBtn = page.locator('[data-testid="album-play-all-button"]');
  await expect(playAllBtn).toBeVisible({ timeout: 5_000 });
  await expect(playAllBtn).not.toBeDisabled();

  // Click Play All
  await playAllBtn.click();

  // Wait for the now-playing panel to appear with a track title
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // The first track in the album (sorted by track_number) should be Track One
  await waitForTitle(page, 'Track One');

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 4: Double-clicking a specific track row starts playback from that track
// ----------------------------------------------------------------

test('double-clicking Track Three row starts playback from Track Three', async () => {
  await navigateToAlbumDetail(page);

  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Find the row containing "Track Three" and double-click it to play
  const trackRows = page.locator('[data-testid="track-row"]');
  const trackThreeRow = trackRows.filter({ hasText: 'Track Three' });
  await trackThreeRow.waitFor({ state: 'visible', timeout: 10_000 });
  await trackThreeRow.dblclick();

  // Wait for the now-playing panel
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Now-playing title must be Track Three (or any track from the album, since
  // TrackList passes clickedIndex as startIndex — Track Three is at index 2)
  await waitForTitle(page, 'Track Three');

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 5: Next button advances track while album detail page is displayed
// ----------------------------------------------------------------

test('next button advances to Track Two while album detail page is open', async () => {
  // Start playback via direct IPC (bypasses MediaCard branching logic)
  await startPlayback(page);

  // Navigate to the album detail page while playback is running
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
  await navigateToAlbumDetail(page);

  // Verify we are still on Track One
  const initialTitle = await getNowPlayingTitle(page);
  expect(initialTitle).toBe('Track One');

  // Click the next button in the playback controls
  await page.click('[data-testid="next-button"]');

  // Wait for the title to change to Track Two
  await waitForTitle(page, 'Track Two');

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).toBe('Track Two');

  // State should still be Playing
  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 6: Album info matches seeded data
// ----------------------------------------------------------------

test('album detail page displays correct title and track count matching seed data', async () => {
  await navigateToAlbumDetail(page);

  // The album title element must exactly match the seeded title
  const titleEl = page.locator('[data-testid="album-title"]');
  await expect(titleEl).toHaveText('Playwright Album');

  // The track count element must include the number 5
  const trackCountEl = page.locator('[data-testid="album-track-count"]');
  await expect(trackCountEl).toContainText('5');

  // All 5 track rows must be present in the track list
  const trackRows = page.locator('[data-testid="track-row"]');
  await expect(trackRows).toHaveCount(5, { timeout: 10_000 });

  // The album detail page container itself must be present
  await expect(page.locator('[data-testid="album-detail-page"]')).toBeVisible();
});
