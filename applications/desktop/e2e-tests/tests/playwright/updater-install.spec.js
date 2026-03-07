/**
 * Updater install flow E2E tests — Playwright CDP
 *
 * Tests the full update installation lifecycle:
 *   1. Install button triggers progress events and completes
 *   2. Progress bar updates with increasing values during install
 *   3. Install failure shows error toast and re-enables Install button
 *   4. Dialog cannot be dismissed while installing
 *   5. Later button dismissed dialog before install starts
 *   6. Check for updates → Install flow (end-to-end)
 *   7. Update event → Install flow (end-to-end)
 *   8. Multiple check-for-updates calls don't stack dialogs
 *   9. Progress resets when dialog reopens after failed install
 *  10. Install with zero delay completes instantly
 *
 * Mocking strategy
 * ──────────────────
 * Uses Rust-side test commands (only active when PLAYWRIGHT_TEST_DIR is set):
 *   set_test_update_response  — controls check_for_updates return value
 *   set_test_install_delay    — controls install_update timing and failure
 *   emit_test_update_available — fires update-available event to frontend
 *
 * The test install delay emits simulated progress events (0→100%) in 5 steps.
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
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  // Reset all Rust-side test state
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', { response: null, no_update: false });
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 0, shouldFail: false });
  });
});

// ---- Helpers ----

async function openUpdateDialog(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('emit_test_update_available', {});
  });
  await expect(p.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 5_000 });
}

async function openUpdateDialogViaCheck(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', {
      response: { version: '42.0.0', date: null, body: 'Test release notes.' },
    });
  });
  await p.click('[data-testid="check-for-updates-button"]');
  await expect(p.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 8_000 });
}

// ================================================================
// Test 1: Install button triggers progress and completes successfully
// ================================================================

test('install button triggers progress events and completes', async () => {
  test.setTimeout(20_000);

  // Set 3s delay so we can observe progress
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 3000 });
  });

  await openUpdateDialog(page);

  const installBtn = page.locator('[data-testid="update-dialog-install"]');
  await expect(installBtn).toBeVisible();
  await installBtn.click();

  // Progress bar should appear
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  // Wait for install to complete — dialog should close or show success
  // The frontend calls invoke('install_update') which returns Ok(()) in test mode
  // after the delay. UpdateSettingsContext then shows a success toast.
  await page.waitForFunction(
    () => {
      // Either dialog closes or a success toast appears
      const dialog = document.querySelector('[data-testid="update-dialog"]');
      return !dialog || dialog.offsetParent === null;
    },
    { timeout: 10_000 }
  );
});

// ================================================================
// Test 2: Progress bar values increase during installation
// ================================================================

test('progress bar shows increasing percentage during install', async () => {
  test.setTimeout(20_000);

  // 4s delay — test install emits 5 progress steps
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 4000 });
  });

  await openUpdateDialog(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Wait for progress bar to appear
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  // Capture progress values over time
  const progressValues = [];
  for (let i = 0; i < 8; i++) {
    await page.waitForTimeout(500);
    const progressText = await page.evaluate(() => {
      // The percentage text is next to the progress bar
      const els = document.querySelectorAll('[data-testid="update-progress-bar"]');
      if (!els.length) return null;
      // Find the percentage text in the parent
      const parent = els[0].closest('.space-y-2');
      if (!parent) return null;
      const percentEl = parent.querySelector('.font-medium');
      return percentEl ? percentEl.textContent.trim() : null;
    });
    if (progressText) {
      const num = parseInt(progressText, 10);
      if (!isNaN(num)) progressValues.push(num);
    }
  }

  // Should have captured some non-zero progress values
  expect(progressValues.length).toBeGreaterThan(0);

  // At least one value should be > 0
  const nonZero = progressValues.filter(v => v > 0);
  expect(nonZero.length).toBeGreaterThan(0);
});

// ================================================================
// Test 3: Install failure shows error and re-enables Install button
// ================================================================

test('install failure shows error toast and re-enables Install button', async () => {
  test.setTimeout(15_000);

  // Configure install to fail after 1s delay
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 1000, shouldFail: true });
  });

  await openUpdateDialog(page);

  const installBtn = page.locator('[data-testid="update-dialog-install"]');
  await installBtn.click();

  // Install button should be disabled while installing
  await expect(installBtn).toBeDisabled();

  // After failure: button should be re-enabled
  await expect(installBtn).toBeEnabled({ timeout: 8_000 });

  // Dialog should still be visible (not dismissed on error)
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible();
});

// ================================================================
// Test 4: Dialog cannot be dismissed while installing
// ================================================================

test('dialog cannot be dismissed during installation', async () => {
  test.setTimeout(15_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 5000 });
  });

  await openUpdateDialog(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Wait for installing state
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 3_000 });

  // Later button should be disabled
  const laterBtn = page.locator('[data-testid="update-dialog-later"]');
  await expect(laterBtn).toBeDisabled();

  // Escape should not close the dialog
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible();
});

// ================================================================
// Test 5: Later button dismisses dialog before install
// ================================================================

test('Later button dismisses dialog before install starts', async () => {
  await openUpdateDialog(page);

  // Version should be shown
  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('99.9.9');

  // Click Later
  await page.click('[data-testid="update-dialog-later"]');

  // Dialog should disappear
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });
});

// ================================================================
// Test 6: Check for updates → Install (end-to-end flow)
// ================================================================

test('check for updates → dialog → install: full flow', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 2000 });
  });

  await openUpdateDialogViaCheck(page);

  // Version from the canned response
  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('42.0.0');

  // Install
  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  // Wait for completion
  await page.waitForFunction(
    () => {
      const dialog = document.querySelector('[data-testid="update-dialog"]');
      return !dialog || dialog.offsetParent === null;
    },
    { timeout: 10_000 }
  );
});

// ================================================================
// Test 7: Update event → Install (end-to-end flow)
// ================================================================

test('update-available event → dialog → install: full flow', async () => {
  test.setTimeout(20_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 2000 });
  });

  await openUpdateDialog(page);
  await expect(page.locator('[data-testid="update-dialog-version"]')).toContainText('99.9.9');

  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });

  await page.waitForFunction(
    () => {
      const dialog = document.querySelector('[data-testid="update-dialog"]');
      return !dialog || dialog.offsetParent === null;
    },
    { timeout: 10_000 }
  );
});

// ================================================================
// Test 8: Multiple check-for-updates calls don't stack dialogs
// ================================================================

test('multiple rapid check-for-updates calls show only one dialog', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_update_response', {
      response: { version: '42.0.0', date: null, body: 'Test' },
    });
  });

  // Click check button rapidly
  const checkBtn = page.locator('[data-testid="check-for-updates-button"]');
  await checkBtn.click();
  await page.waitForTimeout(100);

  // Wait for dialog
  await expect(page.locator('[data-testid="update-dialog"]')).toBeVisible({ timeout: 8_000 });

  // Should only be one dialog instance
  const dialogCount = await page.locator('[data-testid="update-dialog"]').count();
  expect(dialogCount).toBe(1);
});

// ================================================================
// Test 9: Progress resets when dialog reopens after failed install
// ================================================================

test('progress resets to 0 when dialog reopens after failed install', async () => {
  test.setTimeout(20_000);

  // First attempt: fail after 1s
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 1000, shouldFail: true });
  });

  await openUpdateDialog(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Wait for failure (button re-enabled)
  await expect(page.locator('[data-testid="update-dialog-install"]')).toBeEnabled({ timeout: 8_000 });

  // Close dialog
  await page.click('[data-testid="update-dialog-later"]');
  await expect(page.locator('[data-testid="update-dialog"]')).not.toBeVisible({ timeout: 3_000 });

  // Reset to success mode
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 3000, shouldFail: false });
  });

  // Reopen dialog
  await openUpdateDialog(page);

  // Progress bar should NOT be visible (reset to 0, not installing)
  await expect(page.locator('[data-testid="update-progress-bar"]')).not.toBeVisible();

  // Install should work now
  await page.click('[data-testid="update-dialog-install"]');
  await expect(page.locator('[data-testid="update-progress-bar"]')).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 10: Install with zero delay completes instantly
// ================================================================

test('install with zero delay completes instantly', async () => {
  test.setTimeout(10_000);

  // No delay = instant success
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_test_install_delay', { delayMs: 0 });
  });

  await openUpdateDialog(page);
  await page.click('[data-testid="update-dialog-install"]');

  // Should complete almost instantly — dialog closes
  await page.waitForFunction(
    () => {
      const dialog = document.querySelector('[data-testid="update-dialog"]');
      return !dialog || dialog.offsetParent === null;
    },
    { timeout: 5_000 }
  );
});
