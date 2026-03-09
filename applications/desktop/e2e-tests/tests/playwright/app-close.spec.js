/**
 * App close — Playwright E2E regression test
 *
 * Verifies that clicking the window close button (WM_CLOSE) causes the
 * soul-player-desktop process to terminate cleanly within 5 seconds.
 *
 * This guards against:
 *  - Deadlock from calling window geometry APIs on a destroyed HWND
 *  - async task never running because the tokio runtime shut down first
 *  - app.exit() message being dropped by a partially-shut-down event loop
 *
 * The test sends CloseMainWindow() (identical to the user clicking X) via
 * PowerShell, then polls the PID until the OS reports the process as gone.
 *
 * IMPORTANT: This test terminates the app process. It MUST run as a standalone
 * suite so it does not break tests that run after it.
 *
 *   npx playwright test --config playwright.cdp.config.js tests/playwright/app-close.spec.js
 */

import { test, expect, chromium } from '@playwright/test';
import { execSync, execFileSync } from 'child_process';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- helpers ----

/** Returns true if the OS reports the PID as alive. */
function isProcessAlive(pid) {
  try {
    process.kill(pid, 0); // Signal 0 = probe only, no actual signal
    return true;
  } catch {
    return false;
  }
}

/** Polls until the process is gone or the deadline passes. Returns elapsed ms. */
async function waitForProcessExit(pid, timeoutMs) {
  const start = Date.now();
  const deadline = start + timeoutMs;
  while (Date.now() < deadline) {
    if (!isProcessAlive(pid)) return Date.now() - start;
    await new Promise(r => setTimeout(r, 100));
  }
  return -1; // Timed out
}

/** Send WM_CLOSE to a process via CloseMainWindow() — same as clicking X. */
function sendWindowClose(pid) {
  // CloseMainWindow() posts WM_CLOSE to the main window of the process,
  // which is identical to the user clicking the title-bar X button.
  // This triggers Tauri's CloseRequested event (not a force-kill).
  execSync(
    `powershell -NonInteractive -Command "$p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if ($p) { $p.CloseMainWindow() }"`,
    { timeout: 5_000 },
  );
}

// ---- test ----

let browser;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  const page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found — is the app running?');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 15_000 });
});

// afterAll is intentionally minimal — the process is expected to be dead.
test.afterAll(async () => {
  // browser.close() may throw since the CDP target is gone; ignore the error.
  try { await browser.close(); } catch { /* expected */ }
});

test('app process exits within 5 seconds of window close (WM_CLOSE)', async () => {
  test.setTimeout(20_000);

  const pid = parseInt(process.env.PLAYWRIGHT_APP_PID || '0', 10);
  if (!pid) {
    test.skip(true, 'PLAYWRIGHT_APP_PID not set — run via playwright-global-setup');
    return;
  }

  // Confirm process is alive before we try to close it.
  expect(isProcessAlive(pid), `Process ${pid} must be alive at test start`).toBe(true);

  const closeStart = Date.now();

  // Send WM_CLOSE — this triggers Tauri's CloseRequested event handler,
  // which should save window state and call std::process::exit(0).
  sendWindowClose(pid);

  // Wait up to 5 seconds for the process to disappear.
  const elapsed = await waitForProcessExit(pid, 5_000);

  if (elapsed === -1) {
    // Process still alive after timeout — force-kill for cleanup then fail.
    try { execFileSync('taskkill', ['/PID', String(pid), '/F']); } catch { /* ignore */ }
    throw new Error(
      `Process ${pid} did NOT exit within 5 seconds after WM_CLOSE. ` +
      `The close handler is hung (likely a deadlock or std::process::exit not reached).`
    );
  }

  console.log(`[app-close] Process ${pid} exited ${elapsed}ms after WM_CLOSE`);

  // Should exit quickly (well under 3 seconds — 2s timeout on DB write + overhead).
  expect(elapsed).toBeLessThan(5_000);
});
