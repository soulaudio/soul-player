/**
 * Multi-artist support — Playwright CDP E2E tests
 *
 * Verifies that tracks with multiple artists:
 *   - Display all artist names as comma-separated clickable links in the tracklist
 *   - Navigate to the correct artist detail page when an artist link is clicked
 *   - Show featured tracks on the featured artist's page (via track_artists junction)
 *
 * Seed data (from playwright-global-setup.js):
 *   Artist 2001 — "Playwright Artist"  — primary on tracks 2001-2005 + Collab Track
 *   Artist 2003 — "Featured Artist"    — featured only on Collab Track (track 2006)
 *   Track  2006 — "Collab Track"       — album 2001, both artists in track_artists junction
 *
 * Tests:
 *   1. Single-artist track shows one artist name (backward-compat fallback)
 *   2. Multi-artist track shows both artist names
 *   3. Artists are separated by a comma in the rendered row
 *   4. Clicking the primary artist link navigates to their detail page
 *   5. Clicking the featured artist link navigates to their detail page
 *   6. Featured artist page shows the Collab Track (via junction query)
 *   7. Primary artist page shows all 6 tracks (own 5 + the collab track)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- Shared CDP connection ----

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page
    .evaluate(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke('stop_playback');
      } catch {}
    })
    .catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Start each test on the Tracks page (shows all tracks, including Collab Track)
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page
    .evaluate(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke('stop_playback');
      } catch {}
    })
    .catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

/** Return the track-row locator for the given track title. */
function trackRow(title) {
  return page.locator('[data-testid="track-row"]').filter({ hasText: title });
}

/** Return the artist link (role=button) within a track row matching the given artist name. */
function artistLinkIn(row, artistName) {
  return row.locator('[role="button"]').filter({ hasText: artistName });
}

// ----------------------------------------------------------------
// Test 1: Single-artist track shows exactly one artist (backward-compat path)
// ----------------------------------------------------------------

test('single-artist track shows the primary artist name', async () => {
  const row = trackRow('Track One').first();
  await row.waitFor({ state: 'visible', timeout: 10_000 });

  // "Playwright Artist" must appear as a clickable role=button element
  const link = artistLinkIn(row, 'Playwright Artist');
  await expect(link).toBeVisible();

  // "Featured Artist" must NOT appear on a single-artist track row
  await expect(row.getByText('Featured Artist', { exact: true })).not.toBeVisible();
});

// ----------------------------------------------------------------
// Test 2: Multi-artist track shows both artist names
// ----------------------------------------------------------------

test('multi-artist track shows both artist names as clickable links', async () => {
  const row = trackRow('Collab Track').first();
  await row.waitFor({ state: 'visible', timeout: 10_000 });

  // Both artist links must be present and visible
  await expect(artistLinkIn(row, 'Playwright Artist')).toBeVisible();
  await expect(artistLinkIn(row, 'Featured Artist')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 3: Multiple artists are separated by a comma
// ----------------------------------------------------------------

test('multiple artists are rendered with a comma separator', async () => {
  const row = trackRow('Collab Track').first();
  await row.waitFor({ state: 'visible', timeout: 10_000 });

  const text = await row.textContent();

  // Both names must appear with a comma somewhere between them
  const primaryIdx = text.indexOf('Playwright Artist');
  const featuredIdx = text.indexOf('Featured Artist');

  expect(primaryIdx).toBeGreaterThanOrEqual(0);
  expect(featuredIdx).toBeGreaterThan(primaryIdx);

  // There must be a comma character between the two names
  const between = text.slice(primaryIdx + 'Playwright Artist'.length, featuredIdx);
  expect(between).toContain(',');
});

// ----------------------------------------------------------------
// Test 4: Clicking the primary artist link navigates to their detail page
// ----------------------------------------------------------------

test('clicking primary artist link on multi-artist track navigates to their page', async () => {
  const row = trackRow('Collab Track').first();
  await row.waitFor({ state: 'visible', timeout: 10_000 });

  await artistLinkIn(row, 'Playwright Artist').click();

  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="artist-name"]')).toHaveText('Playwright Artist');
});

// ----------------------------------------------------------------
// Test 5: Clicking the featured artist link navigates to their detail page
// ----------------------------------------------------------------

test('clicking featured artist link on multi-artist track navigates to their page', async () => {
  const row = trackRow('Collab Track').first();
  await row.waitFor({ state: 'visible', timeout: 10_000 });

  await artistLinkIn(row, 'Featured Artist').click();

  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="artist-name"]')).toHaveText('Featured Artist');
});

// ----------------------------------------------------------------
// Test 6: Featured artist page shows the Collab Track
//
// "Featured Artist" appears in track_artists only for track 2006.
// get_by_artist now queries via the junction table, so the page must
// list "Collab Track" even though it is not the primary artist on the track.
// ----------------------------------------------------------------

test('featured artist page shows the collab track they appear on', async () => {
  // Navigate via artists list so we exercise the standard navigation path
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="artists-page"]', { timeout: 15_000 });

  // Click Featured Artist card
  const card = page.locator('[data-testid="media-card-artist-2003"]');
  await card.waitFor({ state: 'visible', timeout: 15_000 });
  await card.locator('p').filter({ hasText: 'Featured Artist' }).first().click();

  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="artist-name"]')).toHaveText('Featured Artist');

  // The Collab Track must appear in the artist's tracklist
  const collabRow = page.locator('[data-testid="track-row"]').filter({ hasText: 'Collab Track' });
  await collabRow.waitFor({ state: 'visible', timeout: 10_000 });
  await expect(collabRow).toBeVisible();
});

// ----------------------------------------------------------------
// Test 7: Primary artist page includes the collab track as well as their own tracks
//
// Playwright Artist is primary on tracks 2001-2005 and also on track 2006 (Collab Track).
// The artist page must list all 6 tracks.
// ----------------------------------------------------------------

test('primary artist page shows own tracks plus the collab track', async () => {
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="artists-page"]', { timeout: 15_000 });

  const card = page.locator('[data-testid="media-card-artist-2001"]');
  await card.waitFor({ state: 'visible', timeout: 15_000 });
  await card.locator('p').filter({ hasText: 'Playwright Artist' }).first().click();

  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });

  // 6 tracks total: Track One–Five (5) + Collab Track (1)
  await expect(page.locator('[data-testid="track-row"]')).toHaveCount(6, { timeout: 10_000 });

  // Collab Track must be one of them
  await expect(
    page.locator('[data-testid="track-row"]').filter({ hasText: 'Collab Track' })
  ).toBeVisible();
});
