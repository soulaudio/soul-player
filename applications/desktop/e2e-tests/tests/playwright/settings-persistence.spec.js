/**
 * Settings persistence — Playwright CDP E2E tests
 *
 * Verifies that user settings written via set_user_setting are readable via
 * get_user_setting and get_user_settings, and survive page navigation.
 *
 * Backend commands used:
 *   set_user_setting(key: string, value: JSON) → void
 *   get_user_setting(key: string) → JSON | null
 *   get_user_settings() → Array<{ key: string, value: JSON }>
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

async function setSetting(p, key, value) {
  return p.evaluate(
    async ({ k, v }) => window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: v }),
    { k: key, v: value }
  );
}

async function getSetting(p, key) {
  return p.evaluate(
    async (k) => window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k }),
    key
  );
}

async function getAllSettings(p) {
  return p.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_user_settings')
  );
}

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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
});

// ── Test 1: set and read back audio.volume ────────────────────────────────────

test('set and read back audio.volume', async () => {
  await setSetting(page, 'audio.volume', 75);
  const value = await getSetting(page, 'audio.volume');
  expect(value).toBe(75);
});

// ── Test 2: set and read back ui.theme ───────────────────────────────────────

test('set and read back ui.theme', async () => {
  await setSetting(page, 'ui.theme', 'ocean');
  const value = await getSetting(page, 'ui.theme');
  expect(value).toBe('ocean');
});

// ── Test 3: set and read back import.confidence_threshold ────────────────────

test('set and read back import.confidence_threshold', async () => {
  await setSetting(page, 'import.confidence_threshold', 90);
  const value = await getSetting(page, 'import.confidence_threshold');
  expect(value).toBe(90);
});

// ── Test 4: set and read back boolean false ───────────────────────────────────

test('set and read back app.auto_update_enabled = false', async () => {
  await setSetting(page, 'app.auto_update_enabled', false);
  const value = await getSetting(page, 'app.auto_update_enabled');
  expect(value).toBe(false);
});

// ── Test 5: settings persist across page navigation ──────────────────────────

test('settings persist across page navigation', async () => {
  // Write several settings
  await setSetting(page, 'audio.volume', 75);
  await setSetting(page, 'ui.theme', 'ocean');
  await setSetting(page, 'import.confidence_threshold', 90);
  await setSetting(page, 'app.auto_update_enabled', false);

  // Navigate away and back to force any in-memory caches to update
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForTimeout(500);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });

  // Settings must still be readable and return their last-written values
  expect(await getSetting(page, 'audio.volume')).toBe(75);
  expect(await getSetting(page, 'ui.theme')).toBe('ocean');
  expect(await getSetting(page, 'import.confidence_threshold')).toBe(90);
  expect(await getSetting(page, 'app.auto_update_enabled')).toBe(false);
});

// ── Test 6: get_user_settings returns all written keys ───────────────────────

test('get_user_settings returns all written keys with correct values', async () => {
  // Write a known set of settings
  await setSetting(page, 'audio.volume', 75);
  await setSetting(page, 'ui.theme', 'ocean');
  await setSetting(page, 'import.confidence_threshold', 90);
  await setSetting(page, 'app.auto_update_enabled', false);

  const all = await getAllSettings(page);
  expect(Array.isArray(all)).toBe(true);

  const keys = all.map(s => s.key);
  expect(keys).toContain('audio.volume');
  expect(keys).toContain('ui.theme');
  expect(keys).toContain('import.confidence_threshold');
  expect(keys).toContain('app.auto_update_enabled');

  // Verify value of audio.volume
  const volumeSetting = all.find(s => s.key === 'audio.volume');
  expect(volumeSetting).toBeTruthy();
  expect(volumeSetting.value).toBe(75);
});
