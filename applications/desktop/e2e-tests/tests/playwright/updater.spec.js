/**
 * Updater E2E tests — Playwright CDP
 *
 * Tests the update notification flow end-to-end:
 *   1. Settings page renders update section with correct initial state
 *   2. Backend event "update-available" → UpdateDialog appears
 *   3. UpdateDialog "Later" button dismisses it
 *   4. "Check for Updates" button → dialog when update is available (mocked)
 *   5. "Check for Updates" button → toast when already up-to-date (mocked)
 *   6. Install button shows progress state (install_update mocked to avoid restart)
 *
 * Mocking strategy
 * ──────────────────
 * window.__TAURI_INTERNALS__.invoke is non-writable + non-configurable in Tauri v2,
 * so JS-side patching is impossible.  Instead we use Rust-side test commands that
 * store canned responses in static Mutex state — only active when PLAYWRIGHT_TEST_DIR
 * is set.  The real Tauri commands read this state before doing any real work.
 *
 *   set_test_update_response({ response, noUpdate })  — controls check_for_updates
 *   set_test_install_delay({ delayMs })               — controls install_update (u64, required → must use camelCase)
 *   emit_test_update_available({})                    — fires a real update-available event
 *
 * Tauri v2 camelCase rule
 * ────────────────────────
 * Required params (non-Optional) MUST use camelCase in JS: delayMs, not delay_ms.
 * Optional params (Option<T>) work with snake_case because Tauri defaults to None when key is not found.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ── CDP connection shared across all tests in this file ────────────────────

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Navigate to settings → About page before each test
test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-settings-about"]');
  await page.waitForSelector('[data-testid="check-for-updates-button"]', { timeout: 10_000 });
});

test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  // Reset Rust-side test state so canned responses don't bleed between tests
  // NOTE: Tauri snake_case params — use no_update / delay_ms, NOT camelCase
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', { response: null, no_update: false });
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 0, shouldFail: false });
  });
});

// ── Tests ──────────────────────────────────────────────────────────────────

test('settings page shows update section with controls', async () => {
  const autoUpdateToggle = page.locator('[data-testid="auto-update-toggle"]');
  await expect(autoUpdateToggle).toBeVisible();
  await expect(autoUpdateToggle).toBeChecked();

  const silentToggle = page.locator('[data-testid="silent-update-toggle"]');
  await expect(silentToggle).toBeVisible();
  await expect(silentToggle).not.toBeChecked();

  const checkBtn = page.locator('[data-testid="check-for-updates-button"]');
  await expect(checkBtn).toBeVisible();
  await expect(checkBtn).toBeEnabled();
});

test('update-available backend event shows update dialog', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('emit_test_update_available', {});
  });

  const dialog = page.locator('[data-testid="update-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 5_000 });

  const version = page.locator('[data-testid="update-dialog-version"]');
  await expect(version).toContainText('99.9.9');

  await page.screenshot({ path: 'screenshots/update-dialog-from-event.png' });
});

test('update dialog "Later" button dismisses the dialog', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('emit_test_update_available', {});
  });
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 5_000 });

  await page.click('[data-testid="update-dialog-later"]');

  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });
});

test('"Check for Updates" shows dialog when update is available', async () => {
  // Arm the Rust-side canned response before clicking
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', {
      response: { version: '42.0.0', date: null, body: 'Big release!' },
    });
  });

  await page.click('[data-testid="check-for-updates-button"]');

  const dialog = page.locator('[data-testid="update-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 8_000 });

  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('42.0.0');

  await page.screenshot({ path: 'screenshots/update-dialog-from-check.png' });
});

test('"Check for Updates" shows up-to-date toast when no update', async () => {
  // no_update: true → Rust returns Ok(None) from check_for_updates
  // NOTE: Tauri uses snake_case param names — must use no_update, not noUpdate
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', { no_update: true });
  });

  await page.click('[data-testid="check-for-updates-button"]');

  // No update dialog
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });

  // Up-to-date toast — translation key settings.upToDate = "You're on the latest version!"
  await expect(page.getByText(/latest version/i)).toBeVisible({ timeout: 5_000 });
});

test('Install Now button triggers installation progress', async () => {
  // Set a 5 second delay so the progress bar is visible before install completes.
  // NOTE: Tauri camelCase for required params — use delayMs (not delay_ms)
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 5000 });
  });

  // Open the dialog via the event
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('emit_test_update_available', {});
  });
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 5_000 });

  const installBtn = page.locator('[data-testid="update-dialog-install"]');
  await expect(installBtn).toBeVisible();
  await installBtn.click();

  // Progress bar should appear while installing (install_update is sleeping for 5s)
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 3_000 });

  await page.screenshot({ path: 'screenshots/update-installing.png' });
});
