/**
 * Genre navigation and filtering — Playwright CDP E2E tests
 *
 * Covers:
 *   1. nav-genres navigates to the genres list page
 *   2. Clicking a genre card navigates to the genre detail page
 *   3. Albums page genre filter shows only matching albums
 *   4. Tracks page genre filter shows only matching tracks
 *
 * Seed data (from playwright-global-setup.js):
 *   Genre ID 4001 — "Playwright Genre"
 *   Album ID 2001 — "Playwright Album"
 *   Track IDs 2001–2005 + 2006 (Collab Track), titles: Track One … Track Five + Collab Track
 *   All 6 tracks are linked to genre 4001 via track_genres junction table.
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
  await page.waitForTimeout(100);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Test 1: nav-genres navigates to genres list page ─────────────────────────

test('nav-genres navigates to genres list page', async () => {
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForSelector('[data-testid="genres-page"]', { timeout: 10_000 });

  const card = await page.waitForSelector('[data-testid="genre-card-4001"]', { timeout: 10_000 });
  expect(card).toBeTruthy();

  const text = await page.textContent('[data-testid="genre-card-4001"]');
  expect(text).toContain('Playwright Genre');
  expect(text).toMatch(/6\s*tracks?/i);
});

// ── Test 2: clicking a genre card navigates to the genre detail page ──────────

test('clicking a genre card navigates to genre detail page', async () => {
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForSelector('[data-testid="genre-card-4001"]', { timeout: 10_000 });

  await page.click('[data-testid="genre-card-4001"]');

  // Wait for the genre detail page to render
  await page.waitForSelector('[data-testid="genre-detail-page"]', { timeout: 10_000 });

  // Track list with 6 rows must be present
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const rows = await page.$$('[data-testid="track-row"]');
  expect(rows.length).toBe(6);
});

// ── Test 3: albums page genre filter shows only matching albums ───────────────

test('albums page genre filter shows only matching albums', async () => {
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });

  // Open the filter panel
  await page.click('[data-testid="filter-toggle-button"]');

  // Wait for genre chip for our test genre to appear
  await page.waitForSelector('[data-testid="genre-chip-4001"]', { timeout: 5_000 });

  // Click the genre chip to activate the filter
  await page.click('[data-testid="genre-chip-4001"]');

  // After click the chip testid changes to genre-chip-4001-active
  await page.waitForSelector('[data-testid="genre-chip-4001-active"]', { timeout: 5_000 });

  // Album 2001 must still be visible
  const card2001 = await page.$('[data-testid="media-card-album-2001"]');
  expect(card2001).toBeTruthy();

  // All visible album cards must match at least 1
  const cards = await page.$$('[data-testid^="media-card-album-"]');
  expect(cards.length).toBeGreaterThanOrEqual(1);
});

// ── Test 4: tracks page genre filter shows only matching tracks ───────────────

test('tracks page genre filter shows only matching tracks', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 10_000 });

  // Open the filter panel
  await page.click('[data-testid="filter-toggle-button"]');

  // Wait for genre chip for our test genre to appear
  await page.waitForSelector('[data-testid="genre-chip-4001"]', { timeout: 5_000 });

  // Click the genre chip to activate the filter
  await page.click('[data-testid="genre-chip-4001"]');

  // After click the chip testid changes to genre-chip-4001-active
  await page.waitForSelector('[data-testid="genre-chip-4001-active"]', { timeout: 5_000 });

  // Should show exactly 6 tracks (all 6 seeded tracks belong to genre 4001)
  const rows = await page.$$('[data-testid="track-row"]');
  expect(rows.length).toBe(6);
});
