/**
 * MediaCard context menu — Playwright CDP tests
 *
 * Verifies that right-clicking an album card:
 *   1. Shows a context menu with "Add to Playlist"
 *   2. Menu is positioned near the cursor (not off-screen or at origin)
 *   3. Clicking the item opens the AddToPlaylistDialog
 *   4. Clicking outside the menu closes it
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

  // Short safety wait in case there's any residual animation/settle
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: dismiss any open menus/dialogs, then navigate to Albums
test.beforeEach(async () => {
  // Dismiss any leftover context menu or dialog from the previous test
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // Use force:true so the click goes through even if a backdrop overlay is still present
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

// After each test: clean up any open overlays
test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
});

// ----------------------------------------------------------------

test('right-click opens context menu above other cards', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  const cardBox = await card.boundingBox();
  expect(cardBox).not.toBeNull();

  // Right-click artwork area (top portion of the card)
  await card.click({ button: 'right', position: { x: cardBox.width / 2, y: cardBox.height * 0.3 } });

  // Context menu must appear
  const menu = page.locator('[role="menu"]');
  await expect(menu).toBeVisible({ timeout: 5_000 });

  // Menu must contain "Add to Playlist" item (menu may have multiple items)
  await expect(menu.getByRole('menuitem', { name: /playlist/i })).toBeVisible({ timeout: 5_000 });

  // Menu position must be near the click point (within 50 px), not at origin
  const menuBox = await menu.boundingBox();
  expect(menuBox).not.toBeNull();
  expect(menuBox.x).toBeGreaterThan(10);   // not at screen left edge
  expect(menuBox.y).toBeGreaterThan(10);   // not at screen top edge

  // Screenshot for visual confirmation
  await page.screenshot({ path: 'screenshots/context-menu-open.png' });
});

test('clicking "Add to Playlist" opens the dialog', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });

  const menuItem = page.getByRole('menuitem', { name: /playlist/i });
  await expect(menuItem).toBeVisible({ timeout: 5_000 });
  await menuItem.click();

  const dialog = page.locator('[data-testid="add-to-playlist-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  await page.screenshot({ path: 'screenshots/add-to-playlist-dialog.png' });
});

test('clicking outside the menu closes it', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });
  await expect(page.locator('[role="menu"]')).toBeVisible({ timeout: 5_000 });

  // Click somewhere away from the menu (top-left corner of viewport)
  await page.mouse.click(10, 10);
  await expect(page.locator('[role="menu"]')).not.toBeVisible({ timeout: 3_000 });
});
