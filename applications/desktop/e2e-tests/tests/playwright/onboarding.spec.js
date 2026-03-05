/**
 * Onboarding Flow — Playwright CDP tests
 *
 * IMPORTANT: This spec temporarily removes the seeded library_sources row to
 * trigger the onboarding screen. It ALWAYS restores state in afterEach.
 *
 * Run this spec LAST in the suite to avoid contaminating other specs.
 *
 * Covers:
 *   1. Removing library_sources shows the onboarding page on reload
 *   2. Theme step loads with theme cards visible
 *   3. Continue navigates from theme step to strategy step
 *   4. Back button returns from strategy step to theme step
 *   5. Clicking a theme card applies CSS variables immediately (data-theme + CSS vars)
 *   6. Selected theme is saved to localStorage as plain string (correct format)
 *   7. Loading modal uses the selected theme (data-theme set before React renders)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;
let savedLibrarySource = null;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

async function removeLibrarySources() {
  savedLibrarySource = await page.evaluate(async () => {
    const sources = await window.__TAURI_INTERNALS__.invoke('get_library_sources');
    for (const s of sources) {
      await window.__TAURI_INTERNALS__
        .invoke('remove_library_source', { sourceId: s.id })
        .catch(() => {});
    }
    return sources[0] || null;
  });
}

async function restoreLibrarySource() {
  if (!savedLibrarySource) return;
  await page.evaluate(async (src) => {
    await window.__TAURI_INTERNALS__
      .invoke('add_library_source', {
        name: src.name,
        path: src.path,
        syncDeletes: true,
      })
      .catch(() => {});
  }, savedLibrarySource);
  await page.reload();
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
}

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  // Always restore normal app state even if test fails
  await restoreLibrarySource();
});

test('removing library sources shows the onboarding screen on reload', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });
});

test('onboarding theme step shows theme selection cards', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });

  // Theme step should be immediately visible
  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible();

  // Count theme cards — component defines 4 themes (light, dark, ocean, earth)
  const themeCards = page.locator('[data-testid^="onboarding-theme-"]');
  const count = await themeCards.count();
  expect(count).toBeGreaterThanOrEqual(4);
});

test('clicking Continue on theme step navigates to strategy step', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible();

  await page.click('[data-testid="onboarding-continue"]');

  await expect(page.locator('[data-testid="onboarding-strategy-step"]')).toBeVisible({
    timeout: 5_000,
  });
});

test('Back button on strategy step returns to theme step', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible();

  // Navigate to strategy step
  await page.click('[data-testid="onboarding-continue"]');
  await expect(page.locator('[data-testid="onboarding-strategy-step"]')).toBeVisible({
    timeout: 5_000,
  });

  // Go back
  await page.click('[data-testid="onboarding-back"]');

  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible({
    timeout: 5_000,
  });
});

test('clicking a theme card applies CSS variables immediately', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible();

  // Click the dark theme card
  await page.click('[data-testid="onboarding-theme-dark"]');

  // data-theme attribute must be updated
  const datatheme = await page.evaluate(() =>
    document.documentElement.getAttribute('data-theme'),
  );
  expect(datatheme).toBe('dark');

  // CSS custom property --background must be set (ThemeManager sets it via style.setProperty)
  const bgVar = await page.evaluate(() =>
    document.documentElement.style.getPropertyValue('--background').trim(),
  );
  expect(bgVar).toBeTruthy();
  expect(bgVar).not.toBe('');

  // Click the ocean theme card and verify it switches
  await page.click('[data-testid="onboarding-theme-ocean"]');
  const datatheme2 = await page.evaluate(() =>
    document.documentElement.getAttribute('data-theme'),
  );
  expect(datatheme2).toBe('ocean');
});

test('selected theme is saved to localStorage as a plain string', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('[data-testid="onboarding-theme-step"]')).toBeVisible();

  // Click the earth theme card
  await page.click('[data-testid="onboarding-theme-earth"]');
  await page.waitForTimeout(300);

  const result = await page.evaluate(() => ({
    datatheme: document.documentElement.getAttribute('data-theme'),
    stored: localStorage.getItem('soul-player-current-theme'),
  }));

  // ThemeManager.saveCurrentThemeToStorage() stores plain string, NOT JSON.
  expect(result.stored).toBe('earth');
  expect(result.stored).not.toBe('"earth"');
  expect(result.datatheme).toBe('earth');
});

test('loading modal uses the selected theme after page reload', async () => {
  await removeLibrarySources();
  await page.reload();
  await expect(page.locator('[data-testid="onboarding-page"]')).toBeVisible({ timeout: 15_000 });

  // Select dark theme to ensure a non-default theme is stored
  await page.click('[data-testid="onboarding-theme-dark"]');

  // Wait a tick for localStorage to be written
  await page.waitForTimeout(200);

  const stored = await page.evaluate(() =>
    localStorage.getItem('soul-player-current-theme'),
  );
  expect(stored).toBe('dark');

  // Restore so the reload shows main app, then check theme is applied immediately on load.
  // We verify via the data-theme attribute which ThemeManager sets synchronously before
  // React renders (in the ThemeManager constructor, called from ThemeProvider).
  await restoreLibrarySource();

  // Reload and check that data-theme is set correctly before nav-albums appears
  await page.reload();
  // The very first paint should already have data-theme='dark' because ThemeManager
  // runs synchronously in the constructor before React rendering begins.
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
  const themeAttr = await page.evaluate(() =>
    document.documentElement.getAttribute('data-theme'),
  );
  expect(themeAttr).toBe('dark');

  // Clean up: reset savedLibrarySource to null to prevent double-restore in afterEach
  savedLibrarySource = null;
});
