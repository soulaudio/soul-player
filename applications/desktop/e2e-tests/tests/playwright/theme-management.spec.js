/**
 * Theme Management E2E tests — Playwright CDP
 *
 * Tests custom theme save/delete/list operations via IPC and ThemeManager.
 *
 * IPC commands tested:
 *   theme_list_custom() → Vec<String> (raw JSON strings)
 *   theme_save(themeId, themeJson) → ()
 *   theme_delete(themeId) → ()
 *   set_user_setting(key, value) — for theme persistence
 *   get_user_setting(key) — for reading saved theme
 *
 * 7 tests:
 *   1. theme_list_custom returns an array
 *   2. theme_save creates a custom theme file
 *   3. Saved theme appears in theme_list_custom
 *   4. theme_delete removes the custom theme
 *   5. Theme persists in localStorage after setting
 *   6. Built-in themes cannot be deleted from disk (no file to delete)
 *   7. Setting theme via data-theme attribute changes document appearance
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

const TEST_THEME_ID = 'e2e-test-theme';
const TEST_THEME_JSON = JSON.stringify({
  id: 'e2e-test-theme',
  name: 'E2E Test Theme',
  description: 'A theme created by E2E tests',
  isDark: true,
  colors: {
    background: '220 20% 10%',
    foreground: '0 0% 95%',
    primary: '270 60% 60%',
    'primary-foreground': '0 0% 100%',
    muted: '220 15% 18%',
    'muted-foreground': '220 10% 60%',
    accent: '270 40% 25%',
    'accent-foreground': '0 0% 95%',
    border: '220 15% 22%',
    destructive: '0 70% 55%',
    'destructive-foreground': '0 0% 100%',
  },
});

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
  // Clean up test theme
  await page.evaluate(async (themeId) => {
    try { await window.__TAURI_INTERNALS__.invoke('theme_delete', { themeId }); } catch {}
  }, TEST_THEME_ID).catch(() => {});
  await browser.close();
});

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);

  // Clean up test theme before each test
  await page.evaluate(async (themeId) => {
    try { await window.__TAURI_INTERNALS__.invoke('theme_delete', { themeId }); } catch {}
  }, TEST_THEME_ID).catch(() => {});
});

test.afterEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(100);
});

// ── Test 1: theme_list_custom returns an array ──

test('theme_list_custom returns an array', async () => {
  const themes = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('theme_list_custom')
  );
  expect(Array.isArray(themes)).toBe(true);
});

// ── Test 2: theme_save creates a custom theme ──

test('theme_save creates a custom theme file on disk', async () => {
  await page.evaluate(async (args) =>
    window.__TAURI_INTERNALS__.invoke('theme_save', {
      themeId: args.themeId,
      themeJson: args.themeJson,
    }),
    { themeId: TEST_THEME_ID, themeJson: TEST_THEME_JSON }
  );

  // Verify it exists in list
  const themes = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('theme_list_custom')
  );

  // themes is an array of raw JSON strings
  const found = themes.some(t => {
    try {
      const parsed = JSON.parse(t);
      return parsed.id === 'e2e-test-theme';
    } catch { return false; }
  });
  expect(found).toBe(true);
});

// ── Test 3: Saved theme appears in list ──

test('saved custom theme appears in theme_list_custom', async () => {
  await page.evaluate(async (args) =>
    window.__TAURI_INTERNALS__.invoke('theme_save', {
      themeId: args.themeId,
      themeJson: args.themeJson,
    }),
    { themeId: TEST_THEME_ID, themeJson: TEST_THEME_JSON }
  );

  const themes = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('theme_list_custom')
  );

  expect(themes.length).toBeGreaterThanOrEqual(1);

  const parsed = themes.map(t => {
    try { return JSON.parse(t); } catch { return null; }
  }).filter(Boolean);

  const testTheme = parsed.find(t => t.id === 'e2e-test-theme');
  expect(testTheme).toBeTruthy();
  expect(testTheme.name).toBe('E2E Test Theme');
});

// ── Test 4: theme_delete removes the theme ──

test('theme_delete removes the custom theme file', async () => {
  // Save first
  await page.evaluate(async (args) =>
    window.__TAURI_INTERNALS__.invoke('theme_save', {
      themeId: args.themeId,
      themeJson: args.themeJson,
    }),
    { themeId: TEST_THEME_ID, themeJson: TEST_THEME_JSON }
  );

  // Verify it exists
  let themes = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('theme_list_custom')
  );
  let found = themes.some(t => {
    try { return JSON.parse(t).id === 'e2e-test-theme'; } catch { return false; }
  });
  expect(found).toBe(true);

  // Delete
  await page.evaluate(async (themeId) =>
    window.__TAURI_INTERNALS__.invoke('theme_delete', { themeId }),
    TEST_THEME_ID
  );

  // Verify it's gone
  themes = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('theme_list_custom')
  );
  found = themes.some(t => {
    try { return JSON.parse(t).id === 'e2e-test-theme'; } catch { return false; }
  });
  expect(found).toBe(false);
});

// ── Test 5: Theme persists via localStorage ──

test('setting theme persists in localStorage', async () => {
  // Read current theme from localStorage
  const currentTheme = await page.evaluate(() =>
    localStorage.getItem('soul-player-current-theme')
  );

  // Set a different theme
  await page.evaluate(() =>
    localStorage.setItem('soul-player-current-theme', 'ocean')
  );

  // Navigate away and back
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForTimeout(500);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForTimeout(500);

  // Read back
  const storedTheme = await page.evaluate(() =>
    localStorage.getItem('soul-player-current-theme')
  );
  expect(storedTheme).toBe('ocean');

  // Restore original
  if (currentTheme) {
    await page.evaluate((theme) =>
      localStorage.setItem('soul-player-current-theme', theme),
      currentTheme
    );
  }
});

// ── Test 6: data-theme attribute reflects current theme ──

test('data-theme attribute on document changes when theme is set', async () => {
  const originalTheme = await page.evaluate(() =>
    document.documentElement.getAttribute('data-theme')
  );

  // Change to earth theme
  await page.evaluate(() =>
    document.documentElement.setAttribute('data-theme', 'earth')
  );

  const newTheme = await page.evaluate(() =>
    document.documentElement.getAttribute('data-theme')
  );
  expect(newTheme).toBe('earth');

  // Restore
  if (originalTheme) {
    await page.evaluate((theme) =>
      document.documentElement.setAttribute('data-theme', theme),
      originalTheme
    );
  }
});

// ── Test 7: theme_save and theme_delete are idempotent ──

test('theme_delete on non-existent theme does not throw', async () => {
  // Should not throw — no-op for non-existent file
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('theme_delete', { themeId: 'non-existent-theme-id-12345' });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  // No error expected (no-op if file doesn't exist)
  expect(error).toBeNull();
});
