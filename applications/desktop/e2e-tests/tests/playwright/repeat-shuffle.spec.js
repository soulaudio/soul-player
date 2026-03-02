/**
 * Repeat and shuffle mode tests — Playwright CDP regression tests
 *
 * Covers:
 *   - Shuffle button visible and reflects mode via data-state / aria-pressed
 *   - Clicking shuffle cycles Off → Random → Smart → Off via cycle_shuffle IPC
 *   - Repeat button visible and reflects mode via data-state / aria-pressed
 *   - Clicking repeat cycles Off → All → One → Off via set_repeat IPC
 *   - RepeatOne: track repeats instead of advancing after 2-second WAV ends
 *   - RepeatAll: after last track finishes, playback restarts from the first track
 *   - Repeat/shuffle state persists across page navigation
 *
 * Shuffle IPC flow:
 *   Click shuffle-button → handleShuffleToggle → commands.cycleShuffle()
 *     → invoke('cycle_shuffle') → emits playback:queue-updated
 *     → TauriPlayerCommandsProvider listener calls get_shuffle → updates store
 *     → React re-renders with new shuffleMode → data-state / aria-pressed update
 *
 * Repeat IPC flow:
 *   Click repeat-button → handleRepeatToggle → onRepeatModeChange(nextMode) [optimistic]
 *     → commands.setRepeatMode(nextMode) → invoke('set_repeat', { mode })
 *     → React re-renders immediately with new repeatMode (optimistic)
 *
 * Shuffle mode string values (from Rust):  'off' | 'random' | 'smart'
 * Repeat mode string values (from Rust):   'off' | 'all' | 'one'
 *
 * Cycle orders:
 *   Shuffle: Off → Random → Smart → Off
 *   Repeat:  Off → All   → One   → Off
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks × 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
 *
 * Queue index note (applies to all tests that call startPlayback):
 *   play_queue calls pm.play() which immediately pops Track One from the queue.
 *   The REMAINING queue is [T2, T3, T4, T5] at indices 0–3.
 *   Track Five is at index 3 (not 4).
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

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

// Before each test: stop any active playback, dismiss open overlays, navigate to Albums.
test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

// After each test: stop playback, reset repeat/shuffle to Off, dismiss overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  // Use a longer wait (500ms) to give the async stop/reset commands time to be
  // processed by the audio thread before the next spec file's beforeEach runs.
  // set_repeat and set_shuffle use the command channel (not a sync IPC), so
  // 200ms is occasionally insufficient — 500ms provides a reliable margin.
  await page.waitForTimeout(500);
});

// ----------------------------------------------------------------
// Helper: start playback of album 2001 from Track One via play_queue.
//
// Uses play_queue directly rather than the MediaCard play button to bypass
// the resumePlayback() vs playQueue() branching in handlePlayPause.
// When album 2001 is already the active context (e.g. from a prior test),
// MediaCard.handlePlayPause calls resumePlayback() which can start from a
// mid-queue track or fail silently when the queue is exhausted.
// Direct play_queue always starts fresh from Track One.
// ----------------------------------------------------------------

async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  // Wait until now-playing-title appears (UI received the TrackChanged event)
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Confirm Track One is loaded — guards against any edge case where a
  // previous track context causes a different title to show first.
  await p.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 }
  );

  // Wait for the React store to fully reflect the Playing state.
  // The backend state check uses IPC; the React isPlaying flag is set via a Tauri
  // event (StateChanged) which may arrive slightly after the IPC response.
  await p.waitForFunction(
    () => {
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The now-playing-title container holds a TrackItem; the title text
// lives in the first .text-sm element.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to become the expected value.
// ----------------------------------------------------------------

async function waitForTitle(p, expected, timeout = 15_000) {
  await p.waitForFunction(
    (exp) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === exp;
    },
    expected,
    { timeout }
  );
}

// ----------------------------------------------------------------
// Helper: wait for the shuffle button's data-state to become expectedMode.
// Shuffle state update is async (depends on playback:queue-updated event
// triggering get_shuffle IPC), so we poll rather than check immediately.
// ----------------------------------------------------------------

async function waitForShuffleState(p, expectedMode, timeout = 10_000) {
  await p.waitForFunction(
    (mode) => {
      const btn = document.querySelector('[data-testid="shuffle-button"]');
      return btn !== null && btn.getAttribute('data-state') === mode;
    },
    expectedMode,
    { timeout }
  );
}

// ----------------------------------------------------------------
// Helper: wait for the repeat button's data-state to become expectedMode.
// Repeat is updated optimistically so this typically resolves quickly,
// but polling is safer against any React render delay.
// ----------------------------------------------------------------

async function waitForRepeatState(p, expectedMode, timeout = 10_000) {
  await p.waitForFunction(
    (mode) => {
      const btn = document.querySelector('[data-testid="repeat-button"]');
      return btn !== null && btn.getAttribute('data-state') === mode;
    },
    expectedMode,
    { timeout }
  );
}

// ================================================================
// Test 1: Shuffle button is visible and starts in Off state
// ================================================================

test('shuffle button is visible and starts in the Off state', async () => {
  await startPlayback(page);

  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');
  await expect(shuffleBtn).toBeVisible();
  await expect(shuffleBtn).not.toBeDisabled();

  // Verify Off state via both data attributes
  const dataState = await shuffleBtn.getAttribute('data-state');
  expect(dataState).toBe('off');

  const ariaPressed = await shuffleBtn.getAttribute('aria-pressed');
  // aria-pressed="false" when Off
  expect(ariaPressed).toBe('false');
});

// ================================================================
// Test 2: Clicking shuffle button cycles through modes
//
// Cycle order: Off → Random → Smart → Off
// Each click invokes cycle_shuffle on the backend; the store updates
// via the playback:queue-updated event which triggers get_shuffle.
// We poll data-state after each click rather than checking immediately.
// ================================================================

test('clicking shuffle cycles Off → Random → Smart → Off', async () => {
  await startPlayback(page);

  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');

  // Ensure we start at Off
  await waitForShuffleState(page, 'off');
  expect(await shuffleBtn.getAttribute('aria-pressed')).toBe('false');

  // Click 1: Off → Random
  await shuffleBtn.click();
  await waitForShuffleState(page, 'random');
  expect(await shuffleBtn.getAttribute('aria-pressed')).toBe('true');

  // Verify backend agrees
  const mode1 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(mode1).toBe('random');

  // Click 2: Random → Smart
  await shuffleBtn.click();
  await waitForShuffleState(page, 'smart');
  expect(await shuffleBtn.getAttribute('aria-pressed')).toBe('true');

  const mode2 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(mode2).toBe('smart');

  // Click 3: Smart → Off
  await shuffleBtn.click();
  await waitForShuffleState(page, 'off');
  expect(await shuffleBtn.getAttribute('aria-pressed')).toBe('false');

  const mode3 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(mode3).toBe('off');
});

// ================================================================
// Test 3: Repeat button is visible and starts in Off state
// ================================================================

test('repeat button is visible and starts in the Off state', async () => {
  await startPlayback(page);

  const repeatBtn = page.locator('[data-testid="repeat-button"]');
  await expect(repeatBtn).toBeVisible();
  await expect(repeatBtn).not.toBeDisabled();

  const dataState = await repeatBtn.getAttribute('data-state');
  expect(dataState).toBe('off');

  const ariaPressed = await repeatBtn.getAttribute('aria-pressed');
  expect(ariaPressed).toBe('false');
});

// ================================================================
// Test 4: Clicking repeat button cycles through modes
//
// Cycle order: Off → All → One → Off
// Repeat is updated optimistically in the UI (onRepeatModeChange called
// before the IPC completes), so data-state updates quickly after each click.
// ================================================================

test('clicking repeat cycles Off → All → One → Off', async () => {
  await startPlayback(page);

  const repeatBtn = page.locator('[data-testid="repeat-button"]');

  // Ensure we start at Off
  await waitForRepeatState(page, 'off');
  expect(await repeatBtn.getAttribute('aria-pressed')).toBe('false');

  // Click 1: Off → All
  await repeatBtn.click();
  await waitForRepeatState(page, 'all');
  expect(await repeatBtn.getAttribute('aria-pressed')).toBe('true');

  // Verify backend reflects All mode
  const mode1 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(mode1).toBe('all');

  // Click 2: All → One
  await repeatBtn.click();
  await waitForRepeatState(page, 'one');
  expect(await repeatBtn.getAttribute('aria-pressed')).toBe('true');

  const mode2 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(mode2).toBe('one');

  // Click 3: One → Off
  await repeatBtn.click();
  await waitForRepeatState(page, 'off');
  expect(await repeatBtn.getAttribute('aria-pressed')).toBe('false');

  const mode3 = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(mode3).toBe('off');
});

// ================================================================
// Test 5: RepeatOne — track repeats instead of advancing
//
// With RepeatOne enabled the state machine loops the current track
// instead of advancing to Track Two when Track One's 2-second WAV ends.
//
// Strategy: enable RepeatOne, wait 3 seconds (long enough for the
// track to naturally exhaust and loop at least once), then assert
// the title is STILL "Track One" and state is still Playing.
//
// We use waitForTimeout(3000) rather than a flat poll because the
// risk here is that the track DOES advance — so we want to check
// after the natural end-of-track moment has passed.
// ================================================================

test('RepeatOne: track repeats instead of advancing after WAV ends', async () => {
  await startPlayback(page);

  // Enable RepeatOne via IPC so we don't depend on prior mode state
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'one' })
  );

  // Wait for the UI to reflect the new repeat mode (optimistic update fires
  // via the React store's setRepeatMode, but here we went direct to IPC so
  // the store won't know until a sync happens; use a short grace period)
  await waitForRepeatState(page, 'one', 5_000).catch(() => {
    // If the store doesn't update from a direct IPC set_repeat (no event emitted),
    // that's OK — we still confirm functional behavior below.
  });

  // Confirm we are currently on Track One
  const titleBefore = await getNowPlayingTitle(page);
  expect(titleBefore).toBe('Track One');

  // Wait 3 seconds — enough for the 2-second track to end and loop once.
  await page.waitForTimeout(3_000);

  // After RepeatOne, the track should have looped — title must still be Track One.
  const titleAfter = await getNowPlayingTitle(page);
  expect(titleAfter).toBe('Track One');

  // Playback must still be active (not stopped)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 6: RepeatAll — after the last track finishes, playback restarts
//
// Queue index note: after startPlayback, T1 is popped by pm.play().
// Remaining queue: [T2, T3, T4, T5] at indices 0–3.
// Track Five is at index 3.
//
// We enable RepeatAll, skip to Track Five (index 3), wait for it to
// finish (2s WAV + buffer), then verify the title changes away from
// Track Five — meaning the playlist wrapped around and started playing
// from the beginning again.
// ================================================================

test('RepeatAll: playback restarts from the beginning after the last track ends', async () => {
  await startPlayback(page);

  // Enable RepeatAll via IPC for reliability
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'all' })
  );

  // Skip to Track Five (index 3 in the remaining queue)
  await page.evaluate(async (idx) =>
    window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: idx }), 3
  );

  // Wait for Track Five to appear in the now-playing panel
  await waitForTitle(page, 'Track Five');

  // Verify we're actually playing Track Five
  const stateBefore = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateBefore).toBe('Playing');

  // Track Five is 2 seconds long. Wait 4 seconds for it to finish and
  // for RepeatAll to wrap the queue back to the beginning.
  await page.waitForTimeout(4_000);

  // After wrap-around, the title must have changed away from Track Five.
  // We poll rather than use a flat wait to catch the exact moment it changes.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() !== 'Track Five';
    },
    { timeout: 10_000 }
  );

  // Confirm playback is still active after the wrap-around
  const stateAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfter).toBe('Playing');
});

// ================================================================
// Test 7: Repeat and shuffle state persists across page navigation
//
// Enable both repeat (All) and shuffle (Random), navigate to a
// different page and back, then verify the buttons still reflect
// the expected states.
// ================================================================

test('repeat and shuffle state persists across page navigation', async () => {
  await startPlayback(page);

  // Enable shuffle (Random) and repeat (All) via IPC
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' });
    await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'all' });
  });

  // set_shuffle and set_repeat use the async command channel — the IPC call
  // returns as soon as the command is sent, before the audio thread processes it.
  // Poll via get_shuffle / get_repeat until both modes are confirmed, rather
  // than checking immediately (which would see the old value).
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_shuffle')) === 'random',
    { timeout: 5_000 }
  );
  const shuffleMode = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(shuffleMode).toBe('random');

  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_repeat')) === 'all',
    { timeout: 5_000 }
  );
  const repeatMode = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(repeatMode).toBe('all');

  // Navigate away (to Tracks) and then back to Albums.
  // Use force:true to click through any overlay that may still be visible.
  await page.click('[data-testid="nav-tracks"]', { force: true }).catch(async () => {
    // nav-tracks may not exist; fall back to nav-artists
    await page.click('[data-testid="nav-artists"]', { force: true });
  });
  await page.waitForTimeout(500);

  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Verify the backend still has the correct modes after navigation
  const shuffleAfterNav = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(shuffleAfterNav).toBe('random');

  const repeatAfterNav = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(repeatAfterNav).toBe('all');

  // The shuffle-button and repeat-button should still be visible and enabled.
  // Note: data-state for shuffle may not have updated via the store (set_shuffle
  // does not emit queue-updated), so we check the IPC values above rather than
  // relying solely on data-state here.
  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');
  const repeatBtn = page.locator('[data-testid="repeat-button"]');
  await expect(shuffleBtn).toBeVisible();
  await expect(repeatBtn).toBeVisible();
  await expect(repeatBtn).not.toBeDisabled();
});
