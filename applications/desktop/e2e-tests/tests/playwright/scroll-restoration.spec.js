/**
 * Scroll Restoration — Playwright CDP E2E tests
 *
 * Verifies that when the user navigates from a list page (Albums, Artists, Tracks)
 * to a detail page and then goes back, the list page's scroll position is restored
 * to where they left off.
 *
 * Pattern:
 *   1. Navigate to list page
 *   2. Programmatically scroll the scroll container to a known position
 *   3. Navigate to a detail page (causing LibraryPageLayout to unmount & save position)
 *   4. Navigate back to the list page (causing LibraryPageLayout to mount & restore position)
 *   5. Assert scrollTop matches what was set in step 2
 *
 * Back navigation is done via pushState + popstate so we can control it precisely
 * without depending on back-button testids.
 *
 * FAILS before implementation (scroll-container testid missing + no restoration hook).
 * PASSES after:
 *   - data-testid="scroll-container" added to LibraryPageLayout's scrollable div
 *   - useScrollRestoration hook created and called in LibraryPageLayout
 *
 * Seed data available (from playwright-global-setup.js):
 *   Albums:  2001 "Playwright Album", 2002 "Long Album", 2003 "Marathon Album"
 *   Artists: 2001 "Playwright Artist"
 *   Tracks:  2001–2005 (album 2001), 3001–3005 (album 2002), 4001–4010 (album 2003)
 *            = 15 total tracks (enough to overflow a standard viewport at 56px/row)
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
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(150);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(150);
});

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

/** Read scrollTop of the page's scroll container. */
async function getScrollTop(p) {
  return p.evaluate(() => {
    const el = document.querySelector('[data-testid="scroll-container"]');
    return el ? el.scrollTop : -1;
  });
}

/**
 * Set scrollTop on the scroll container and fire a scroll event so the
 * useScrollRestoration listener captures the new position.
 * Returns the actual scrollTop after setting (browser clamps to valid range).
 */
async function setScrollTop(p, value) {
  return p.evaluate((target) => {
    const el = document.querySelector('[data-testid="scroll-container"]');
    if (!el) return -1;
    el.scrollTop = target;
    el.dispatchEvent(new Event('scroll', { bubbles: false }));
    return el.scrollTop;
  }, value);
}

/** Navigate using React Router via pushState + popstate. */
async function navigateTo(p, path, fromPath) {
  await p.evaluate(({ to, from }) => {
    const state = from ? { from } : {};
    window.history.pushState(state, '', to);
    window.dispatchEvent(new PopStateEvent('popstate', { state }));
  }, { to: path, from: fromPath });
}

// ----------------------------------------------------------------
// Test 1: Albums page restores scroll after visiting album detail
// ----------------------------------------------------------------

test('albums page scroll position is restored after navigating to album detail and back', async () => {
  // Navigate to albums
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 15_000 });

  // The scroll container must exist (FAILS before testid is added)
  await expect(page.locator('[data-testid="scroll-container"]')).toBeVisible({ timeout: 5_000 });

  // Scroll to a known position — browser clamps so we read back the actual value
  const target = 150;
  const actual = await setScrollTop(page, target);

  // If the page has no overflow (all cards fit without scrolling), skip this test
  // to avoid asserting against a position of 0 that was never set.
  test.skip(actual === 0, 'Albums page not tall enough to scroll at current viewport size');

  // Give the listener one tick to record the position
  await page.waitForTimeout(50);

  // Navigate to album detail, setting from=/albums in state so back works
  await navigateTo(page, '/albums/2001', '/albums');
  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 10_000 });

  // Navigate back to albums
  await navigateTo(page, '/albums', null);
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });

  // Wait one rAF for scroll restoration to apply
  await page.waitForTimeout(100);

  const restored = await getScrollTop(page);
  expect(restored).toBeGreaterThanOrEqual(actual - 5); // within 5px tolerance
  expect(restored).toBeLessThanOrEqual(actual + 5);
});

// ----------------------------------------------------------------
// Test 2: Tracks page restores scroll (15 rows × ~56px = reliable overflow)
// ----------------------------------------------------------------

test('tracks page scroll position is restored after navigating away and back', async () => {
  // Navigate to tracks
  await navigateTo(page, '/tracks', null);
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 15_000 });
  await page.waitForSelector('[data-testid="scroll-container"]', { timeout: 5_000 });

  // Scroll down — with 15 tracks at ~56px each the page overflows most viewports
  const target = 200;
  const actual = await setScrollTop(page, target);
  test.skip(actual === 0, 'Tracks page not tall enough to scroll');

  await page.waitForTimeout(50);

  // Navigate to album detail from tracks page
  await navigateTo(page, '/albums/2001', '/tracks');
  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 10_000 });

  // Navigate back to tracks
  await navigateTo(page, '/tracks', null);
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 10_000 });

  await page.waitForTimeout(100);

  const restored = await getScrollTop(page);
  expect(restored).toBeGreaterThanOrEqual(actual - 5);
  expect(restored).toBeLessThanOrEqual(actual + 5);
});

// ----------------------------------------------------------------
// Test 3: Artists page restores scroll after visiting artist detail
// ----------------------------------------------------------------

test('artists page scroll position is restored after navigating to artist detail and back', async () => {
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid="artists-page"]', { timeout: 15_000 });
  await page.waitForSelector('[data-testid="scroll-container"]', { timeout: 5_000 });

  const target = 150;
  const actual = await setScrollTop(page, target);
  test.skip(actual === 0, 'Artists page not tall enough to scroll');

  await page.waitForTimeout(50);

  await navigateTo(page, '/artists/2001', '/artists');
  await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 10_000 });

  await navigateTo(page, '/artists', null);
  await page.waitForSelector('[data-testid="artists-page"]', { timeout: 10_000 });

  await page.waitForTimeout(100);

  const restored = await getScrollTop(page);
  expect(restored).toBeGreaterThanOrEqual(actual - 5);
  expect(restored).toBeLessThanOrEqual(actual + 5);
});

// ----------------------------------------------------------------
// Test 4: Scroll position is NOT restored when navigating to a different list page
// (each page has its own independent scroll state)
// ----------------------------------------------------------------

test('scroll position is independent per page (albums and tracks have separate positions)', async () => {
  // Set a scroll position on albums page
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 15_000 });
  const albumsScrollActual = await setScrollTop(page, 120);
  await page.waitForTimeout(50);

  // Navigate to tracks page — should NOT inherit albums scroll position
  await navigateTo(page, '/tracks', '/albums');
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 10_000 });
  await page.waitForTimeout(100);

  // Tracks scroll should be INDEPENDENT from albums scroll — it must not equal
  // albumsScrollActual. (Tracks may have a previously saved position from another
  // test run within this session, but it should never carry over from albums.)
  if (albumsScrollActual > 0) {
    const tracksScrollTop = await getScrollTop(page);
    expect(tracksScrollTop).not.toBe(albumsScrollActual);
  }

  // Navigate back to albums — should restore albums position
  await navigateTo(page, '/albums', '/tracks');
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });
  await page.waitForTimeout(100);

  if (albumsScrollActual > 0) {
    const albumsRestored = await getScrollTop(page);
    expect(albumsRestored).toBeGreaterThanOrEqual(albumsScrollActual - 5);
  }
});

// ----------------------------------------------------------------
// Test 5: Scroll container exists on all major list pages
// (smoke test that the testid is present everywhere it should be)
// ----------------------------------------------------------------

test('scroll-container testid is present on all major list pages', async () => {
  const pages = [
    { nav: '[data-testid="nav-albums"]', page: '[data-testid="albums-page"]' },
    { nav: '[data-testid="nav-artists"]', page: '[data-testid="artists-page"]' },
    { nav: '[data-testid="nav-tracks"]', page: '[data-testid="tracks-page"]' },
    { nav: '[data-testid="nav-playlists"]', page: '[data-testid="playlists-page"]' },
  ];

  for (const { nav, page: pageTestId } of pages) {
    await page.click(nav, { force: true });
    await page.waitForSelector(pageTestId, { timeout: 15_000 });

    const scrollContainer = page.locator('[data-testid="scroll-container"]');
    await expect(scrollContainer).toBeVisible({ timeout: 5_000 });
  }
});
