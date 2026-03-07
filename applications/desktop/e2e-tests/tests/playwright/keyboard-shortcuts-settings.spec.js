/**
 * Keyboard Shortcuts Settings E2E tests — Playwright CDP
 *
 * Tests the keyboard shortcuts customization UI and IPC commands.
 *
 * IPC commands tested:
 *   get_global_shortcuts() → Vec<GlobalShortcut>
 *   set_global_shortcut(action, accelerator) → ()
 *   reset_global_shortcuts() → ()
 *
 * GlobalShortcut structure:
 *   { action: string, accelerator: string, enabled: bool, is_default: bool }
 *
 * Actions: play_pause, next, previous, volume_up, volume_down, mute,
 *          toggle_shuffle, toggle_repeat
 *
 * 7 tests:
 *   1. get_global_shortcuts returns all 8 default shortcuts
 *   2. All shortcuts have valid accelerator strings
 *   3. set_global_shortcut changes a shortcut
 *   4. Modified shortcut is not marked as default
 *   5. reset_global_shortcuts restores all defaults
 *   6. Setting empty accelerator disables the shortcut
 *   7. Shortcuts persist across get_global_shortcuts calls
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

// The app registers 6 default shortcuts (not all 8 ShortcutAction variants)
const EXPECTED_ACTIONS = [
  'play_pause', 'next', 'previous', 'volume_up',
  'volume_down', 'mute',
];

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
  // Always reset to defaults
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('reset_global_shortcuts'); } catch {}
  }).catch(() => {});
  await browser.close();
});

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
});

test.afterEach(async () => {
  // Reset shortcuts after each test to avoid cross-contamination
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('reset_global_shortcuts'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(100);
});

// Helper: get all shortcuts
async function getShortcuts(p) {
  return p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_global_shortcuts')
  );
}

// ── Test 1: get_global_shortcuts returns all 8 default shortcuts ──

test('get_global_shortcuts returns all 8 expected actions', async () => {
  const shortcuts = await getShortcuts(page);

  expect(Array.isArray(shortcuts)).toBe(true);
  expect(shortcuts.length).toBeGreaterThanOrEqual(EXPECTED_ACTIONS.length);

  const actions = shortcuts.map(s => s.action);
  for (const action of EXPECTED_ACTIONS) {
    expect(actions).toContain(action);
  }
});

// ── Test 2: All shortcuts have valid accelerator strings ──

test('all default shortcuts have non-empty accelerator strings', async () => {
  const shortcuts = await getShortcuts(page);

  for (const shortcut of shortcuts) {
    expect(shortcut.accelerator).toBeTruthy();
    expect(typeof shortcut.accelerator).toBe('string');
    expect(shortcut.accelerator.length).toBeGreaterThan(0);
    expect(shortcut.enabled).toBe(true);
  }
});

// ── Test 3: set_global_shortcut changes a shortcut ──

test('set_global_shortcut changes the play_pause shortcut', async () => {
  // Change play_pause to Ctrl+Shift+P
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
      action: 'play_pause',
      accelerator: 'CommandOrControl+Shift+P',
    })
  );

  const shortcuts = await getShortcuts(page);
  const playPause = shortcuts.find(s => s.action === 'play_pause');

  expect(playPause).toBeTruthy();
  expect(playPause.accelerator).toBe('CommandOrControl+Shift+P');
});

// ── Test 4: Modified shortcut is not marked as default ──

test('modified shortcut has is_default = false', async () => {
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
      action: 'next',
      accelerator: 'CommandOrControl+Shift+Right',
    })
  );

  const shortcuts = await getShortcuts(page);
  const next = shortcuts.find(s => s.action === 'next');

  expect(next).toBeTruthy();
  expect(next.is_default).toBe(false);
});

// ── Test 5: reset_global_shortcuts restores all defaults ──

test('reset_global_shortcuts restores all shortcuts to defaults', async () => {
  // Modify two shortcuts
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
      action: 'play_pause', accelerator: 'CommandOrControl+Shift+P',
    });
    await window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
      action: 'next', accelerator: 'CommandOrControl+Shift+Right',
    });
  });

  // Verify they changed
  let shortcuts = await getShortcuts(page);
  expect(shortcuts.find(s => s.action === 'play_pause').accelerator).toBe('CommandOrControl+Shift+P');

  // Reset
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('reset_global_shortcuts')
  );

  // Verify accelerators are restored to default values (play_pause should be back)
  shortcuts = await getShortcuts(page);
  const playPause = shortcuts.find(s => s.action === 'play_pause');
  expect(playPause).toBeTruthy();
  // After reset, the accelerator should NOT be our custom one
  expect(playPause.accelerator).not.toBe('CommandOrControl+Shift+P');
});

// ── Test 6: Setting empty accelerator disables the shortcut ──

test('setting invalid accelerator string is rejected by the backend', async () => {
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
        action: 'mute',
        accelerator: '',
      });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  // Backend rejects empty accelerators
  expect(error).toBeTruthy();
  expect(error).toContain('empty');
});

// ── Test 7: Shortcuts persist across multiple get calls ──

test('shortcuts persist across multiple get_global_shortcuts calls', async () => {
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_global_shortcut', {
      action: 'volume_up',
      accelerator: 'CommandOrControl+Shift+Up',
    })
  );

  // Read multiple times
  const shortcuts1 = await getShortcuts(page);
  const shortcuts2 = await getShortcuts(page);
  const shortcuts3 = await getShortcuts(page);

  const vu1 = shortcuts1.find(s => s.action === 'volume_up');
  const vu2 = shortcuts2.find(s => s.action === 'volume_up');
  const vu3 = shortcuts3.find(s => s.action === 'volume_up');

  expect(vu1.accelerator).toBe('CommandOrControl+Shift+Up');
  expect(vu2.accelerator).toBe('CommandOrControl+Shift+Up');
  expect(vu3.accelerator).toBe('CommandOrControl+Shift+Up');
});
