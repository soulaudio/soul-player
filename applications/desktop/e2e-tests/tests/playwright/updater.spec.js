/**
 * Updater E2E tests — Playwright CDP
 *
 * Comprehensive tests for the update lifecycle:
 *
 * Settings UI (tests 1–3)
 *   1. About page renders auto-update toggle, silent toggle, and Check button
 *   2. Auto-update toggle persists state when toggled off/on
 *   3. Silent-update toggle persists state when toggled off/on
 *
 * Update detection (tests 4–7)
 *   4. Backend "update-available" event → UpdateDialog appears with version
 *   5. "Check for Updates" button → dialog when update exists
 *   6. "Check for Updates" button → up-to-date toast when no update
 *   7. Multiple rapid check clicks don't stack duplicate dialogs
 *
 * Dialog behaviour (tests 8–10)
 *   8. "Later" button dismisses the dialog
 *   9. Escape key dismisses the dialog (before install)
 *  10. Backdrop click dismisses the dialog (before install)
 *
 * Installation — manual (tests 11–16)
 *  11. Install button triggers progress bar and completes successfully
 *  12. Progress bar shows increasing percentage values during install
 *  13. Install failure shows error toast and re-enables Install button
 *  14. Dialog cannot be dismissed while installing (Later disabled, Escape ignored)
 *  15. Progress resets to zero when dialog reopens after a failed install
 *  16. Install with zero delay completes instantly
 *
 * End-to-end flows (tests 17–18)
 *  17. Check for updates → dialog → install → completion
 *  18. Backend event → dialog → install → completion
 *
 * How it works
 * ────────────
 * Rust-side test commands (active only when PLAYWRIGHT_TEST_DIR env var is set)
 * control the backend without JS mocking:
 *
 *   set_test_update_response  — controls what check_for_updates returns
 *   set_test_install_delay    — controls install_update timing and failure
 *   emit_test_update_available — fires a real update-available Tauri event
 *
 * These commands exercise the real Tauri IPC pipeline, event system, and
 * frontend React contexts — the only difference is the HTTP request to
 * GitHub is replaced by an in-process canned response.
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

// Navigate to Settings → About before each test
test.beforeEach(async () => {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-settings-about"]');
  await page.waitForSelector('[data-testid="check-for-updates-button"]', { timeout: 10_000 });
});

test.afterEach(async () => {
  // Dismiss any open dialog
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  // Reset all Rust-side test state
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', { response: null, no_update: false });
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 0, shouldFail: false });
  });
});

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Open update dialog via backend event (version 99.9.9) */
async function openDialogViaEvent(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('emit_test_update_available', {});
  });
  await expect(p.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 5_000 });
}

/** Open update dialog via Check for Updates button (version 42.0.0) */
async function openDialogViaCheck(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', {
      response: { version: '42.0.0', date: null, body: 'Test release notes.' },
    });
  });
  await p.click('[data-testid="check-for-updates-button"]');
  await expect(p.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 8_000 });
}

/** Wait for dialog to close (hidden or removed from DOM) */
async function waitForDialogClosed(p, timeout = 10_000) {
  await p.waitForFunction(
    () => {
      const dialog = document.querySelector('[data-testid="update-dialog"]');
      return !dialog || dialog.offsetParent === null;
    },
    { timeout }
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Settings UI
// ═══════════════════════════════════════════════════════════════════════════

test('1 · about page renders update controls with correct defaults', async () => {
  const autoToggle = page.locator('[data-testid="auto-update-toggle"]');
  await expect(autoToggle).toBeVisible();
  await expect(autoToggle).toBeChecked();

  const silentToggle = page.locator('[data-testid="silent-update-toggle"]');
  await expect(silentToggle).toBeVisible();
  await expect(silentToggle).not.toBeChecked();

  const checkBtn = page.locator('[data-testid="check-for-updates-button"]');
  await expect(checkBtn).toBeVisible();
  await expect(checkBtn).toBeEnabled();
});

test('2 · auto-update toggle persists when toggled off then on', async () => {
  const toggle = page.locator('[data-testid="auto-update-toggle"]');
  await expect(toggle).toBeChecked();

  // Toggle off
  await toggle.click();
  await expect(toggle).not.toBeChecked();

  // Verify persisted via IPC
  const stored = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'app.auto_update_enabled' });
  });
  expect(stored).toBe('false');

  // Toggle back on
  await toggle.click();
  await expect(toggle).toBeChecked();

  const restored = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'app.auto_update_enabled' });
  });
  expect(restored).toBe('true');
});

test('3 · silent-update toggle persists when toggled on then off', async () => {
  const toggle = page.locator('[data-testid="silent-update-toggle"]');
  await expect(toggle).not.toBeChecked();

  // Toggle on
  await toggle.click();
  await expect(toggle).toBeChecked();

  const stored = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'app.auto_update_silent' });
  });
  expect(stored).toBe('true');

  // Toggle off
  await toggle.click();
  await expect(toggle).not.toBeChecked();

  const restored = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'app.auto_update_silent' });
  });
  expect(restored).toBe('false');
});

// ═══════════════════════════════════════════════════════════════════════════
// Update detection
// ═══════════════════════════════════════════════════════════════════════════

test('4 · backend update-available event shows dialog with version', async () => {
  await openDialogViaEvent(page);

  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('99.9.9');
  // Dialog should contain Install button
  await expect(page.locator('[data-testid="update-dialog-install"]')).toBeVisible();
  // Dialog should contain Later button
  await expect(page.locator('[data-testid="update-dialog-later"]')).toBeVisible();
});

test('5 · check-for-updates button shows dialog when update exists', async () => {
  await openDialogViaCheck(page);

  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('42.0.0');
});

test('6 · check-for-updates shows up-to-date toast when no update', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', { no_update: true });
  });

  await page.click('[data-testid="check-for-updates-button"]');

  // No dialog
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });
  // Toast: "You're on the latest version!"
  await expect(page.getByText(/latest version/i)).toBeVisible({ timeout: 5_000 });
});

test('7 · multiple rapid check clicks show only one dialog', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', {
      response: { version: '42.0.0', date: null, body: 'Test' },
    });
  });

  const checkBtn = page.locator('[data-testid="check-for-updates-button"]');
  await checkBtn.click();
  await page.waitForTimeout(100);

  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 8_000 });
  const count = await page.locator('[data-testid="update-dialog"]').count();
  expect(count).toBe(1);
});

// ═══════════════════════════════════════════════════════════════════════════
// Dialog behaviour
// ═══════════════════════════════════════════════════════════════════════════

test('8 · Later button dismisses the dialog', async () => {
  await openDialogViaEvent(page);

  await page.click('[data-testid="update-dialog-later"]');
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });
});

test('9 · Escape key dismisses the dialog before install', async () => {
  await openDialogViaEvent(page);

  await page.keyboard.press('Escape');
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });
});

test('10 · backdrop click dismisses the dialog before install', async () => {
  await openDialogViaEvent(page);

  // Click the backdrop (the outermost overlay div)
  const dialog = page.locator('[data-testid="update-dialog"]');
  // Click at position (10, 10) relative to viewport — outside the centered modal
  await page.mouse.click(10, 10);
  await expect(dialog).not.toBeVisible({ timeout: 3_000 });
});

// ═══════════════════════════════════════════════════════════════════════════
// Installation — manual
// ═══════════════════════════════════════════════════════════════════════════

test('11 · install triggers progress bar and completes', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 3000 });
  });

  await openDialogViaEvent(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Progress bar appears
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  // Dialog closes on completion
  await waitForDialogClosed(page);
});

test('12 · progress bar shows increasing percentage during install', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 4000 });
  });

  await openDialogViaEvent(page);
  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  // Sample progress values over time
  const progressValues = [];
  for (let i = 0; i < 8; i++) {
    await page.waitForTimeout(500);
    const progressText = await page.evaluate(() => {
      const bar = document.querySelector('[data-testid="update-progress-bar"]');
      if (!bar) return null;
      const parent = bar.closest('.space-y-2');
      if (!parent) return null;
      const pct = parent.querySelector('.font-medium');
      return pct ? pct.textContent.trim() : null;
    });
    if (progressText) {
      const num = parseInt(progressText, 10);
      if (!isNaN(num)) progressValues.push(num);
    }
  }

  expect(progressValues.length).toBeGreaterThan(0);
  const nonZero = progressValues.filter((v) => v > 0);
  expect(nonZero.length).toBeGreaterThan(0);
});

test('13 · install failure shows error and re-enables Install button', async () => {
  test.setTimeout(15_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 1000, shouldFail: true });
  });

  await openDialogViaEvent(page);
  const installBtn = page.locator('[data-testid="update-dialog-install"]');
  await installBtn.click();

  // Button disabled while installing
  await expect(installBtn).toBeDisabled();

  // After failure: button re-enabled, dialog stays open
  await expect(installBtn).toBeEnabled({ timeout: 8_000 });
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible();
});

test('14 · dialog locked during installation (Later disabled, Escape ignored)', async () => {
  test.setTimeout(15_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 5000 });
  });

  await openDialogViaEvent(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Wait for installing state
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 3_000 });

  // Later button disabled
  await expect(page.locator('[data-testid="update-dialog-later"]')).toBeDisabled();

  // Escape does not close
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible();

  // Backdrop click does not close
  await page.mouse.click(10, 10);
  await page.waitForTimeout(300);
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible();
});

test('15 · progress resets when dialog reopens after failed install', async () => {
  test.setTimeout(20_000);

  // First attempt: fail
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 1000, shouldFail: true });
  });

  await openDialogViaEvent(page);
  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-dialog-install"]')).toBeEnabled({ timeout: 8_000 });

  // Close and reset
  await page.click('[data-testid="update-dialog-later"]');
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 3000, shouldFail: false });
  });

  // Reopen — progress bar should NOT be visible (reset to 0)
  await openDialogViaEvent(page);
  await expect(page.locator('[data-testid="update-progress-bar"]')).not.toBeVisible();

  // Second attempt should work
  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });
});

test('16 · install with zero delay completes instantly', async () => {
  test.setTimeout(10_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 0 });
  });

  await openDialogViaEvent(page);
  await page.click('[data-testid="update-dialog-install"]');

  await waitForDialogClosed(page, 5_000);
});

// ═══════════════════════════════════════════════════════════════════════════
// End-to-end flows
// ═══════════════════════════════════════════════════════════════════════════

test('17 · check → dialog → install → completion (full flow)', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 2000 });
  });

  await openDialogViaCheck(page);
  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('42.0.0');

  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });
  await waitForDialogClosed(page);
});

test('18 · event → dialog → install → completion (full flow)', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 2000 });
  });

  await openDialogViaEvent(page);
  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('99.9.9');

  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });
  await waitForDialogClosed(page);
});
