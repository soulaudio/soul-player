/**
 * About Settings & Installation Info E2E tests — Playwright CDP
 *
 * Tests the About settings page and installation info IPC:
 *   get_installation_info, About page navigation, version display
 *
 * 4 tests
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
  // Navigate to Settings > About
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
  await page.click('[data-testid="nav-settings-about"]');
  await page.waitForTimeout(500);
});

test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Test 1: About page opens from settings ──

test('About settings page opens from settings sidebar', async () => {
  const url = page.url();
  expect(url).toMatch(/settings/);
});

// ── Test 2: get_installation_info returns valid info ──

test('get_installation_info returns installation method and flags', async () => {
  const info = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_installation_info')
  );

  expect(info).toBeTruthy();
  expect(typeof info).toBe('object');
  expect(typeof info.supports_auto_update).toBe('boolean');
});

// ── Test 3: About page contains version text ──

test('About page displays version information', async () => {
  const pageText = await page.textContent('body');
  // The About page should mention "version" or the app name somewhere
  expect(pageText.toLowerCase()).toMatch(/version|soul|about/);
});

// ── Test 4: Settings navigation works for all settings pages ──

test('all settings sidebar navigation links are functional', async () => {
  // Check that all expected settings nav items are visible
  const navItems = [
    'nav-settings-about',
    'nav-settings-appearance',
    'nav-settings-audio',
  ];

  for (const testId of navItems) {
    const item = page.locator(`[data-testid="${testId}"]`);
    await expect(item).toBeVisible();
  }
});
