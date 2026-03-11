/**
 * Artist page — Playwright CDP E2E tests
 *
 * NOTE: This spec requires a rebuilt binary so that the data-testid attributes
 * added to ArtistPage.tsx are embedded in the distributed assets:
 *   - data-testid="artist-detail-page"   (outer container of ArtistPage)
 *   - data-testid="artist-name"          (h1 with artist name)
 *   - data-testid="artist-stats"         (p with album/track counts)
 *   - data-testid="artist-play-all-button" (Play All button)
 *   - data-testid="artist-album-card-{id}" (ArtistAlbumCard divs)
 *
 * Covers the Artists grid and Artist detail page:
 *
 *   1. Artists page loads and shows the seeded Playwright Artist card
 *   2. Clicking an artist card title navigates to the artist detail page
 *   3. Artist detail page displays the correct artist name
 *   4. Artist detail page shows album stats (at least 1 album, 5 tracks)
 *   5. Artist detail page shows seeded album in the discography section
 *   6. Play All button starts playback from the artist's tracks
 *   7. Clicking an album card on the artist detail page navigates to album detail
 *
 * Seed data (from playwright-global-setup.js):
 *   Artist ID 2001 — "Playwright Artist"
 *   Album  ID 2001 — "Playwright Album" — 5 tracks (Track One … Track Five, 2-second WAV)
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

// Before each test: stop any active playback, dismiss open overlays, navigate to Artists.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Artists list — use force:true so the click goes through even if a
  // backdrop overlay is still present from the previous test.
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="artists-page"]', { timeout: 15_000 });
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
// Helper: navigate from the Artists grid to the artist detail page for artist 2001.
//
// The ArtistCard is a MediaCard wrapper, so the outer div has
// data-testid="media-card-artist-2001". Navigation is triggered by clicking the
// title <p> inside the card (same pattern as album-playback.spec.js's
// navigateToAlbumDetail helper).
// ----------------------------------------------------------------

async function navigateToArtistDetail(p) {
  // The card must be visible first
  await p.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });

  // Click the title text inside the card — the <p> element with the artist name
  // is the click target that triggers navigation.
  const card = p.locator('[data-testid="media-card-artist-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Artist' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();

  // Wait for the artist detail page container to appear
  await p.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  // Also wait for the artist name heading to be rendered
  await p.waitForSelector('[data-testid="artist-name"]', { timeout: 10_000 });
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The now-playing-title container holds a TrackItem; the title is in the
// first .text-sm element (same pattern as other spec files).
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
// Test 1: Artists page loads and shows the Playwright Artist card
// ----------------------------------------------------------------

test('artists page loads and shows Playwright Artist card', async () => {
  // The artists-page container must be visible (applied by LibraryPageLayout via pageTestId)
  await expect(page.locator('[data-testid="artists-page"]')).toBeVisible();

  // The seeded artist card must be present with the correct title text
  await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });
  const card = page.locator('[data-testid="media-card-artist-2001"]');
  await expect(card).toBeVisible();
  await expect(card).toContainText('Playwright Artist');
});

// ----------------------------------------------------------------
// Test 2: Clicking the artist card title navigates to the artist detail page
// ----------------------------------------------------------------

test('clicking artist card title navigates to artist detail page', async () => {
  await navigateToArtistDetail(page);

  // The artist detail page outer container must be visible
  await expect(page.locator('[data-testid="artist-detail-page"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 3: Artist detail page displays the correct artist name
// ----------------------------------------------------------------

test('artist detail page shows correct artist name', async () => {
  await navigateToArtistDetail(page);

  const nameEl = page.locator('[data-testid="artist-name"]');
  await expect(nameEl).toBeVisible();
  await expect(nameEl).toHaveText('Playwright Artist');
});

// ----------------------------------------------------------------
// Test 4: Artist detail page shows album and track stats
//
// The seeded data has 1 album and 5 tracks for Playwright Artist.
// The artist-stats paragraph renders "{album_count} Albums • {track_count} Tracks".
// ----------------------------------------------------------------

test('artist detail page shows correct album and track counts in stats', async () => {
  await navigateToArtistDetail(page);

  const statsEl = page.locator('[data-testid="artist-stats"]');
  await expect(statsEl).toBeVisible();

  // Must contain the album count (1) and track count (6: Track One–Five + Collab Track)
  await expect(statsEl).toContainText('1');
  await expect(statsEl).toContainText('6');
});

// ----------------------------------------------------------------
// Test 5: Artist detail page shows the seeded album in the discography
//
// The ArtistAlbumCard for album 2001 has data-testid="artist-album-card-2001".
// ----------------------------------------------------------------

test('artist detail page discography contains Playwright Album', async () => {
  await navigateToArtistDetail(page);

  // Wait for the discography album card for album 2001
  const albumCard = page.locator('[data-testid="artist-album-card-2001"]');
  await albumCard.waitFor({ state: 'visible', timeout: 10_000 });
  await expect(albumCard).toBeVisible();
  await expect(albumCard).toContainText('Playwright Album');
});

// ----------------------------------------------------------------
// Test 6: Play All button starts playback from the artist's tracks
// ----------------------------------------------------------------

test('clicking Play All on artist detail page starts playback', async () => {
  await navigateToArtistDetail(page);

  // Wait for the Play All button to appear and be enabled
  const playAllBtn = page.locator('[data-testid="artist-play-all-button"]');
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

  // The first track should be Track One (lowest track_number in the artist's tracks)
  await waitForTitle(page, 'Track One');

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');

  // The now-playing title must be visible
  await expect(page.locator('[data-testid="now-playing-title"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 7: Clicking an album card on the artist detail page navigates to album detail
// ----------------------------------------------------------------

test('clicking Playwright Album card on artist detail navigates to album detail page', async () => {
  await navigateToArtistDetail(page);

  // Wait for the album card to appear in the discography
  const albumCard = page.locator('[data-testid="artist-album-card-2001"]');
  await albumCard.waitFor({ state: 'visible', timeout: 10_000 });

  // Click the album card to navigate to album detail
  await albumCard.click();

  // Wait for the album detail page to appear
  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="album-detail-page"]')).toBeVisible();

  // The album title must match the seeded value
  const albumTitleEl = page.locator('[data-testid="album-title"]');
  await expect(albumTitleEl).toBeVisible({ timeout: 5_000 });
  await expect(albumTitleEl).toHaveText('Playwright Album');
});
