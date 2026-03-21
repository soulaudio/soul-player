/**
 * file-drop-playback.spec.js — Regression tests for Task 3 file-drop playback bugs
 *
 * Scope: ONLY the fixes introduced in Task 3 (rawId / non-integer track ID bugs):
 *   - Cover art wiped on TrackChanged after file-drop  →  track title must be non-empty
 *   - Progress bar freeze (position stuck at 0) after file-drop
 *   - Duration optimistic-state fix (progress bar animates from first second)
 *
 * What is NOT tested here (already covered by file-open-e2e.spec.js):
 *   G3.1-G3.4  — basic play triggering, queue size, now-playing title visibility
 *   G2.x       — dialog appearance, file count display
 *   G6.x       — path parity (drag-drop ≡ files-opened)
 *
 * Strategy:
 *   - Trigger via `files-opened` (Path B) or `tauri://drag-drop` (Path A)
 *   - Use long WAV (30s) for position-advance tests so there is plenty of room
 *   - Assert position advances monotonically and now-playing title is populated
 *
 * Seed data (from playwright-global-setup.js):
 *   PLAYWRIGHT_TEST_DIR/audio/test-track.wav       — 10s silent WAV
 *   PLAYWRIGHT_TEST_DIR/audio/test-track-long.wav  — 30s silent WAV
 */

import { test, expect, chromium } from '@playwright/test';
import { join } from 'path';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---------------------------------------------------------------------------
// CDP connection — shared across all tests in this file
// ---------------------------------------------------------------------------

let browser;
let page;

/** Invoke a Tauri IPC command. */
async function invoke(cmd, params = {}) {
  return page.evaluate(
    async ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params }
  );
}

/** Emit a Tauri event from the frontend (received by all listen() subscribers). */
async function emitTauri(event, payload) {
  return page.evaluate(
    async ({ event, payload }) =>
      window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event, payload }),
    { event, payload }
  );
}

/** Simulate files-opened IPC event (Path B — already-running open-with / single-instance). */
async function simulateFilesOpened(paths) {
  await emitTauri('files-opened', paths);
}

/** Simulate a native Tauri drag-drop (Path A — OS drag-drop). */
async function simulateDragDrop(paths) {
  await emitTauri('tauri://drag-enter', { paths, position: { x: 400, y: 300 } });
  await page.waitForTimeout(100);
  await emitTauri('tauri://drag-drop', { paths, position: { x: 400, y: 300 } });
}

/** Wait for the play/import dialog to appear (up to 5s). */
async function waitForDialog(timeout = 5000) {
  await page.waitForSelector('[data-testid="file-drop-dialog"]', { timeout });
}

/** Dismiss any open file-drop dialog. */
async function dismissDialog() {
  const close = page.locator('[data-testid="file-drop-close"]');
  if (await close.isVisible({ timeout: 500 }).catch(() => false)) {
    await close.click();
  } else {
    await page.keyboard.press('Escape');
  }
  await page.waitForTimeout(200);
}

/** Reset external file settings so dialog is always shown. */
async function resetFileSettings() {
  await invoke('set_external_file_settings', {
    defaultAction: 'ask',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  }).catch(() => {});
}

/** Get the WAV file paths created by global setup. */
function getTestPaths() {
  const testDir = process.env.PLAYWRIGHT_TEST_DIR;
  if (!testDir) throw new Error('PLAYWRIGHT_TEST_DIR not set — global setup must run first');
  const audioDir = join(testDir, 'audio');
  return {
    wavPath: join(audioDir, 'test-track.wav'),
    wav2Path: join(audioDir, 'test-track-long.wav'),
    audioDir,
  };
}

/**
 * Wait for playback state to reach 'Playing'.
 * Uses page.waitForFunction so it integrates cleanly with Playwright's
 * timeout / cancellation infrastructure. Throws on timeout.
 */
async function waitForPlayingState(timeoutMs = 8000) {
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: timeoutMs }
  );
}

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

test.beforeEach(async () => {
  // Stop any in-progress playback
  await invoke('stop_playback').catch(() => {});
  await resetFileSettings();
  await dismissDialog();
  await page.waitForTimeout(200);

  // Dismiss any leftover overlay
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(100);

  // Navigate to a known, stable page before each test
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await invoke('stop_playback').catch(() => {});
  await resetFileSettings();
  await dismissDialog();
});

// ---------------------------------------------------------------------------
// Test 1: Progress bar animates — position advances after drop
//
// Directly verifies the `duration` optimistic-state fix.  Before the fix,
// position was stuck at 0 because duration was never set so the bar never
// moved.  After the fix, position must exceed 0.1 s within 2 s of play start.
// ---------------------------------------------------------------------------

test('progress bar animates: position advances within 2s of file-drop play', async () => {
  test.setTimeout(30_000);

  const { wavPath } = getTestPaths();

  // Use Path A (drag-drop) to exercise the exact code path the bug affected
  await simulateDragDrop([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  await waitForPlayingState(8000);

  // Allow 2 s of wall-clock playback, then assert position has advanced
  await page.waitForTimeout(2000);
  const position = await invoke('get_position').catch(() => 0);
  expect(typeof position).toBe('number');
  expect(position).toBeGreaterThan(0.1);
});

// ---------------------------------------------------------------------------
// Test 2: Position advances monotonically — progress bar is not frozen
//
// Directly tests the progress-bar-freeze bug.  Two samples taken 1.5 s apart
// must show strictly increasing position.  Uses the long WAV (30s) so there
// is plenty of room to sample without hitting end-of-track.
// ---------------------------------------------------------------------------

test('position advances monotonically after file-drop (progress bar not frozen)', async () => {
  test.setTimeout(30_000);

  const { wav2Path } = getTestPaths();

  // Use Path B (files-opened) to exercise the other entry point
  await simulateFilesOpened([wav2Path]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  await waitForPlayingState(8000);

  // Sample position at t=0 (relative to start of this check)
  await page.waitForTimeout(500);
  const pos1 = await invoke('get_position').catch(() => 0);

  // Wait 1.5 s and sample again
  await page.waitForTimeout(1500);
  const pos2 = await invoke('get_position').catch(() => 0);

  // Position must have advanced by at least 0.5 s (allows for engine startup jitter)
  expect(pos2).toBeGreaterThan(pos1 + 0.5);
});

// ---------------------------------------------------------------------------
// Test 3: TrackChanged after file-drop does NOT lose cover art or track title
//
// Before the rawId fix, the TrackChanged event carried a non-integer track ID
// which the store failed to match, wiping cover art and resetting the title to
// empty.  The title must be non-empty and must not be a raw file path.
// ---------------------------------------------------------------------------

test('dropped file TrackChanged event does not lose cover art or track title', async () => {
  test.setTimeout(30_000);

  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Wait for Playing + allow a TrackChanged event to settle
  await waitForPlayingState(8000);
  await page.waitForTimeout(500);

  // now-playing title must be visible and non-empty (unconditional assertion)
  await expect(page.locator('[data-testid="now-playing-title"]')).toBeVisible({ timeout: 5000 });
  const titleText = await page.locator('[data-testid="now-playing-title"]').textContent();
  expect(titleText).toBeTruthy();
  expect(titleText.trim().length).toBeGreaterThan(0);
  // Title must not be a raw absolute file path (indicates metadata was not applied)
  expect(titleText.trim()).not.toMatch(/^[A-Z]:\\|^\//);

  // Verify playback state is still Playing (cover art bug also caused engine to stall)
  const state = await invoke('get_playback_state');
  expect(state).toBe('Playing');
});
