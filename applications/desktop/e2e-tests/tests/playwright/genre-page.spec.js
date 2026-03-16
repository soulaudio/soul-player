/**
 * Genre detail page — Playwright CDP E2E tests
 *
 * Covers the genre detail page layout, track listing, and playback flows:
 *
 *   1. Genre detail page loads with correct title
 *   2. Genre track list shows all 6 seeded tracks
 *   3. Play All button starts playback from the first track
 *   4. Double-clicking a track row starts playback from that track
 *   5. Back button navigates away from the genre page
 *
 * Navigation note:
 *   There is no genre entry in the NavBar (only Home/Albums/Artists/Playlists/Tracks).
 *   The genre detail page at /genres/:id is normally reached by clicking a genre badge
 *   elsewhere in the UI. Since no such surface exists in the current UI, we navigate
 *   programmatically via the React Router history API injected through page.evaluate().
 *   This is reliable because the app uses React Router v6 with a browser history, and
 *   window.__reactRouterNavigate (injected by the helper below) dispatches a proper
 *   navigation event that React Router picks up.
 *
 * Seed data (from playwright-global-setup.js):
 *   Genre ID 4001 — "Playwright Genre"
 *   Track IDs 2001–2005 + 2006 (Collab Track), titles: Track One … Track Five + Collab Track
 *   All 6 tracks are linked to genre 4001 via track_genres junction table.
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

  // Navigate to Albums list as the stable starting point.
  // Use force:true so the click goes through even if a backdrop overlay is still present.
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
// Helper: navigate to the genre detail page for genre 4001.
//
// The NavBar has no genre link (genres: Home/Albums/Artists/Playlists/Tracks only).
// We use the React Router v6 navigation API exposed on the window object.
// React Router v6 stores its navigate function internally, but we can trigger
// navigation by dispatching a popstate event with the desired path, which is
// equivalent to calling navigate('/genres/4001') in userland.
//
// Method: dispatch a custom navigation event via window.history.pushState +
// a popstate event. React Router v6 (with createBrowserRouter / BrowserRouter)
// listens for popstate events and updates its internal state accordingly.
// ----------------------------------------------------------------

async function navigateToGenrePage(p) {
  await p.evaluate(() => {
    window.history.pushState({}, '', '/genres/4001');
    window.dispatchEvent(new PopStateEvent('popstate', { state: {} }));
  });

  // Wait for the genre detail page container to be rendered
  await p.waitForSelector('[data-testid="genre-detail-page"]', { timeout: 15_000 });
  // Also wait for the genre title heading to confirm data has loaded
  await p.waitForSelector('[data-testid="genre-title"]', { timeout: 10_000 });
}

// ----------------------------------------------------------------
// Helper: start playback of genre 4001 tracks by invoking play_queue directly.
//
// This bypasses the GenrePage's handlePlayAll() branching logic and directly
// puts all 6 seeded tracks into the queue, starting from Track One.
// ----------------------------------------------------------------

async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_genre_tracks', { genreId: 4001 });
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
// Test 1: Genre detail page loads with correct title
// ----------------------------------------------------------------

test('genre detail page loads with correct title', async () => {
  await navigateToGenrePage(page);

  // Page container must be visible
  const pageEl = page.locator('[data-testid="genre-detail-page"]');
  await expect(pageEl).toBeVisible({ timeout: 15_000 });

  // Genre title must match the seeded value
  const titleEl = page.locator('[data-testid="genre-title"]');
  await expect(titleEl).toBeVisible({ timeout: 10_000 });
  await expect(titleEl).toHaveText('Playwright Genre', { timeout: 10_000 });
});

// ----------------------------------------------------------------
// Test 2: Genre track list shows all 5 seeded tracks
// ----------------------------------------------------------------

test('genre track list shows all 6 seeded tracks', async () => {
  await navigateToGenrePage(page);

  // Wait for the track list container (rendered by the shared TrackList component)
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  const trackRows = page.locator('[data-testid="track-row"]');
  await expect(trackRows).toHaveCount(6, { timeout: 10_000 });

  // Verify each expected track title is present in a row
  const expectedTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five', 'Collab Track'];
  for (const title of expectedTitles) {
    const row = trackRows.filter({ hasText: title });
    await expect(row).toBeVisible({ timeout: 5_000 });
  }
});

// ----------------------------------------------------------------
// Test 3: Track count badge shows 5 tracks
// ----------------------------------------------------------------

test('genre track count shows 6 tracks', async () => {
  await navigateToGenrePage(page);

  // The genre-track-count element shows "{count} tracks • {duration}"
  const countEl = page.locator('[data-testid="genre-track-count"]');
  await expect(countEl).toBeVisible({ timeout: 5_000 });
  await expect(countEl).toContainText('6');
});

// ----------------------------------------------------------------
// Test 4: Play All button starts playback
// ----------------------------------------------------------------

test('clicking Play All starts playback from Track One', async () => {
  await navigateToGenrePage(page);

  // Wait for and click the Play All button
  const playAllBtn = page.locator('[data-testid="genre-play-all-button"]');
  await expect(playAllBtn).toBeVisible({ timeout: 5_000 });
  await expect(playAllBtn).not.toBeDisabled();
  await playAllBtn.click();

  // Wait for the now-playing panel to appear
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // The first track (sorted by track_number) should be Track One
  await waitForTitle(page, 'Track One');

  // Poll until state is Playing — manager.rs emits StateChanged(Stopped) briefly between
  // tracks during auto-advance (play_next_in_queue). If T1 just ended when we check,
  // the one-shot IPC would see 'Stopped'; polling retries until T2 starts, confirming
  // that the Play All click did trigger active playback.
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Now-playing title must be visible
  await expect(page.locator('[data-testid="now-playing-title"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 5: Double-clicking a track row starts playback from that track
// ----------------------------------------------------------------

test('double-clicking Track Three row starts playback from Track Three', async () => {
  await navigateToGenrePage(page);

  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Find the row containing "Track Three" and double-click it
  const trackRows = page.locator('[data-testid="track-row"]');
  const trackThreeRow = trackRows.filter({ hasText: 'Track Three' });
  await trackThreeRow.waitFor({ state: 'visible', timeout: 10_000 });
  await trackThreeRow.dblclick();

  // Wait for the now-playing panel to appear
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Now-playing title should be Track Three (TrackList passes clickedIndex as startIndex)
  await waitForTitle(page, 'Track Three');

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 6: Back button navigates away from the genre page
// ----------------------------------------------------------------

// ----------------------------------------------------------------
// Test 7: Artist name in track row is clickable and navigates to artist page
//
// This test will FAIL before the fix because GenrePage's TrackList mapping
// omits artistId, causing ArtistLink to render as a plain non-clickable span.
// After fix: artistId is included → ArtistLink renders [role="button"] → clickable.
// ----------------------------------------------------------------

test('back button navigates away from genre detail page', async () => {
  await navigateToGenrePage(page);

  // The back button navigates to /library?tab=genres (no library route exists,
  // so we just verify we are no longer on the genre detail page).
  const backBtn = page.locator('[data-testid="genre-back-button"]');
  await expect(backBtn).toBeVisible({ timeout: 5_000 });
  await backBtn.click();

  // Wait for the genre detail page to disappear
  await page.waitForFunction(
    () => !document.querySelector('[data-testid="genre-detail-page"]'),
    { timeout: 10_000 }
  );

  // Verify we navigated away — genre-title should no longer be visible
  await expect(page.locator('[data-testid="genre-detail-page"]')).not.toBeVisible();
});

// ----------------------------------------------------------------
// Test 7: Artist name in track row is clickable and navigates to artist page
//
// FAILS before fix: GenrePage's TrackList mapping omits artistId, causing
// ArtistLink to render as a plain <span> without role="button".
// PASSES after fix: artistId is included → ArtistLink is clickable.
// ----------------------------------------------------------------

test('artist name in genre track row is clickable and navigates to artist page', async () => {
  await navigateToGenrePage(page);
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Get the first track row
  const firstRow = page.locator('[data-testid="track-row"]').first();
  await firstRow.waitFor({ state: 'visible', timeout: 10_000 });

  // The artist cell should contain a clickable span (role="button") — not plain text.
  // ArtistLink renders role="button" only when artistId is provided.
  const artistLink = firstRow.locator('[role="button"]').filter({ hasText: 'Playwright Artist' });
  await expect(artistLink).toBeVisible({ timeout: 5_000 });

  // Click the artist link
  await artistLink.click();

  // Should navigate to the artist detail page
  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="artist-detail-page"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 8: Album name in track row is clickable and navigates to album page
//
// FAILS before fix: GenrePage's TrackList mapping omits albumId, causing
// AlbumLink to render as a plain <span> without role="button".
// PASSES after fix: albumId is included → AlbumLink is clickable.
// ----------------------------------------------------------------

test('album name in genre track row is clickable and navigates to album page', async () => {
  await navigateToGenrePage(page);
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  const firstRow = page.locator('[data-testid="track-row"]').first();
  await firstRow.waitFor({ state: 'visible', timeout: 10_000 });

  // The album cell should contain a clickable span (role="button") — not plain text.
  // AlbumLink renders role="button" only when albumId is provided.
  const albumLink = firstRow.locator('[role="button"]').filter({ hasText: 'Playwright Album' });
  await expect(albumLink).toBeVisible({ timeout: 5_000 });

  // Click the album link — AlbumLink calls stopPropagation so it won't double-click the row
  await albumLink.click();

  // Should navigate to the album detail page
  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="album-detail-page"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 9: Double-clicking a track in genre page records genre playback context
//
// FAILS before fix: TrackList.handlePlay calls commands.playQueue() without
// calling backend.recordContext(), so the genre context is never stored.
// PASSES after fix: onBeforePlay callback records genre context before playback.
// ----------------------------------------------------------------

test('double-clicking track in genre page records genre playback context', async () => {
  await navigateToGenrePage(page);
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Double-click Track One to start playback
  const firstRow = page.locator('[data-testid="track-row"]').filter({ hasText: 'Track One' });
  await firstRow.waitFor({ state: 'visible', timeout: 10_000 });
  await firstRow.dblclick();

  // Wait for playback to start
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Verify that the most recent playback context is for genre 4001
  const contexts = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts', { limit: 1 });
  });

  expect(contexts).toHaveLength(1);
  expect(contexts[0].contextType).toBe('genre');
  expect(contexts[0].contextId).toBe('4001');
});
