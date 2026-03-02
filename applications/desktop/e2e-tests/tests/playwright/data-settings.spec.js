/**
 * Data settings — Playwright CDP tests
 *
 * TDD suite for the Music Data settings page (/settings/music-data).
 * Tests encode expected behavior; failures direct us to fix the implementation.
 *
 * Covers:
 *  1. Navigating to Music Data settings shows Library Settings and Data
 *     Management sections
 *  2. Watched folders list shows the seeded library_sources row
 *  3. Rescan button on a watched folder triggers scan-progress-indicator
 *  4. Reset to factory settings button opens confirmation dialog;
 *     clicking Cancel dismisses it without losing data
 *
 * Navigation pattern (mirrors audio-settings.spec.js):
 *   settings-button → nav-settings-about (sidebar ready) → nav-settings-musicData
 *   → library-sources-toggle (page loaded)
 *
 * Seed data (from playwright-global-setup.js):
 *   - library_sources row: device_id='desktop-local', path = audioDir
 *     This is the seeded watched folder visible in LibrarySettingsPage.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );

  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Navigate to settings → Music Data before each test.
test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Open settings panel.
  await page.click('[data-testid="settings-button"]', { force: true });

  // Wait for sidebar to be ready (any nav item confirms it).
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });

  // Navigate to the Music Data section.
  await page.click('[data-testid="nav-settings-musicData"]');

  // Wait for the Library Sources toggle — confirms MusicDataSettingsPage rendered.
  // LibrarySettingsPage has a loading state (spinner) while fetching library sources;
  // allow up to 25 s for the component to finish loading on slower machines.
  await page.waitForSelector('[data-testid="library-sources-toggle"]', { timeout: 25_000 });
});

test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ================================================================
// Test 1: Music Data settings page shows expected sections
//
// TDD target: both LibrarySettingsPage (library-sources-toggle) and
// DataManagementSettingsPage (reset-factory-settings-button) must be
// present after navigating to /settings/music-data.
// ================================================================

test('music data settings page shows library and data management sections', async () => {
  // Library settings section toggle must be visible.
  const sourcesToggle = page.locator('[data-testid="library-sources-toggle"]');
  await expect(sourcesToggle).toBeVisible();

  // Data management section with reset button must be visible.
  const resetBtn = page.locator('[data-testid="reset-factory-settings-button"]');
  await expect(resetBtn).toBeVisible();
});

// ================================================================
// Test 2: Watched folders list shows the seeded library source
//
// TDD target: after expanding the Library Sources section, the seeded
// library_sources row (device_id='desktop-local') must appear as a
// watch-folder-item in the list.
// ================================================================

test('watched folders list shows the seeded library source', async () => {
  // The Watched Folders section starts pre-expanded (expandedSection initialises to
  // 'sources'), so watch-folder-item entries are immediately visible without clicking
  // the toggle.
  await page.waitForSelector('[data-testid^="watch-folder-item-"]', { timeout: 5_000 });

  const items = page.locator('[data-testid^="watch-folder-item-"]');
  await expect(items.first()).toBeVisible();

  const count = await items.count();
  expect(count).toBeGreaterThanOrEqual(1);
});

// ================================================================
// Test 3: Rescan button on a watched folder triggers scan-progress-indicator
//
// TDD target: clicking [data-testid="rescan-button-{id}"] on the first
// watched folder fires rescan_library_source, which triggers scan-started
// → ScanProgressIndicator renders.
//
// This exercises: settings UI → Tauri command → scan event → React component.
// ================================================================

test('rescan button on watched folder triggers scan-progress-indicator', async () => {
  test.setTimeout(30_000);

  // The Watched Folders section starts pre-expanded — rescan buttons are immediately
  // visible after beforeEach loads the settings page.
  await page.waitForSelector('[data-testid^="rescan-button-"]', { timeout: 5_000 });

  // Click the rescan button on the first source.
  const rescanBtn = page.locator('[data-testid^="rescan-button-"]').first();
  await rescanBtn.click();

  // ScanProgressIndicator must appear within 5 s.
  await page.waitForSelector('[data-testid="scan-progress-indicator"]', { timeout: 10_000 });

  const indicator = page.locator('[data-testid="scan-progress-indicator"]');
  await expect(indicator).toBeVisible();

  // Wait for the scan to finish (cleanup — leaves app in a clean state).
  await page
    .waitForSelector('[data-testid="scan-progress-indicator"]', {
      state: 'hidden',
      timeout: 45_000,
    })
    .catch(() => {});
});

// ================================================================
// Test 4: Reset to factory settings opens confirmation dialog;
//         clicking Cancel dismisses it without resetting data
//
// TDD target:
//   - Clicking reset-factory-settings-button shows reset-confirm-dialog
//   - Clicking reset-dialog-cancel-button closes the dialog
//   - The watched folders list still shows at least one item (data intact)
//
// Guards against: dialog auto-confirming, Cancel not working, data loss.
// ================================================================

test('reset dialog appears on button click and Cancel dismisses it safely', async () => {
  // Click the Reset to Factory Settings button.
  const resetBtn = page.locator('[data-testid="reset-factory-settings-button"]');
  await resetBtn.click();

  // Confirmation dialog must appear.
  const dialog = page.locator('[data-testid="reset-confirm-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 3_000 });

  // Click Cancel.
  const cancelBtn = page.locator('[data-testid="reset-dialog-cancel-button"]');
  await expect(cancelBtn).toBeVisible();
  await cancelBtn.click();

  // Dialog must be dismissed.
  await expect(dialog).not.toBeVisible({ timeout: 3_000 });

  // Data must be intact: the Watched Folders section is still expanded
  // (the dialog cancel doesn't affect section state), so items are still visible.
  // (A completed reset would have restarted the app and closed the window.)
  await page.waitForSelector('[data-testid^="watch-folder-item-"]', { timeout: 5_000 });

  const items = page.locator('[data-testid^="watch-folder-item-"]');
  const count = await items.count();
  expect(count).toBeGreaterThanOrEqual(1);
});
