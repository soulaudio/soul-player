/**
 * Album Page Sorting & Filtering E2E tests — Playwright CDP
 *
 * Tests the Albums grid page functionality: search filtering,
 * card rendering, navigation to detail, and the grid layout.
 *
 * 7 tests:
 *   1. Albums page shows all 3 seeded albums
 *   2. Search filters albums by name
 *   3. Clearing search shows all albums again
 *   4. Clicking album card navigates to detail page
 *   5. Album detail shows correct track count
 *   6. Back navigation from detail returns to albums grid
 *   7. Albums page search shows correct count in placeholder
 *
 * Seed data:
 *   Album 2001 — "Playwright Album" (5 tracks)
 *   Album 2002 — "Long Album" (5 tracks)
 *   Album 2003 — "Marathon Album" (10 tracks)
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
  if (!page) throw new Error('Main window not found');
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
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 15_000 });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 10_000 });
});

test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(100);
});

// ── Test 1: All 3 albums visible ──

test('Albums page shows all 3 seeded albums', async () => {
  const cards = page.locator('[data-testid^="media-card-album-"]');
  const count = await cards.count();
  expect(count).toBe(3);

  await expect(page.locator('[data-testid="media-card-album-2001"]')).toBeVisible();
  await expect(page.locator('[data-testid="media-card-album-2002"]')).toBeVisible();
  await expect(page.locator('[data-testid="media-card-album-2003"]')).toBeVisible();
});

// ── Test 2: Search filters by name ──

test('search input filters albums by name', async () => {
  const searchInput = page.locator('input[type="text"], input[type="search"]').first();
  await searchInput.fill('Playwright');
  await page.waitForTimeout(500);

  const cards = page.locator('[data-testid^="media-card-album-"]');
  const count = await cards.count();
  expect(count).toBe(1);

  await expect(page.locator('[data-testid="media-card-album-2001"]')).toBeVisible();
  await expect(page.locator('[data-testid="media-card-album-2002"]')).not.toBeVisible();
});

// ── Test 3: Clearing search shows all ──

test('clearing search shows all albums again', async () => {
  const searchInput = page.locator('input[type="text"], input[type="search"]').first();
  await searchInput.fill('Playwright');
  await page.waitForTimeout(500);

  // Clear
  await searchInput.fill('');
  await page.waitForTimeout(500);

  const cards = page.locator('[data-testid^="media-card-album-"]');
  const count = await cards.count();
  expect(count).toBe(3);
});

// ── Test 4: Click album navigates to detail ──

test('clicking album card navigates to album detail page', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  // Click the card title to navigate
  const titleEl = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleEl.click();

  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await expect(page.locator('[data-testid="album-detail-page"]')).toBeVisible();
});

// ── Test 5: Album detail shows correct track count ──

test('album detail page shows correct number of tracks', async () => {
  // Navigate to Playwright Album (5 tracks)
  const card = page.locator('[data-testid="media-card-album-2001"]');
  const titleEl = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleEl.click();

  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  const trackRows = page.locator('[data-testid="track-row"]');
  const count = await trackRows.count();
  expect(count).toBe(5);
});

// ── Test 6: Back navigation ──

test('navigating back from album detail returns to albums grid', async () => {
  // Go to detail
  const card = page.locator('[data-testid="media-card-album-2001"]');
  const titleEl = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleEl.click();
  await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });

  // Navigate back via nav
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 15_000 });

  const cards = page.locator('[data-testid^="media-card-album-"]');
  const count = await cards.count();
  expect(count).toBe(3);
});

// ── Test 7: Search placeholder shows correct count ──

test('search placeholder shows correct album count', async () => {
  const searchInput = page.locator('input[type="text"], input[type="search"]').first();
  const placeholder = await searchInput.getAttribute('placeholder');

  // Should contain "3" (the album count)
  expect(placeholder).toContain('3');
});
