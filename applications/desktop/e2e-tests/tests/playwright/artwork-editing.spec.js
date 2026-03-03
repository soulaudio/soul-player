/**
 * Artwork editing — Playwright CDP tests
 *
 * Verifies that right-clicking an album card shows an "Edit Artwork" menu item
 * and that the EditArtworkDialog works correctly:
 *   1. Dialog opens from album card right-click context menu
 *   2. Dialog shows drop zone and browse/select button in initial (select) state
 *   3. Cancel button closes the dialog without changes
 *   4. Escape key closes the dialog
 *   5. File input accepts image files and transitions to the crop state
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const TEST_IMAGE_PATH = join(__dirname, '..', '..', 'fixtures', 'test-artwork.png');

// ---- CDP connection shared across tests in this file ----

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

// Before each test: navigate to Albums and wait for cards
test.beforeEach(async () => {
  // Dismiss any leftover context menu or dialog
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // Use force:true in case a backdrop overlay is still present
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });

  // Wait for album 2001 specifically
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });
});

// After each test: close any open overlays
test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  // Press Escape again in case we're in crop state which needs another close
  await page.keyboard.press('Escape').catch(() => {});
});

// ----------------------------------------------------------------

test('artwork dialog opens from album card right-click menu', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  // Right-click to open context menu
  await card.click({ button: 'right' });

  // Context menu must appear
  const menu = page.locator('[role="menu"]');
  await expect(menu).toBeVisible({ timeout: 5_000 });

  // "Edit Artwork" item must be present
  const editArtworkItem = page.locator('[data-testid="context-menu-edit-artwork"]');
  await expect(editArtworkItem).toBeVisible({ timeout: 3_000 });

  // Click "Edit Artwork"
  await editArtworkItem.click();

  // Dialog must appear
  const dialog = page.locator('[data-testid="edit-artwork-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });
});

test('artwork dialog shows drop zone and browse button in select state', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });
  const editArtworkItem = page.locator('[data-testid="context-menu-edit-artwork"]');
  await expect(editArtworkItem).toBeVisible({ timeout: 5_000 });
  await editArtworkItem.click();

  const dialog = page.locator('[data-testid="edit-artwork-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  // Drop zone and select button must be visible in initial select state
  const dropZone = page.locator('[data-testid="artwork-drop-zone"]');
  await expect(dropZone).toBeVisible({ timeout: 5_000 });

  const selectButton = page.locator('[data-testid="artwork-select-button"]');
  await expect(selectButton).toBeVisible({ timeout: 5_000 });
});

test('cancel button closes artwork dialog without changes', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });
  const editArtworkItem = page.locator('[data-testid="context-menu-edit-artwork"]');
  await expect(editArtworkItem).toBeVisible({ timeout: 5_000 });
  await editArtworkItem.click();

  const dialog = page.locator('[data-testid="edit-artwork-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  // Click cancel button
  const cancelButton = page.locator('[data-testid="artwork-cancel-button"]');
  await expect(cancelButton).toBeVisible({ timeout: 5_000 });
  await cancelButton.click();

  // Dialog must be hidden
  await expect(dialog).not.toBeVisible({ timeout: 5_000 });
});

test('Escape key closes artwork dialog', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });
  const editArtworkItem = page.locator('[data-testid="context-menu-edit-artwork"]');
  await expect(editArtworkItem).toBeVisible({ timeout: 5_000 });
  await editArtworkItem.click();

  const dialog = page.locator('[data-testid="edit-artwork-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  // Press Escape
  await page.keyboard.press('Escape');

  // Dialog must close
  await expect(dialog).not.toBeVisible({ timeout: 5_000 });
});

test('file input accepts image files and transitions to crop state', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });

  await card.click({ button: 'right' });
  const editArtworkItem = page.locator('[data-testid="context-menu-edit-artwork"]');
  await expect(editArtworkItem).toBeVisible({ timeout: 5_000 });
  await editArtworkItem.click();

  const dialog = page.locator('[data-testid="edit-artwork-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  // Use setInputFiles on the hidden file input to simulate a file selection
  const fileInput = page.locator('[data-testid="artwork-file-input"]');
  await fileInput.setInputFiles(TEST_IMAGE_PATH);

  // After a file is selected, the component reads it and transitions to 'crop' state
  // The crop container should become visible
  const cropper = page.locator('[data-testid="artwork-cropper"]');
  await expect(cropper).toBeVisible({ timeout: 10_000 });
});
