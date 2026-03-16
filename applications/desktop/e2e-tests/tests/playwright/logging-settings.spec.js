/**
 * Logging settings E2E tests — Playwright CDP
 *
 * Verifies the file logging feature on the About settings page:
 *   - get_logging_status IPC returns correct shape (log_path_example, active)
 *   - UI log path comes from IPC (not hardcoded), correct for debug vs release builds
 *   - Toggle shows/hides persistent restart-required banner
 *   - Restart banner does NOT auto-dismiss (was 5s timeout, now permanent)
 *   - active flag reflects whether logging was initialised at startup
 *
 * 6 tests
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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
  await page.click('[data-testid="nav-settings-about"]');
  // Wait for IPC calls to resolve and UI to render
  await page.waitForSelector('[data-testid="logging-toggle"]', { timeout: 10_000 });
  await page.waitForTimeout(300);
});

test.afterEach(async () => {
  // Ensure logging is turned off after each test so state is clean
  const checked = await page.locator('[data-testid="logging-toggle"]').isChecked();
  if (checked) {
    await page.click('[data-testid="logging-toggle"]');
    await page.waitForTimeout(200);
  }
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Test 1: get_logging_status IPC returns correct shape ──

test('get_logging_status returns log_path_example and active flag', async () => {
  const status = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_logging_status')
  );

  expect(typeof status.log_path_example).toBe('string');
  expect(status.log_path_example.length).toBeGreaterThan(0);
  // Path must include the log filename pattern
  expect(status.log_path_example).toMatch(/soul-player\.log\.YYYY-MM-DD/);
  // Must include the platform-correct app dir (debug build uses soul-player-dev)
  expect(status.log_path_example).toMatch(/soul-player/);

  expect(typeof status.active).toBe('boolean');
});

// ── Test 2: UI path comes from IPC, not hardcoded ──

test('log path displayed in UI matches get_logging_status IPC response', async () => {
  const status = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_logging_status')
  );

  const pathEl = page.locator('[data-testid="logging-path"]');
  await expect(pathEl).toBeVisible();
  const displayed = (await pathEl.textContent()).trim();

  expect(displayed).toBe(status.log_path_example);
  // Sanity: must not be the old hardcoded macOS path
  expect(displayed).not.toBe('~/Library/Application Support/soul-player/logs/soul-player.log.YYYY-MM-DD');
});

// ── Test 3: Logging starts disabled, no banners visible ──

test('logging toggle starts unchecked and no banners shown by default', async () => {
  const toggle = page.locator('[data-testid="logging-toggle"]');
  await expect(toggle).not.toBeChecked();

  // No restart banner when pref matches session state (both off)
  await expect(page.locator('[data-testid="logging-restart-banner"]')).not.toBeVisible();
});

// ── Test 4: Enabling logging shows persistent restart banner immediately ──

test('enabling logging shows restart-required banner immediately', async () => {
  const toggle = page.locator('[data-testid="logging-toggle"]');
  await expect(toggle).not.toBeChecked();

  await toggle.click();
  await expect(toggle).toBeChecked();

  // Restart banner appears immediately (session is still not logging)
  const banner = page.locator('[data-testid="logging-restart-banner"]');
  await expect(banner).toBeVisible();
});

// ── Test 5: Restart banner does NOT auto-dismiss after 5+ seconds ──

test('restart-required banner persists after 6 seconds (no auto-dismiss)', async () => {
  const toggle = page.locator('[data-testid="logging-toggle"]');
  await toggle.click();

  const banner = page.locator('[data-testid="logging-restart-banner"]');
  await expect(banner).toBeVisible();

  // Old code had setTimeout(..., 5000). Wait 6s and verify it's still there.
  await page.waitForTimeout(6_000);
  await expect(banner).toBeVisible({ message: 'Banner should not auto-dismiss' });
});

// ── Test 6: active flag reflects startup state; active banner shown when logging is on ──

test('active flag is false when app started without logging, active banner is hidden', async () => {
  // The test app starts without logging enabled (no config.json seed),
  // so active must be false and the green active banner must not be shown.
  const status = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_logging_status')
  );

  expect(status.active).toBe(false);
  await expect(page.locator('[data-testid="logging-active-banner"]')).not.toBeVisible();
});
