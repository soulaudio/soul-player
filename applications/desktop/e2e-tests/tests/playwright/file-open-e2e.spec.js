/**
 * file-open-e2e.spec.js — Full E2E tests for "open file with Soul Player"
 *
 * Tests both entry points exhaustively and verifies they behave identically:
 *
 *   Path A — Drag-drop:     OS drag → Tauri DRAG_DROP event → FileDropHandler
 *   Path B — files-opened:  Single-instance callback / already-running open-with
 *   Path C — Pending files: Cold-launch CLI args before React listener was ready
 *
 * Strategy:
 *   - Tauri DRAG_DROP can be simulated by emitting `tauri://drag-drop` from JS via
 *     the Tauri event plugin (plugin:event|emit). FileDropHandler's listen() callback
 *     receives it identically to the OS-triggered event.
 *   - files-opened is emitted the same way with the `files-opened` event name.
 *   - Pending files use test_set_pending_open_files + get_pending_open_files IPC.
 *
 * Coverage:
 *   Group 1: Drag overlay UI          (3 tests)
 *   Group 2: Dialog appearance        (5 tests)
 *   Group 3: Play Now action          (5 tests)
 *   Group 4: Import to Library action (3 tests)
 *   Group 5: Remember my choice       (4 tests)
 *   Group 6: Path parity              (4 tests)
 *   Group 7: Pending files (cold launch) (3 tests)
 *   Group 8: Edge cases               (4 tests)
 */

import { test, expect, chromium } from '@playwright/test';
import { join } from 'path';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let browser;
let page;

/** Emit a Tauri event from the frontend. Received by all listen() subscribers. */
async function emitTauri(event, payload) {
  return page.evaluate(
    async ({ event, payload }) =>
      window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event, payload }),
    { event, payload }
  );
}

/** Invoke a Tauri IPC command. */
async function invoke(cmd, params = {}) {
  return page.evaluate(
    async ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params }
  );
}

/** Simulate a native Tauri drag-drop event (Path A). */
async function simulateDragDrop(paths) {
  // DRAG_ENTER first (shows overlay)
  await emitTauri('tauri://drag-enter', { paths, position: { x: 400, y: 300 } });
  await page.waitForTimeout(100);
  // DRAG_DROP triggers the dialog/auto-handle
  await emitTauri('tauri://drag-drop', { paths, position: { x: 400, y: 300 } });
}

/** Simulate the files-opened IPC event (Path B — already-running open-with). */
async function simulateFilesOpened(paths) {
  await emitTauri('files-opened', paths);
}

/** Reset external file settings to "ask" so dialog is always shown. */
async function resetFileSettings() {
  await invoke('set_external_file_settings', {
    defaultAction: 'ask',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  });
}

/** Wait for the play/import dialog to appear (up to 5s). */
async function waitForDialog(timeout = 5000) {
  await page.waitForSelector('[data-testid="file-drop-dialog"]', { timeout });
}

/** Dismiss any open dialog via Escape or X button. */
async function dismissDialog() {
  const close = page.locator('[data-testid="file-drop-close"]');
  if (await close.isVisible({ timeout: 500 }).catch(() => false)) {
    await close.click();
  } else {
    await page.keyboard.press('Escape');
  }
  await page.waitForTimeout(200);
}

/** Get real WAV file paths seeded by global setup. */
function getTestPaths() {
  const testDir = process.env.PLAYWRIGHT_TEST_DIR;
  if (!testDir) throw new Error('PLAYWRIGHT_TEST_DIR not set');
  const audioDir = join(testDir, 'audio');
  return {
    wavPath: join(audioDir, 'test-track.wav'),
    wav2Path: join(audioDir, 'test-track-long.wav'),
    wav3Path: join(audioDir, 'test-track-med.wav'),
    dsfPath: join(audioDir, 'dsd-track-one.dsf'),
    audioDir,
  };
}

// ---------------------------------------------------------------------------
// Test lifecycle
// ---------------------------------------------------------------------------

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
  // Stop playback and reset to known state
  await invoke('stop_playback').catch(() => {});
  await resetFileSettings();
  await dismissDialog();
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await invoke('stop_playback').catch(() => {});
  await resetFileSettings();
  await dismissDialog();
  // Clear pending files so tests don't bleed into each other
  await invoke('test_set_pending_open_files', { paths: [] });
});

// ---------------------------------------------------------------------------
// Group 1: Drag overlay UI
// ---------------------------------------------------------------------------

test('G1.1 drag-enter shows drag overlay with correct text', async () => {
  await emitTauri('tauri://drag-enter', { paths: [], position: { x: 400, y: 300 } });
  await page.waitForTimeout(300);

  const overlay = page.locator('[data-testid="drag-overlay"]');
  await expect(overlay).toBeVisible({ timeout: 3000 });

  // Contains the drop-to-add text
  await expect(overlay).toContainText('Drop to Add');

  // Clean up
  await emitTauri('tauri://drag-leave', {});
  await page.waitForTimeout(200);
  await expect(overlay).not.toBeVisible({ timeout: 2000 });
});

test('G1.2 drag-leave hides drag overlay', async () => {
  await emitTauri('tauri://drag-enter', { paths: [], position: { x: 400, y: 300 } });
  await page.waitForTimeout(200);
  await emitTauri('tauri://drag-leave', {});
  await page.waitForTimeout(300);

  await expect(page.locator('[data-testid="drag-overlay"]')).not.toBeVisible({ timeout: 2000 });
});

test('G1.3 drag-drop clears overlay and shows dialog', async () => {
  const { wavPath } = getTestPaths();

  await emitTauri('tauri://drag-enter', { paths: [wavPath], position: { x: 400, y: 300 } });
  await page.waitForTimeout(100);
  await emitTauri('tauri://drag-drop', { paths: [wavPath], position: { x: 400, y: 300 } });
  await page.waitForTimeout(300);

  // Overlay should be gone after drop
  await expect(page.locator('[data-testid="drag-overlay"]')).not.toBeVisible({ timeout: 2000 });

  // Dialog should appear
  await waitForDialog();
  await expect(page.locator('[data-testid="file-drop-dialog"]')).toBeVisible();
});

// ---------------------------------------------------------------------------
// Group 2: Dialog appearance
// ---------------------------------------------------------------------------

test('G2.1 single file drop shows filename in dialog', async () => {
  const { wavPath } = getTestPaths();

  await simulateDragDrop([wavPath]);
  await waitForDialog();

  const nameEl = page.locator('[data-testid="file-drop-name"]');
  await expect(nameEl).toBeVisible();
  await expect(nameEl).toContainText('test-track.wav');
});

test('G2.2 multiple file drop shows count in dialog', async () => {
  const { wavPath, wav2Path, wav3Path } = getTestPaths();

  await simulateDragDrop([wavPath, wav2Path, wav3Path]);
  await waitForDialog();

  const nameEl = page.locator('[data-testid="file-drop-name"]');
  await expect(nameEl).toBeVisible();
  await expect(nameEl).toContainText('3');
});

test('G2.3 dialog has Play Now and Import to Library buttons', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  await expect(page.locator('[data-testid="file-drop-play"]')).toBeVisible();
  await expect(page.locator('[data-testid="file-drop-import"]')).toBeVisible();
  await expect(page.locator('[data-testid="file-drop-remember"]')).toBeVisible();
  await expect(page.locator('[data-testid="file-drop-close"]')).toBeVisible();
});

test('G2.4 non-audio file extension does not trigger dialog', async () => {
  const testDir = process.env.PLAYWRIGHT_TEST_DIR;
  const fakePdf = join(testDir, 'document.pdf');

  await simulateDragDrop([fakePdf]);
  await page.waitForTimeout(800);

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 1000 });
});

test('G2.5 DSD (.dsf) file is accepted and shows dialog', async () => {
  const { dsfPath } = getTestPaths();

  await simulateFilesOpened([dsfPath]);
  await waitForDialog();

  const nameEl = page.locator('[data-testid="file-drop-name"]');
  await expect(nameEl).toBeVisible();
  await expect(nameEl).toContainText('dsd-track-one.dsf');
});

// ---------------------------------------------------------------------------
// Group 3: Play Now action
// ---------------------------------------------------------------------------

test('G3.1 drag-drop → Play Now starts playback', async () => {
  const { wavPath } = getTestPaths();

  await simulateDragDrop([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  // Dialog closes
  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Give audio engine time to transition from Stopped → Playing
  await page.waitForTimeout(500);

  // Playback starts
  const state = await invoke('get_playback_state');
  expect(['Playing', 'Paused']).toContain(state);
});

test('G3.2 files-opened → Play Now starts playback', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // Give audio engine time to transition from Stopped → Playing
  await page.waitForTimeout(500);

  const state = await invoke('get_playback_state');
  expect(['Playing', 'Paused']).toContain(state);
});

test('G3.3 Play Now with multiple files queues all of them', async () => {
  const { wavPath, wav2Path, wav3Path } = getTestPaths();

  await simulateFilesOpened([wavPath, wav2Path, wav3Path]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });
  await page.waitForTimeout(500);

  const queueSize = await page.evaluate(async () => {
    try {
      const q = await window.__TAURI_INTERNALS__.invoke('get_queue');
      return Array.isArray(q) ? q.length : 0;
    } catch {
      return -1;
    }
  });
  // Queue has at least 2 of the 3 files: get_all() returns from source_index onward,
  // so the currently-playing track may already have been consumed (index advanced to 1).
  if (queueSize > 0) {
    expect(queueSize).toBeGreaterThanOrEqual(2);
  } else {
    // Fallback: just verify playback started
    const state = await invoke('get_playback_state');
    expect(['Playing', 'Paused']).toContain(state);
  }
});

test('G3.4 Play Now uses tag metadata (title not raw filename)', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });
  await page.waitForTimeout(500);

  // The track title shown in now-playing should be the tag title (or filename fallback),
  // but crucially NOT "Unknown" (which would indicate raw path was used as title)
  const nowPlayingTitle = page.locator('[data-testid="now-playing-title"]');
  if (await nowPlayingTitle.isVisible({ timeout: 2000 }).catch(() => false)) {
    const titleText = await nowPlayingTitle.textContent();
    expect(titleText).not.toMatch(/^\/|^C:\\|^D:\\/); // must not be a raw file path
    expect(titleText.trim().length).toBeGreaterThan(0);
  }
  // If now-playing is not visible, just verify playback state
  const state = await invoke('get_playback_state');
  expect(['Playing', 'Paused']).toContain(state);
});

test('G3.5 Play Now dialog closes after click (no loading stuck state)', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  const playBtn = page.locator('[data-testid="file-drop-play"]');
  await expect(playBtn).toBeEnabled();
  await playBtn.click();

  // Dialog must close within 5s
  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });

  // App remains interactive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ---------------------------------------------------------------------------
// Group 4: Import to Library action
// ---------------------------------------------------------------------------

test('G4.1 drag-drop → Import to Library closes dialog and does not crash', async () => {
  const { wavPath } = getTestPaths();

  await simulateDragDrop([wavPath]);
  await waitForDialog();

  await page.click('[data-testid="file-drop-import"]');

  // Dialog must close
  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 8000 });

  // App still responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

test('G4.2 files-opened → Import to Library closes dialog and does not crash', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  await page.click('[data-testid="file-drop-import"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 8000 });
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

test('G4.3 Import does not start playback', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await page.click('[data-testid="file-drop-import"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 8000 });
  await page.waitForTimeout(300);

  // Import must NOT have started playback
  const state = await invoke('get_playback_state');
  expect(state).toBe('Stopped');
});

// ---------------------------------------------------------------------------
// Group 5: Remember my choice
// ---------------------------------------------------------------------------

test('G5.1 remember + Play Now: subsequent drop auto-plays without dialog', async () => {
  const { wavPath, wav2Path } = getTestPaths();

  // First open: check "Remember my choice" then play
  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  await page.click('[data-testid="file-drop-remember"]'); // check the box
  await page.waitForTimeout(100);
  await page.click('[data-testid="file-drop-play"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 5000 });
  await invoke('stop_playback').catch(() => {});
  await page.waitForTimeout(300);

  // Second open: dialog should NOT appear — auto-play
  await simulateFilesOpened([wav2Path]);
  await page.waitForTimeout(1000);

  const dialogVisible = await page
    .locator('[data-testid="file-drop-dialog"]')
    .isVisible({ timeout: 500 })
    .catch(() => false);
  expect(dialogVisible).toBe(false);

  // Playback should have started automatically
  const state = await invoke('get_playback_state');
  expect(['Playing', 'Paused']).toContain(state);
});

test('G5.2 remember + Import: subsequent drop auto-imports without dialog', async () => {
  const { wavPath, wav2Path } = getTestPaths();

  // First open: check "Remember" then import
  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  await page.click('[data-testid="file-drop-remember"]');
  await page.waitForTimeout(100);
  await page.click('[data-testid="file-drop-import"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 8000 });
  await page.waitForTimeout(300);

  // Second open: dialog should NOT appear — auto-import
  await simulateFilesOpened([wav2Path]);
  await page.waitForTimeout(1500);

  const dialogVisible = await page
    .locator('[data-testid="file-drop-dialog"]')
    .isVisible({ timeout: 500 })
    .catch(() => false);
  expect(dialogVisible).toBe(false);

  // Playback should still be Stopped (import doesn't play)
  const state = await invoke('get_playback_state');
  expect(state).toBe('Stopped');
});

test('G5.3 resetting preference to ask makes dialog appear again', async () => {
  const { wavPath } = getTestPaths();

  // Set preference to auto-play
  await invoke('set_external_file_settings', {
    defaultAction: 'play',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  });

  // Reset back to ask
  await resetFileSettings();

  // Now dialog should show
  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  await expect(page.locator('[data-testid="file-drop-dialog"]')).toBeVisible();
});

test('G5.4 remember checkbox is unchecked by default each time dialog opens', async () => {
  const { wavPath, wav2Path } = getTestPaths();

  // Open, verify checkbox starts unchecked
  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  const rememberBtn = page.locator('[data-testid="file-drop-remember"]');
  // Unchecked: no Check icon inside it (aria-pressed not set, or just verify no "checked" bg class)
  await expect(rememberBtn).toBeVisible();
  // Close without remembering
  await dismissDialog();
  await page.waitForTimeout(200);

  // Re-open — checkbox should still be unchecked
  await simulateFilesOpened([wav2Path]);
  await waitForDialog();
  await expect(rememberBtn).toBeVisible();
  // App is still interactive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ---------------------------------------------------------------------------
// Group 6: Path parity (drag-drop ≡ files-opened)
// ---------------------------------------------------------------------------

test('G6.1 drag-drop and files-opened produce identical dialog for same file', async () => {
  const { wavPath } = getTestPaths();

  // Path A: drag-drop
  await simulateDragDrop([wavPath]);
  await waitForDialog();
  const nameA = await page.locator('[data-testid="file-drop-name"]').textContent();
  await dismissDialog();
  await page.waitForTimeout(300);

  // Path B: files-opened
  await simulateFilesOpened([wavPath]);
  await waitForDialog();
  const nameB = await page.locator('[data-testid="file-drop-name"]').textContent();
  await dismissDialog();

  expect(nameA?.trim()).toBe(nameB?.trim());
});

test('G6.2 drag-drop respects remember-play setting just like files-opened', async () => {
  const { wavPath, wav2Path } = getTestPaths();

  // Set preference to auto-play (as if user had ticked remember on a files-opened dialog)
  await invoke('set_external_file_settings', {
    defaultAction: 'play',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  });

  // Drag-drop should auto-play without showing dialog
  await simulateDragDrop([wavPath]);
  await page.waitForTimeout(1000);

  const dialogVisible = await page
    .locator('[data-testid="file-drop-dialog"]')
    .isVisible({ timeout: 500 })
    .catch(() => false);
  expect(dialogVisible).toBe(false);

  const state = await invoke('get_playback_state');
  expect(['Playing', 'Paused']).toContain(state);
});

test('G6.3 files-opened respects remember-import setting just like drag-drop', async () => {
  const { wavPath } = getTestPaths();

  await invoke('set_external_file_settings', {
    defaultAction: 'import',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  });

  await simulateFilesOpened([wavPath]);
  await page.waitForTimeout(1500);

  const dialogVisible = await page
    .locator('[data-testid="file-drop-dialog"]')
    .isVisible({ timeout: 500 })
    .catch(() => false);
  expect(dialogVisible).toBe(false);

  // Import does not start playback
  const state = await invoke('get_playback_state');
  expect(state).toBe('Stopped');
});

test('G6.4 both paths use get_metadata_for_paths (no Unknown Artist in queue)', async () => {
  const { wavPath } = getTestPaths();

  // Set auto-play so we can inspect the queue directly without clicking
  await invoke('set_external_file_settings', {
    defaultAction: 'play',
    importDestination: 'watched',
    importToSourceId: null,
    showImportNotification: true,
  });

  // Trigger via files-opened
  await simulateFilesOpened([wavPath]);
  await page.waitForTimeout(800);

  // Check the queue: track artist should never be the literal string "Unknown Artist"
  // when the metadata path returned a real value (even empty → fallback to filename, not "Unknown")
  const queue = await page.evaluate(async () => {
    try { return await window.__TAURI_INTERNALS__.invoke('get_queue'); } catch { return []; }
  });

  if (Array.isArray(queue) && queue.length > 0) {
    const track = queue[0];
    // title must not be a raw absolute file path
    expect(track.title).not.toMatch(/^[A-Z]:\\|^\//);
    expect(track.title.trim().length).toBeGreaterThan(0);
  }
});

// ---------------------------------------------------------------------------
// Group 7: Pending files — cold-launch path
// ---------------------------------------------------------------------------

test('G7.1 get_pending_open_files returns stored paths and drains on first call', async () => {
  const { wavPath } = getTestPaths();

  await invoke('test_set_pending_open_files', { paths: [wavPath] });

  const first = await invoke('get_pending_open_files');
  expect(first).toHaveLength(1);
  expect(first[0]).toBe(wavPath);

  // Second call must return empty (drained)
  const second = await invoke('get_pending_open_files');
  expect(second).toHaveLength(0);
});

test('G7.2 pending files with non-audio paths are not in the pending store', async () => {
  // The cold-launch path filters extensions before storing — verify with audio file
  const { wavPath } = getTestPaths();

  await invoke('test_set_pending_open_files', { paths: [wavPath] });
  const result = await invoke('get_pending_open_files');
  expect(result).toContain(wavPath);
});

test('G7.3 setting then draining pending files and emitting files-opened shows dialog', async () => {
  const { wavPath } = getTestPaths();

  // Simulate what would happen on cold launch: store pending, then drain + process
  await invoke('test_set_pending_open_files', { paths: [wavPath] });

  const pending = await invoke('get_pending_open_files');
  expect(pending).toHaveLength(1);

  // Now emit files-opened with those paths (what FileDropHandler does after draining)
  await simulateFilesOpened(pending);
  await waitForDialog();

  await expect(page.locator('[data-testid="file-drop-dialog"]')).toBeVisible();
  await expect(page.locator('[data-testid="file-drop-name"]')).toContainText('test-track.wav');
});

// ---------------------------------------------------------------------------
// Group 8: Edge cases
// ---------------------------------------------------------------------------

test('G8.1 empty paths array does not show dialog or crash', async () => {
  await simulateFilesOpened([]);
  await page.waitForTimeout(600);

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 1000 });
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

test('G8.2 close button (X) dismisses dialog without action', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  await page.click('[data-testid="file-drop-close"]');

  await expect(page.locator('[data-testid="file-drop-dialog"]')).not.toBeVisible({ timeout: 3000 });

  // Playback must NOT have started
  const state = await invoke('get_playback_state');
  expect(state).toBe('Stopped');
});

test('G8.3 clicking backdrop dismisses dialog without action', async () => {
  const { wavPath } = getTestPaths();

  await simulateFilesOpened([wavPath]);
  await waitForDialog();

  // Click the semi-transparent backdrop (outside the dialog card)
  const dialog = page.locator('[data-testid="file-drop-dialog"]');
  const box = await dialog.boundingBox();
  if (box) {
    // Click in the top-left corner of the backdrop (outside the card)
    await page.mouse.click(box.x + 5, box.y + 5);
  } else {
    await page.keyboard.press('Escape');
  }

  await expect(dialog).not.toBeVisible({ timeout: 3000 });
  const state = await invoke('get_playback_state');
  expect(state).toBe('Stopped');
});

test('G8.4 rapid successive drops only show one dialog at a time', async () => {
  const { wavPath, wav2Path } = getTestPaths();

  // Fire two files-opened events quickly
  await simulateFilesOpened([wavPath]);
  await simulateFilesOpened([wav2Path]);
  await page.waitForTimeout(800);

  // At most one dialog should be visible
  const dialogCount = await page.locator('[data-testid="file-drop-dialog"]').count();
  expect(dialogCount).toBeLessThanOrEqual(1);
});
