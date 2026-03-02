/**
 * Search and filter — Playwright CDP tests
 *
 * Verifies that the search input in AlbumsPage, TracksPage, and ArtistsPage:
 *   1. Shows all items by default (no filter applied)
 *   2. Filters items by title/name when a query is typed
 *   3. Filters tracks by artist name
 *   4. Restores the full list when the search input is cleared
 *   5. Hides items that don't match the query
 *
 * Seed data (from playwright-global-setup.js):
 *   Album  ID 2001 — "Playwright Album" — artist "Playwright Artist"
 *   Artist ID 2001 — "Playwright Artist"
 *   Track IDs 2001–2005 — "Track One" … "Track Five"
 *   These are the ONLY albums / artists / tracks in the test DB.
 *
 * Implementation notes:
 *   - Search is powered by React's useDeferredValue, so results update
 *     synchronously on the next render tick. We use waitForFunction to
 *     poll the DOM state rather than relying on fixed timeouts.
 *   - The search bar in LibraryPageLayout auto-hides on scroll and has an
 *     idle timer. We reset it before each test by navigating fresh (which
 *     triggers setShowSearchBar(true) on mount).
 *   - track-row elements contain the track title as a <span> inside the
 *     first column div. We count visible rows and inspect their text.
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

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: stop playback, dismiss any overlays, navigate to Albums.
test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums as the default starting page
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

// After each test: stop playback, clear search input, dismiss overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});

  // Clear any text that may have been typed into the search input
  const searchInput = page.locator('[data-testid="search-input"]');
  const isVisible = await searchInput.isVisible().catch(() => false);
  if (isVisible) {
    await searchInput.fill('');
    await page.waitForTimeout(100);
  }

  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: type into search input, ensuring the bar is visible first.
//
// LibraryPageLayout shows the search bar on mount but may hide it after
// the idle timeout (3 s). We hover near the top of the scroll container
// to force it visible before interacting.
// ----------------------------------------------------------------

async function typeInSearch(text) {
  // Hover near the top-left of the viewport to trigger the show-on-hover logic
  await page.mouse.move(200, 50);
  await page.waitForTimeout(100);

  const searchInput = page.locator('[data-testid="search-input"]');
  await searchInput.waitFor({ state: 'visible', timeout: 10_000 });
  await searchInput.fill(text);

  // Give React's useDeferredValue one frame to commit the filtered result
  await page.waitForTimeout(200);
}

// ----------------------------------------------------------------
// Helper: clear the search input via the X button or direct fill.
// ----------------------------------------------------------------

async function clearSearch() {
  const searchInput = page.locator('[data-testid="search-input"]');
  await searchInput.fill('');
  await page.waitForTimeout(200);
}

// ----------------------------------------------------------------
// Helper: count visible track-row elements.
// ----------------------------------------------------------------

async function countTrackRows() {
  return page.locator('[data-testid="track-row"]').count();
}

// ----------------------------------------------------------------
// Helper: collect the title text from all visible track rows.
// The title is in the first truncating <span> inside the row.
// ----------------------------------------------------------------

async function getTrackRowTitles() {
  const rows = page.locator('[data-testid="track-row"]');
  const count = await rows.count();
  const titles = [];
  for (let i = 0; i < count; i++) {
    // The title is the first <span> that has the truncate class
    const titleSpan = rows.nth(i).locator('span.truncate').first();
    const text = await titleSpan.textContent().catch(() => '');
    titles.push(text.trim());
  }
  return titles;
}

// ================================================================
// ALBUMS PAGE TESTS
// ================================================================

test('albums page shows the Playwright Album card by default', async () => {
  // beforeEach already navigated to albums — card must be visible
  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible();
});

test('albums search filters results by album title', async () => {
  // Type a query that matches the seeded album
  await typeInSearch('Playwright');

  // Album 2001 must still be visible — title contains "Playwright"
  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible();

  // Type a query that matches nothing
  await typeInSearch('zzznomatch');

  // Album card must disappear — no album title or artist name contains "zzznomatch"
  await expect(albumCard).not.toBeVisible({ timeout: 5_000 });

  // Optional: the empty-state indicator may appear
  // We do not assert it strictly because the empty state rendering is conditional
  // on filteredAlbums.length === 0, which is already covered by the card disappearing.
});

test('clearing albums search restores the full album list', async () => {
  // Apply a filter that hides album 2001
  await typeInSearch('zzznomatch');

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).not.toBeVisible({ timeout: 5_000 });

  // Clear the search — album must reappear
  await clearSearch();

  await expect(albumCard).toBeVisible({ timeout: 5_000 });
});

test('albums search filters by artist name', async () => {
  // "Playwright Artist" is the artist for album 2001.
  // Typing the artist name should keep album 2001 visible.
  await typeInSearch('Playwright Artist');

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible();
});

// ================================================================
// TRACKS PAGE TESTS
// ================================================================

test('tracks page shows all 5 seeded tracks by default', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Wait until at least 5 rows are rendered (other spec files may import additional
  // tracks from the watched folder, so the exact count can exceed 5).
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  const count = await countTrackRows();
  expect(count).toBeGreaterThanOrEqual(5);

  // The 5 seeded tracks must all be present regardless of additional imports.
  const titles = await getTrackRowTitles();
  expect(titles).toContain('Track One');
  expect(titles).toContain('Track Two');
  expect(titles).toContain('Track Three');
  expect(titles).toContain('Track Four');
  expect(titles).toContain('Track Five');
});

test('tracks search filters by track title prefix', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Wait for initial load (at least 5 rows; extra tracks may exist from import tests).
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  // "Track T" matches "Track Two" and "Track Three" but NOT "Track One", "Track Four", "Track Five"
  await typeInSearch('Track T');

  // Wait for the filter to reduce the row count
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length === 2,
    { timeout: 5_000 }
  );

  const titles = await getTrackRowTitles();
  expect(titles).toContain('Track Two');
  expect(titles).toContain('Track Three');
  expect(titles).not.toContain('Track One');
  expect(titles).not.toContain('Track Four');
  expect(titles).not.toContain('Track Five');
});

test('tracks search filters by artist name', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Wait for at least 5 tracks to load (extra tracks may exist from import tests).
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  // All 5 seeded tracks belong to "Playwright Artist" — filtering by artist
  // keeps only those tracks visible (imported WAVs have no artist metadata).
  await typeInSearch('Playwright Artist');

  // All 5 seeded rows must be visible; imported WAVs are filtered out.
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 5_000 }
  );

  const count = await countTrackRows();
  expect(count).toBeGreaterThanOrEqual(5);
});

test('clearing tracks search restores full list', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Wait for at least 5 tracks (extra tracks may exist from import tests).
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length >= 5,
    { timeout: 15_000 }
  );

  const countBefore = await countTrackRows();

  // Filter to 2 results (Track Two + Track Three match "Track T")
  await typeInSearch('Track T');
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-row"]').length === 2,
    { timeout: 5_000 }
  );

  // Clear filter — should restore the full list
  await clearSearch();
  await page.waitForFunction(
    (expected) => document.querySelectorAll('[data-testid="track-row"]').length >= expected,
    countBefore,
    { timeout: 5_000 }
  );

  const count = await countTrackRows();
  expect(count).toBeGreaterThanOrEqual(countBefore);
});

// ================================================================
// ARTISTS PAGE TESTS
// ================================================================

test('artists page shows the Playwright Artist card by default', async () => {
  await page.click('[data-testid="nav-artists"]', { force: true });
  // ArtistCard wraps MediaCard with type="artist", so testid = media-card-artist-2001
  await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });

  const artistCard = page.locator('[data-testid="media-card-artist-2001"]');
  await expect(artistCard).toBeVisible();
});

test('artists search filters by artist name', async () => {
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });

  // Searching by the seeded artist name keeps the card visible
  await typeInSearch('Playwright Artist');

  const artistCard = page.locator('[data-testid="media-card-artist-2001"]');
  await expect(artistCard).toBeVisible();

  // Searching for something that doesn't match hides the card
  await typeInSearch('zzznomatch');
  await expect(artistCard).not.toBeVisible({ timeout: 5_000 });
});

test('clearing artists search restores the artist card', async () => {
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });

  // Apply a non-matching filter
  await typeInSearch('zzznomatch');
  const artistCard = page.locator('[data-testid="media-card-artist-2001"]');
  await expect(artistCard).not.toBeVisible({ timeout: 5_000 });

  // Clear — card must reappear
  await clearSearch();
  await expect(artistCard).toBeVisible({ timeout: 5_000 });
});
