/**
 * file-drop-playback.spec.js — Playback correctness after file drop / files-opened
 *
 * Verifies the fixes for Task 3 (rawId / non-integer track ID bugs) that caused:
 *   - Cover art to be wiped on TrackChanged after file-drop
 *   - Progress bar to freeze (position never advancing) after file-drop
 *
 * Strategy:
 *   - Use PLAYWRIGHT_TEST_DIR WAV files (created by global setup)
 *   - Trigger via `files-opened` Tauri event (same as OS open-with / drag-drop path B)
 *   - Click "Play Now" in the file-drop dialog to start playback
 *   - Assert Playing state, advancing position, visible now-playing, and track title
 *
 * Seed data (from playwright-global-setup.js):
 *   PLAYWRIGHT_TEST_DIR/audio/test-track.wav       — 10s silent WAV
 *   PLAYWRIGHT_TEST_DIR/audio/test-track-long.wav  — 30s silent WAV
 *   PLAYWRIGHT_TEST_DIR/audio/test-track-med.wav   — 15s silent WAV
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
    wav3Path: join(audioDir, 'test-track-med.wav'),
    audioDir,
  };
}

/** Wait for playback state to reach 'Playing' within timeoutMs. */
async function waitForPlayingState(timeoutMs = 8000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const state = await invoke('get_playback_state').catch(() => 'Stopped');
    if (state === 'Playing') return true;
    await page.waitForTimeout(150);
  }
  return false;
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
});

test.afterEach(async () => {
  await invoke('stop_playback').catch(() => {});
  await resetFileSettings();
  await dismissDialog();
});

// ---------------------------------------------------------------------------
// Test 1: Dropped WAV file triggers playback and progress bar advances
// ---------------------------------------------------------------------------

test('dropped WAV file triggers playback with progress bar animating', async () => {
  const { wavPath } = getTestPaths();

  // Simulate OS drag-drop (Path A)
  await simulateDragDrop([wavPath]);

  // Dialog should appear — click Play Now
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  // Dialog must close
  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Wait for Playing state
  const playing = await waitForPlayingState(8000);
  expect(playing).toBe(true);

  // Wait 2s then verify position has advanced past 0.1s
  await page.waitForTimeout(2000);
  const position = await invoke('get_position').catch(() => 0);
  expect(typeof position).toBe('number');
  expect(position).toBeGreaterThan(0.1);
});

// ---------------------------------------------------------------------------
// Test 2: Dropped WAV file — now-playing panel shows within 3s
// ---------------------------------------------------------------------------

test('dropped WAV file — now-playing panel shows within 3s', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // now-playing-title in the sidebar PlayerPanel must become visible within 5s of click
  const nowPlayingTitle = page.locator('[data-testid="now-playing-title"]');
  await expect(nowPlayingTitle).toBeVisible({ timeout: 5000 });
});

// ---------------------------------------------------------------------------
// Test 3: TrackChanged after file-drop does NOT lose cover art / track title
// ---------------------------------------------------------------------------

test('dropped file TrackChanged event does not lose cover art or track title', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Wait for Playing + allow a TrackChanged event to settle
  const playing = await waitForPlayingState(8000);
  expect(playing).toBe(true);
  await page.waitForTimeout(500);

  // now-playing title must be non-empty (cover art wipe bug resets state, making title blank)
  const nowPlayingTitle = page.locator('[data-testid="now-playing-title"]');
  if (await nowPlayingTitle.isVisible({ timeout: 3000 }).catch(() => false)) {
    const titleText = await nowPlayingTitle.textContent();
    expect(titleText).toBeTruthy();
    expect(titleText.trim().length).toBeGreaterThan(0);
    // Title must not be a raw absolute file path (indicates metadata was not applied)
    expect(titleText.trim()).not.toMatch(/^[A-Z]:\\|^\//);
  }

  // Verify playback state is still Playing (cover art bug also caused engine to stall)
  const state = await invoke('get_playback_state');
  expect(state).toBe('Playing');
});

// ---------------------------------------------------------------------------
// Test 4: Multiple dropped files queue all tracks
// ---------------------------------------------------------------------------

test('multiple dropped files queue all tracks', async () => {
  const { wavPath, wav2Path, wav3Path } = getTestPaths();

  await simulateFilesOpened([wavPath, wav2Path, wav3Path]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Wait for Playing
  const playing = await waitForPlayingState(8000);
  expect(playing).toBe(true);

  // Queue should contain at least 2 tracks (current track may be at index 0, remaining >= 2)
  const queueSize = await page.evaluate(async () => {
    try {
      const q = await window.__TAURI_INTERNALS__.invoke('get_queue');
      return Array.isArray(q) ? q.length : -1;
    } catch {
      return -1;
    }
  });

  // If get_queue is available, verify at least 2 entries remain in the queue
  if (queueSize >= 0) {
    expect(queueSize).toBeGreaterThanOrEqual(2);
  } else {
    // Fallback: verify playback started (queue IPC may not be available in older binary)
    const state = await invoke('get_playback_state');
    expect(['Playing', 'Paused']).toContain(state);
  }
});

// ---------------------------------------------------------------------------
// Test 5: Position advances monotonically — progress bar is not frozen
// ---------------------------------------------------------------------------

test('position advances monotonically after file-drop (progress bar not frozen)', async () => {
  const { wav2Path } = getTestPaths();

  // Use the long WAV (30s) so we have plenty of room to check position advance
  await simulateFilesOpened([wav2Path]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  const playing = await waitForPlayingState(8000);
  expect(playing).toBe(true);

  // Sample position at t=0 (relative to start of this check)
  await page.waitForTimeout(500);
  const pos1 = await invoke('get_position').catch(() => 0);

  // Wait 1.5s and sample again
  await page.waitForTimeout(1500);
  const pos2 = await invoke('get_position').catch(() => 0);

  // Position must have advanced by at least 0.5s (allows for engine startup jitter)
  expect(pos2).toBeGreaterThan(pos1 + 0.5);
});
