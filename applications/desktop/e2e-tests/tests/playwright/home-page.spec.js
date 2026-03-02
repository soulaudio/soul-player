/**
 * Home page (/) — Playwright CDP E2E tests
 *
 * Covers the Home page bento-grid layout and navigation:
 *
 *   1. Home page loads and the main container is visible
 *   2. Home page shows at least one bento section with a heading
 *   3. The seeded album appears somewhere on the home page
 *   4. Navigation links (nav-albums, nav-playlists, etc.) are visible from home
 *   5. Clicking nav-albums from home navigates to the Albums list
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — Artist "Playwright Artist"
 *   Track IDs 2001–2005, titles: Track One … Track Five
 *   Playlist ID 3001 — "Favorites" (empty)
 *
 * Home page layout notes:
 *   - With no playback history seeded, recentAlbums is empty.
 *   - jumpBackAlbums falls back to a random shuffle of all albums (album 2001).
 *   - forgottenAlbums shows albums not recently played (also album 2001).
 *   - "Do some crate digging" (bottom) also draws from all albums.
 *   - So album 2001 must appear in at least one section.
 *
 * New data-testid attributes added to source files:
 *   HomePage.tsx:
 *     - data-testid="home-page"          — outer wrapper (loaded state)
 *     - data-testid="home-page-loading"  — loading skeleton wrapper
 *     - data-testid="home-section-{id}"  — each bento section div
 *     - data-testid="home-section-header-{id}" — each section heading
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

  // Capture browser console messages for debugging
  page.on('console', msg => {
    if (msg.text().includes('[HomePage]')) {
      console.log(`[BROWSER CONSOLE ${msg.type()}] ${msg.text()}`);
    }
  });

  // Short safety wait in case there is any residual animation or settling
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: stop any active playback, dismiss open overlays, navigate to Home.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Home — use force:true so the click goes through even if a
  // backdrop overlay is still present from the previous test.
  await page.click('[data-testid="nav-home"]', { force: true });

  // Wait for the home page to finish loading (loading skeleton disappears,
  // home-page container appears). Give the deferred data load 100ms to fire.
  await page.waitForSelector('[data-testid="home-page"]', { timeout: 15_000 });
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
// Test 1: Home page loads and is visible
// ----------------------------------------------------------------

test('home page loads and main container is visible', async () => {
  const homePage = page.locator('[data-testid="home-page"]');
  await expect(homePage).toBeVisible({ timeout: 10_000 });

  // The grid container must be rendered (grid-container class inside home-page)
  const gridContainer = homePage.locator('.grid-container');
  await expect(gridContainer).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 2: Home page shows at least one bento section with a heading
// ----------------------------------------------------------------

test('home page renders at least one bento section with a section heading', async () => {
  // The bento grid populates after a 100ms defer + data load.
  // Wait for any section heading to appear.
  await page.waitForSelector('[data-testid^="home-section-header-"]', { timeout: 15_000 });

  // At least one section must exist
  const sections = page.locator('[data-testid^="home-section-"]');
  const sectionCount = await sections.count();
  expect(sectionCount).toBeGreaterThanOrEqual(1);

  // At least one section heading must be non-empty text
  const headers = page.locator('[data-testid^="home-section-header-"]');
  const headerCount = await headers.count();
  expect(headerCount).toBeGreaterThanOrEqual(1);

  const firstHeaderText = (await headers.first().textContent()).trim();
  expect(firstHeaderText.length).toBeGreaterThan(0);
});

// ----------------------------------------------------------------
// Test 3: The seeded album appears somewhere on the home page
//
// With only album 2001 in the library and no playback history, the home page
// picks it for the "Jump back into" fallback section (random shuffle of all
// albums). It may also appear in "Don't forget about" or "Do some crate digging".
// We assert the MediaCard testid for album 2001 is present anywhere on the page.
// ----------------------------------------------------------------

test('seeded album (Playwright Album) appears on the home page', async () => {
  // Wait for at least one bento section to render before checking for the card
  await page.waitForSelector('[data-testid^="home-section-"]', { timeout: 15_000 });

  // Give the grid a moment to populate — sections are computed via useMemo
  // after grid dimensions are calculated on first ResizeObserver callback.
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 4: Navigation links are visible from the home page
// ----------------------------------------------------------------

test('all nav links are visible from the home page', async () => {
  // These nav items must be present in the sidebar when on the home page
  await expect(page.locator('[data-testid="nav-home"]')).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('[data-testid="nav-artists"]')).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('[data-testid="nav-playlists"]')).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('[data-testid="nav-tracks"]')).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 5: Clicking nav-albums from home navigates to the Albums list
// ----------------------------------------------------------------

test('clicking nav-albums from home navigates to the Albums list', async () => {
  // Confirm we are on the home page first
  await expect(page.locator('[data-testid="home-page"]')).toBeVisible({ timeout: 5_000 });

  // Click the Albums nav item
  await page.click('[data-testid="nav-albums"]', { force: true });

  // The Albums page shows MediaCard entries for albums — wait for album 2001
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="media-card-album-2001"]')).toBeVisible();

  // The home-page container should no longer be present in the DOM
  // (React Router replaces the route)
  await expect(page.locator('[data-testid="home-page"]')).not.toBeVisible({ timeout: 5_000 });
});
