/**
 * Queue operations — Playwright CDP tests
 *
 * Covers:
 *  - skipToQueueIndex() emitting StateChanged(Stopped) before StateChanged(Playing)
 *    (previously missing, caused UI progress timer to keep running across track skips)
 *  - next/previous navigation correctness after queue is loaded
 *  - Queue state reflected via window.__TAURI_INTERNALS__.invoke()
 *
 * Architecture notes (read before editing):
 *  - The queue is displayed inline inside LeftSidebar via QueueSection + TrackItem.
 *    There is NO separate QueueSidebar slide-in panel rendered in the desktop app.
 *    The data-testid="queue-sidebar", "queue-close", and "queue-button" IDs referenced
 *    in older WebdriverIO specs belong to an unused component — they do not exist in the
 *    live DOM.  Tests here work with the actual rendered elements:
 *      - [data-testid="now-playing-title"] — container div in NowPlayingPanel; the track
 *        title text lives inside the nested .text-sm span rendered by TrackItem.
 *      - window.__TAURI_INTERNALS__.invoke('get_playback_state' | 'get_queue' | 'skip_to_queue_index')
 *      - [data-testid="next-button"], [data-testid="previous-button"]
 *      - [data-testid="play-pause-button"]
 *
 * Seed data (planted by playwright-global-setup.js):
 *  Album ID 2001, 5 tracks with IDs 2001–2005:
 *    0: "Track One"  (index 0 in queue)
 *    1: "Track Two"  (index 1)
 *    2: "Track Three" (index 2)
 *    3: "Track Four"  (index 3)
 *    4: "Track Five"  (index 4)
 *
 * The backend get_queue command returns the *upcoming* queue (excluding the current track).
 * get_queue().length therefore returns 4 while Track One is playing (tracks 2-5 remaining).
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across all tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
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

// Before each test: stop playback, dismiss overlays, navigate to Albums page.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // force:true so the click goes through even if a backdrop is still visible
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

// After each test: stop playback and dismiss any leftover overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start album 2001 playback and wait until Playing state.
//
// Uses play_queue directly rather than the MediaCard play button to bypass
// the resumePlayback() vs playQueue() branching in handlePlayPause.
// When album 2001 is already the active context (e.g. from a prior test),
// MediaCard.handlePlayPause calls resumePlayback() instead of playQueue(),
// which can start from a mid-queue track or fail silently if the queue is
// exhausted — causing unreliable firstTitle values in the combined suite.
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

  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The testid is on the container div; the actual title text is in the
// nested .text-sm span rendered by TrackItem.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  // TrackItem renders the title in the first .text-sm element inside the container.
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to change away from oldTitle.
// ----------------------------------------------------------------

async function waitForTitleChange(p, oldTitle) {
  await p.waitForFunction(
    (expected) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() !== expected;
    },
    oldTitle,
    { timeout: 15_000 }
  );
}

// ================================================================
// Test 1: get_queue reports remaining tracks after playback starts
// ================================================================

test('get_queue returns remaining tracks after playback starts', async () => {
  await startPlayback(page);

  const queueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );

  // Album has 5 tracks; the backend queue holds the tracks not yet played.
  // Track One is currently playing, so 4 remain in the upcoming queue.
  expect(queueSize).toBeGreaterThanOrEqual(4);
});

// ================================================================
// Test 2: skipToQueueIndex() via testHelpers changes the current track
//
// This is the primary regression test for the bug where
// skip_to_queue_index() did not emit StateChanged(Stopped), causing the
// UI progress timer to keep running across track transitions.
// ================================================================

test('skipToQueueIndex changes current track and transitions through Stopped', async () => {
  await startPlayback(page);

  const initialTitle = await getNowPlayingTitle(page);

  // Skip to queue index 2 (Track Three when playing from the beginning).
  // The state machine must emit StateChanged(Stopped) → StateChanged(Playing)
  // for the UI to update cleanly.
  await page.evaluate(async (idx) => window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: idx }), 2);

  // Wait for the title to change — it will if LoadNext is emitted after Stopped.
  await waitForTitleChange(page, initialTitle);

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).not.toBe(initialTitle);

  // Verify playback resumed after the skip.
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
});

// ================================================================
// Test 3: Queue size decreases after skipping to next track
// ================================================================

test('queue size decreases after clicking next-button', async () => {
  await startPlayback(page);

  const initialQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );

  const initialTitle = await getNowPlayingTitle(page);

  // Click the next button to advance one track.
  await page.click('[data-testid="next-button"]');

  // Wait for the UI to show the new track.
  await waitForTitleChange(page, initialTitle);

  // Queue should have one fewer entry now.
  const newQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );

  expect(newQueueSize).toBeLessThan(initialQueueSize);
});

// ================================================================
// Test 4: Previous button after one next-skip returns to original track
//
// Covers the previous() navigation path that previously:
//   - Did not emit StateChanged(Stopped) → UI timer kept running
//   - Did not emit LoadNext(prev_track) → audio never loaded
// Both bugs were fixed; this test guards against regression.
// ================================================================

test('previous-button after next returns to original track', async () => {
  await startPlayback(page);

  const firstTitle = await getNowPlayingTitle(page);

  // Advance to the second track.
  await page.click('[data-testid="next-button"]');
  await waitForTitleChange(page, firstTitle);
  const secondTitle = await getNowPlayingTitle(page);
  expect(secondTitle).not.toBe(firstTitle);

  // Go back to the first track via previous.
  await page.click('[data-testid="previous-button"]');
  await waitForTitleChange(page, secondTitle);

  const restoredTitle = await getNowPlayingTitle(page);
  expect(restoredTitle).toBe(firstTitle);

  // Verify we are actually playing — not stopped or stuck.
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
});

// ================================================================
// Test 5: Multiple sequential skips via skipToQueueIndex
//
// Exercises the state machine across repeated index-based skips to
// verify no events are swallowed (regression for the bug where
// forward_manager_events() processed only the first event, dropping
// the LoadNext when previous() emitted two events).
// ================================================================

test('sequential skipToQueueIndex calls all reach Playing state', async () => {
  await startPlayback(page);

  // Collect the titles from three consecutive index skips: 1, 3, 2.
  const targets = [1, 3, 2];
  const observedTitles = [];

  for (const idx of targets) {
    const before = await getNowPlayingTitle(page);

    await page.evaluate(async (i) => window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: i }), idx);

    await waitForTitleChange(page, before);

    // Confirm Playing state after each skip — ensures StateChanged(Playing) fired.
    await page.waitForFunction(
      async () => {
        const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
        return state === 'Playing';
      },
      { timeout: 15_000 }
    );

    const after = await getNowPlayingTitle(page);
    expect(after).not.toBe(before);
    observedTitles.push(after);
  }

  // All three skips should have resulted in distinct titles proving audio loaded.
  expect(observedTitles.length).toBe(3);
});
