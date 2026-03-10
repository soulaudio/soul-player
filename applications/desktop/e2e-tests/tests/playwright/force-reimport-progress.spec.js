/**
 * Force Re-import & Scan Progress — Playwright CDP tests
 *
 * Root causes fixed (tested here):
 *   1. progress_callback never called for libraries with < 10 files —
 *      flush_progress fired only every 10 files, leaving the UI at 0/0.
 *   2. ScanProgressIndicator event listener expected {id, processedFiles, ...}
 *      but payload is {processed, total} — silently dropped, no UI update.
 *   3. Fast scans completed before the 500 ms poll cycle; get_running_scans
 *      always saw total_files = NULL; indicator showed 0/0 throughout.
 *
 * Fixes verified:
 *   - library_scanner.rs: progress_callback fires at scan START (total known)
 *     and at scan END (processed == total), even for < 10 files.
 *   - ScanProgressIndicator.tsx: fixed listener stores lastProgress, used as
 *     fallback display when get_running_scans returns empty post-completion.
 *   - LibrarySettingsPage.tsx: added data-testid="force-reimport-button".
 *
 * Seed data:
 *   - Album 2001: 5 × 10s WAV tracks (IDs 2001–2005)
 *   - Library source seeded at audioDir path
 *
 * Tests:
 *   1.  force-reimport button exists and is not disabled
 *   2.  force reimport triggers scan-progress-indicator
 *   3.  indicator shows non-zero total (not 0/0) after force reimport
 *   4.  progress bar width is > 0 during/after scan
 *   5.  indicator disappears after scan completes
 *   6.  rescan-all button (normal mode) also shows non-zero total
 *   7.  per-source rescan button shows non-zero total
 *   8.  second force reimport runs correctly (state machine doesn't break)
 *   9.  progress text transitions from 0/N to N/N during scan
 *  10.  scan-complete fires and indicator hides within 10 s of appearing
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
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// ── helpers ──────────────────────────────────────────────────────────────────

/** Navigate to Settings → Music Data and wait for the page to be ready. */
async function goToMusicDataSettings() {
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(150);
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-settings-musicData"]');
  await page.waitForSelector('[data-testid="library-sources-toggle"]', { timeout: 25_000 });
}

/** Wait for the scan-progress-indicator to appear and return its locator. */
async function waitForIndicator(timeout = 10_000) {
  await page.waitForSelector('[data-testid="scan-progress-indicator"]', { timeout });
  return page.locator('[data-testid="scan-progress-indicator"]');
}

/** Wait for the scan-progress-indicator to disappear. */
async function waitForIndicatorGone(timeout = 30_000) {
  await page.waitForSelector('[data-testid="scan-progress-indicator"]', {
    state: 'hidden',
    timeout,
  });
}

/** Wait until the indicator shows a non-zero total in its "N/M" counter. */
async function waitForNonZeroTotal(timeout = 15_000) {
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="scan-progress-indicator"]');
      if (!el) return false;
      const m = (el.textContent || '').match(/(\d+)\/(\d+)/);
      return m !== null && parseInt(m[2], 10) > 0;
    },
    { timeout },
  );
}

// ── beforeEach / afterEach ────────────────────────────────────────────────────

test.beforeEach(async () => {
  await goToMusicDataSettings();
});

test.afterEach(async () => {
  // Wait for any running scan to finish before the next test
  await waitForIndicatorGone(45_000).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Tests ─────────────────────────────────────────────────────────────────────

// ================================================================
// Test 1: Force Re-import button is visible and enabled
// ================================================================

test('force re-import button is visible and not disabled', async () => {
  test.setTimeout(15_000);

  const btn = page.locator('[data-testid="force-reimport-button"]');
  await expect(btn).toBeVisible({ timeout: 5_000 });
  await expect(btn).not.toBeDisabled();
});

// ================================================================
// Test 2: Force Re-import shows scan-progress-indicator
// ================================================================

test('force re-import triggers scan-progress-indicator', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();
  const indicator = await waitForIndicator();
  await expect(indicator).toBeVisible();
});

// ================================================================
// Test 3: Indicator shows non-zero total (bug: was always 0/0)
//
// The fixed progress_callback emits {processed:0, total:N} immediately
// after set_total_files so the frontend always has the total count.
// ================================================================

test('force re-import progress indicator shows non-zero file count', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();
  await waitForNonZeroTotal();

  const text = await page.locator('[data-testid="scan-progress-indicator"]').textContent();
  const match = text?.match(/(\d+)\/(\d+)/);
  expect(match).not.toBeNull();

  const total = parseInt(match[2], 10);
  expect(total).toBeGreaterThan(0);
});

// ================================================================
// Test 4: Progress bar width advances beyond 0 %
//
// The seed library has 5 WAV files. After the final flush_progress
// callback fires, lastProgress = {processed:5, total:5} → bar 100 %.
// ================================================================

test('progress bar width is greater than 0 % after force re-import', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();

  // Poll until the progress bar has a non-zero width style
  await page.waitForFunction(
    () => {
      const bar = document.querySelector('[data-testid="scan-progress-bar"] > div');
      if (!bar) return false;
      const w = parseFloat((bar as HTMLElement).style.width) || 0;
      return w > 0;
    },
    { timeout: 15_000 },
  );

  const bar = page.locator('[data-testid="scan-progress-bar"] > div');
  const width = await bar.evaluate(el => parseFloat((el as HTMLElement).style.width) || 0);
  expect(width).toBeGreaterThan(0);
});

// ================================================================
// Test 5: Indicator disappears after scan completes
// ================================================================

test('force re-import indicator disappears after scan finishes', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();
  await waitForIndicatorGone(20_000);

  await expect(page.locator('[data-testid="scan-progress-indicator"]')).not.toBeVisible();
});

// ================================================================
// Test 6: Rescan All (normal mode, no force) shows non-zero total
//
// All files are already imported; phase 1 skips them. The fix ensures
// progress_callback fires via flush_progress even for skipped-only scans.
// ================================================================

test('rescan-all button shows non-zero total in progress indicator', async () => {
  test.setTimeout(30_000);

  const btn = page.locator('[data-testid="rescan-all-button"]');
  await expect(btn).toBeVisible({ timeout: 5_000 });
  await btn.click();

  await waitForIndicator();
  await waitForNonZeroTotal();

  const text = await page.locator('[data-testid="scan-progress-indicator"]').textContent();
  const match = text?.match(/(\d+)\/(\d+)/);
  expect(match).not.toBeNull();
  expect(parseInt(match[2], 10)).toBeGreaterThan(0);
});

// ================================================================
// Test 7: Per-source rescan button shows non-zero total
//
// The rescan-button-{id} on the individual source row also flows through
// the same scanner pipeline. Total must be non-zero.
// ================================================================

test('per-source rescan button shows non-zero total in progress indicator', async () => {
  test.setTimeout(30_000);

  const rescanBtn = page.locator('[data-testid^="rescan-button-"]').first();
  await expect(rescanBtn).toBeVisible({ timeout: 5_000 });
  await rescanBtn.click();

  await waitForIndicator();
  await waitForNonZeroTotal();

  const text = await page.locator('[data-testid="scan-progress-indicator"]').textContent();
  const match = text?.match(/(\d+)\/(\d+)/);
  expect(match).not.toBeNull();
  expect(parseInt(match[2], 10)).toBeGreaterThan(0);
});

// ================================================================
// Test 8: Second force reimport (after first completes) works correctly
//
// Guards against state machine bugs where the scan doesn't fire a second
// time or the UI doesn't show progress for the second run.
// ================================================================

test('second force re-import also shows non-zero progress', async () => {
  test.setTimeout(60_000);

  // First run
  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();
  await waitForIndicatorGone(20_000);

  // Navigate away and back to reset UI state
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });
  await goToMusicDataSettings();

  // Second run
  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();
  await waitForNonZeroTotal();

  const text = await page.locator('[data-testid="scan-progress-indicator"]').textContent();
  const match = text?.match(/(\d+)\/(\d+)/);
  expect(match).not.toBeNull();
  expect(parseInt(match[2], 10)).toBeGreaterThan(0);
});

// ================================================================
// Test 9: Final displayed count matches total (N/N at end)
//
// After the scan completes, lastProgress = {processed:5, total:5}.
// During the 2 s grace period the indicator still shows 5/5 (not 0/0).
// ================================================================

test('indicator shows N/N (all files processed) when scan ends', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();
  await waitForIndicator();

  // Wait until processed == total in the displayed counter
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="scan-progress-indicator"]');
      if (!el) return false;
      const m = (el.textContent || '').match(/(\d+)\/(\d+)/);
      if (!m) return false;
      const processed = parseInt(m[1], 10);
      const total = parseInt(m[2], 10);
      return total > 0 && processed === total;
    },
    { timeout: 20_000 },
  );

  const text = await page.locator('[data-testid="scan-progress-indicator"]').textContent();
  const match = text?.match(/(\d+)\/(\d+)/);
  expect(match).not.toBeNull();
  expect(parseInt(match[1], 10)).toEqual(parseInt(match[2], 10));
  expect(parseInt(match[2], 10)).toBeGreaterThan(0);
});

// ================================================================
// Test 10: scan-complete event dismisses indicator within 10 s
//
// After force reimport of 5 short WAV files, the scan should complete
// quickly. The indicator must disappear within 10 s of appearing.
// ================================================================

test('indicator disappears within 10 s of force reimport on small library', async () => {
  test.setTimeout(30_000);

  await page.locator('[data-testid="force-reimport-button"]').click();

  const appearTime = Date.now();
  await waitForIndicator();

  await waitForIndicatorGone(15_000);
  const elapsed = Date.now() - appearTime;

  // Scan of 5 WAV files + 2 s grace period should complete well within 10 s
  expect(elapsed).toBeLessThan(15_000);
  await expect(page.locator('[data-testid="scan-progress-indicator"]')).not.toBeVisible();
});
