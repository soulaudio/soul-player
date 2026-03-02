/**
 * Now Playing page — Playwright CDP tests
 *
 * Covers the /now-playing route which shows:
 *   - Album artwork (left side)
 *   - Context header (album name + subtitle)
 *   - Full track list from the current playback context (right side)
 *   - Clicking a track in the list starts playback from that position
 *
 * Playback controls (play-pause, next, previous) and the progress bar live in the
 * sidebar PlayerPanel, which is always visible alongside the now-playing page.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist" — 5 tracks × 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
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

// Before each test: stop playback, dismiss overlays, navigate to Albums, start fresh playback,
// then navigate to the /now-playing page.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums to reset UI state
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Start playback of album 2001 from Track One
  await startPlayback(page);

  // Navigate to the /now-playing page by clicking the now-playing-title in the sidebar
  await page.click('[data-testid="now-playing-title"]', { force: true });
  await page.waitForSelector('[data-testid="now-playing-page"]', { timeout: 10_000 });
});

// After each test: stop playback, navigate away from now-playing, clean up overlays.
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
// Copied exactly from playback-controls.spec.js — bypasses the MediaCard
// branching logic so we always start fresh from Track One.
// ----------------------------------------------------------------

async function startPlayback(p) {
  // Fetch tracks for album 2001 and start from Track One via play_queue.
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
    // Record album context so NowPlayingPage shows "Playwright Album" as the header
    // title, not whatever context was left by a previous spec file (e.g. genre-page).
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2001',
        contextName: 'Playwright Album',
        contextArtworkPath: null,
      },
    });
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
  // Small flat wait for the React event handler to finish updating isPlaying.
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: read the current track title from the sidebar NowPlayingPanel.
// ----------------------------------------------------------------

async function getSidebarNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the sidebar now-playing title to become the expected value.
// ----------------------------------------------------------------

async function waitForSidebarTitle(p, expected, timeout = 15_000) {
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
// Test 1: Now playing page shows current track info
// ----------------------------------------------------------------

test('now playing page shows current track info with album and artist', async () => {
  // The now-playing page context header shows album name and context subtitle
  const trackTitle = page.locator('[data-testid="now-playing-track-title"]');
  await expect(trackTitle).toBeVisible({ timeout: 10_000 });

  // Album name must appear as the context header title
  const titleText = await trackTitle.textContent();
  expect(titleText.trim()).toBe('Playwright Album');

  // The current track (Track One) must appear highlighted in the track list
  const queueList = page.locator('[data-testid="now-playing-queue-list"]');
  await expect(queueList).toBeVisible();

  // Track One should be the first item (index 0) and be highlighted
  const firstItem = page.locator('[data-testid="now-playing-queue-item-0"]');
  await expect(firstItem).toBeVisible();
  const firstItemText = await firstItem.textContent();
  expect(firstItemText).toContain('Track One');

  // First item should show the artist name
  expect(firstItemText).toContain('Playwright Artist');
});

// ----------------------------------------------------------------
// Test 2: Now playing page shows album artwork area
// ----------------------------------------------------------------

test('now playing page shows artwork container', async () => {
  const artwork = page.locator('[data-testid="now-playing-artwork"]');
  await expect(artwork).toBeVisible({ timeout: 10_000 });

  // The artwork container must have positive dimensions
  const box = await artwork.boundingBox();
  expect(box).not.toBeNull();
  expect(box.width).toBeGreaterThan(50);
  expect(box.height).toBeGreaterThan(50);
});

// ----------------------------------------------------------------
// Test 3: Now playing page shows playback controls (in sidebar)
// ----------------------------------------------------------------

test('now playing page: playback controls are visible in sidebar', async () => {
  // Controls are rendered in the sidebar PlayerPanel which stays visible
  // alongside all pages including /now-playing.
  const playPauseBtn = page.locator('[data-testid="play-pause-button"]');
  await expect(playPauseBtn).toBeVisible({ timeout: 5_000 });
  await expect(playPauseBtn).not.toBeDisabled();

  const nextBtn = page.locator('[data-testid="next-button"]');
  await expect(nextBtn).toBeVisible();

  const prevBtn = page.locator('[data-testid="previous-button"]');
  await expect(prevBtn).toBeVisible();
});

// ----------------------------------------------------------------
// Test 4: Skip to next track from now playing page
// ----------------------------------------------------------------

test('clicking next button from now playing page advances to Track Two', async () => {
  // Confirm we start on Track One
  const initialTitle = await getSidebarNowPlayingTitle(page);
  expect(initialTitle).toBe('Track One');

  // Click next
  await page.click('[data-testid="next-button"]');

  // Wait for sidebar title to change to Track Two
  await waitForSidebarTitle(page, 'Track Two');

  const newTitle = await getSidebarNowPlayingTitle(page);
  expect(newTitle).toBe('Track Two');

  // The now-playing page should still be visible after the track change
  await expect(page.locator('[data-testid="now-playing-page"]')).toBeVisible();

  // Track Two should now be highlighted in the list (index 1)
  await page.waitForFunction(
    () => {
      const item = document.querySelector('[data-testid="now-playing-queue-item-1"]');
      if (!item) return false;
      // The current track item has bg-primary/10 styling applied
      return item.className.includes('bg-primary') || item.className.includes('border-primary');
    },
    { timeout: 10_000 }
  );
});

// ----------------------------------------------------------------
// Test 5: Now playing page shows all tracks in the queue list
// ----------------------------------------------------------------

test('now playing page queue list shows all album tracks', async () => {
  const queueList = page.locator('[data-testid="now-playing-queue-list"]');
  await expect(queueList).toBeVisible({ timeout: 10_000 });

  // Album has 5 tracks — all should appear in the list
  const items = page.locator('[data-testid^="now-playing-queue-item-"]');
  const count = await items.count();
  expect(count).toBe(5);

  // Verify track names appear in the list
  const listText = await queueList.textContent();
  expect(listText).toContain('Track One');
  expect(listText).toContain('Track Two');
  expect(listText).toContain('Track Three');
  expect(listText).toContain('Track Four');
  expect(listText).toContain('Track Five');
});

// ----------------------------------------------------------------
// Test 6: Clicking a queue item skips to that track
// ----------------------------------------------------------------

test('clicking Track Four in the queue list skips to Track Four', async () => {
  // Confirm we start on Track One
  expect(await getSidebarNowPlayingTitle(page)).toBe('Track One');

  // Click the 4th item (index 3 = "Track Four")
  const trackFourItem = page.locator('[data-testid="now-playing-queue-item-3"]');
  await expect(trackFourItem).toBeVisible({ timeout: 5_000 });

  // Verify it contains "Track Four" before clicking
  const itemText = await trackFourItem.textContent();
  expect(itemText).toContain('Track Four');

  await trackFourItem.click();

  // Wait for sidebar title to change to Track Four
  await waitForSidebarTitle(page, 'Track Four');

  const newTitle = await getSidebarNowPlayingTitle(page);
  expect(newTitle).toBe('Track Four');

  // Verify playback state is still Playing after the skip
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 7: Play-pause toggle from now playing page
// ----------------------------------------------------------------

test('play-pause button toggles between Playing and Paused states', async () => {
  // Start state: Playing
  const state1 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state1).toBe('Playing');

  // Click play-pause to pause
  await page.click('[data-testid="play-pause-button"]');

  // Flat wait to avoid IPC contention (see playback-controls.spec.js comment)
  await page.waitForTimeout(1_500);

  const state2 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state2).toBe('Paused');

  // Now-playing page should still be visible after pausing
  await expect(page.locator('[data-testid="now-playing-page"]')).toBeVisible();

  // Click play-pause again to resume
  await page.click('[data-testid="play-pause-button"]');

  // Poll for Playing state rather than using a flat wait:
  // Track One may have had very little time remaining when paused, so it can
  // finish immediately on resume and auto-advance to Track Two. During that
  // transition the state briefly shows as non-Playing while the new track loads.
  // Polling with a generous timeout catches it once Track Two starts playing.
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 8_000 }
  );

  const state3 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state3).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 8: Now playing page shows progress bar (in sidebar)
// ----------------------------------------------------------------

test('now playing page: progress/seek bar is visible in sidebar', async () => {
  // The ProgressBar is rendered in the sidebar PlayerPanel alongside all pages.
  // It shows current time, the scrubable track, and total duration.
  const progressBar = page.locator('[data-testid="now-playing-progress-bar"]');
  await expect(progressBar).toBeVisible({ timeout: 10_000 });

  // The progress bar must have positive dimensions
  const box = await progressBar.boundingBox();
  expect(box).not.toBeNull();
  expect(box.width).toBeGreaterThan(50);

  // It should display time information (non-empty text)
  const text = await progressBar.textContent();
  expect(text.trim().length).toBeGreaterThan(0);
});
