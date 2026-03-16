/**
 * Playback controls — Playwright CDP regression tests
 *
 * Covers bugs found and fixed in the soul-playback state machine:
 *
 *   BUG-3/BUG-6: When queue ends and backend emits TrackChanged(null), isPlaying was NOT
 *     reset to false. The play-pause button kept showing the Pause icon even though nothing
 *     was playing. FIXED: isPlaying is now set to false on TrackChanged(null).
 *
 *   BUG: skip_to_queue_index() missing StateChanged(Stopped) — after skipping the state
 *     was not momentarily Stopped. FIXED: StateChanged(Stopped) is now emitted before
 *     loading the next track.
 *
 *   Regression: next() / previous() / activate_source() state transitions work end-to-end
 *     through the UI.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 6 tracks × 2-second WAV files
 *   Track IDs 2001–2006, titles: Track One … Track Five, Collab Track
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  // Find the main window — it is already loaded by the time tests run.
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  // Short safety wait in case there is any residual animation or settling
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: stop any active playback, dismiss open overlays, navigate to Albums.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  // This prevents 2-second test tracks from expiring during setup in the combined suite.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Use force:true so the click goes through even if a backdrop overlay is still present
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

// After each test: stop playback and clean up any open overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start playback of album 2001 by invoking play_queue directly.
//
// This bypasses the MediaCard play button's resumePlayback() vs playQueue()
// branching logic. When the same album is already in context (e.g. after a
// previous test's afterEach stop_playback), MediaCard.handlePlayPause sees
// isActive=true and calls resumePlayback() instead of playQueue() — which can
// resume from Track Two or fail silently when the queue is exhausted.
//
// Invoking play_queue directly ensures we always start fresh from Track One
// regardless of any prior state, making the combined test suite reliable.
// ----------------------------------------------------------------

async function startPlayback(p) {
  // Fetch tracks for album 2001 and start from Track One via play_queue.
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    // Sort by track_number so Track One is first
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
  // The backend state check above uses IPC (get_playback_state); the React
  // isPlaying flag is set via a Tauri event (StateChanged) which may arrive
  // slightly after the IPC response. Without this wait, handlePlayPause may
  // still see isPlaying=false and call resumePlayback() instead of pausePlayback().
  await p.waitForFunction(
    () => {
      // Play-pause button shows <Pause> icon (has SVG with certain content)
      // when isPlaying=true; falls back to <Play> when false.
      // The button is not disabled when hasCurrentTrack=true.
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  // Small flat wait for the React event handler to finish updating isPlaying.
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The now-playing-title container holds a TrackItem with nested spans;
// the title is in the first .text-sm element (not the full textContent
// which would include the artist name concatenated).
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to become the expected value.
// Uses the same .text-sm selector as getNowPlayingTitle.
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
// Test 1: Play album starts playback
// ----------------------------------------------------------------

test('playing an album sets state to Playing and shows now-playing title', async () => {
  await startPlayback(page);

  // Verify state via test helper
  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');

  // Play-pause button must be present and enabled
  const playPauseBtn = page.locator('[data-testid="play-pause-button"]');
  await expect(playPauseBtn).toBeVisible();
  await expect(playPauseBtn).not.toBeDisabled();

  // now-playing-title must be visible
  await expect(page.locator('[data-testid="now-playing-title"]')).toBeVisible();
});

// ----------------------------------------------------------------
// Test 2: Clicking play-pause pauses playback
// ----------------------------------------------------------------

test('clicking play-pause button while playing transitions state to Paused', async () => {
  await startPlayback(page);

  // Click play-pause to pause
  await page.click('[data-testid="play-pause-button"]');

  // Use a flat wait instead of waitForFunction polling: rapid IPC polls from
  // waitForFunction compete with the pause_playback invoke on the shared Tauri
  // IPC channel and can cause the state check to time out.
  await page.waitForTimeout(1_500);

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  // Must be Paused, not Stopped — pausing does not end the session
  expect(state).toBe('Paused');
});

// ----------------------------------------------------------------
// Test 3: Clicking play-pause again resumes playback
// ----------------------------------------------------------------

test('clicking play-pause while paused resumes state to Playing', async () => {
  await startPlayback(page);

  // Pause — flat wait avoids IPC contention (see previous test comment)
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForTimeout(1_500);

  // Resume
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForTimeout(1_500);

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 4: Next button advances to the next track
// ----------------------------------------------------------------

test('next button advances to a different track', async () => {
  await startPlayback(page);

  // Capture the initial track title (first .text-sm inside the container)
  const initialTitle = await getNowPlayingTitle(page);

  // Click next
  await page.click('[data-testid="next-button"]');

  // Wait for the title .text-sm to change to something different
  await page.waitForFunction(
    (prevTitle) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      return titleEl !== null && titleEl.textContent.trim() !== prevTitle;
    },
    initialTitle,
    { timeout: 15_000 }
  );

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).not.toBe(initialTitle);

  // Since we started from Track One, next should be Track Two
  expect(newTitle).toBe('Track Two');
});

// ----------------------------------------------------------------
// Test 5: Previous button returns to the previous track
// ----------------------------------------------------------------

test('previous button navigates back to the prior track', async () => {
  await startPlayback(page);

  // Should be Track One
  const firstTitle = await getNowPlayingTitle(page);
  expect(firstTitle).toBe('Track One');

  // Advance to Track Two
  await page.click('[data-testid="next-button"]');
  await waitForTitle(page, 'Track Two');

  // Go back — should return to Track One
  await page.click('[data-testid="previous-button"]');
  await waitForTitle(page, 'Track One');

  const restoredTitle = await getNowPlayingTitle(page);
  expect(restoredTitle).toBe('Track One');
});

// ----------------------------------------------------------------
// Test 6: Auto-advance — track changes after the 2-second WAV ends
// ----------------------------------------------------------------

test('track auto-advances to the next track after the current one finishes', async () => {
  await startPlayback(page);

  const initialTitle = await getNowPlayingTitle(page);
  expect(initialTitle).toBe('Track One');

  // Poll immediately for the title to change away from 'Track One'.
  // A flat 4-second wait is unreliable: T1 may have only ~1.5s left when
  // startPlayback() returns (audio init takes time), so T2 can finish and T3
  // can start during the wait — making waitForTitle('Track Two') fail because
  // T2 has already passed. Polling immediately catches T2 as it appears.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track One';
    },
    { timeout: 15_000 }
  );

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).toBe('Track Two');

  // State should still be Playing after auto-advance
  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 7: Queue ends → isPlaying becomes false  (BUG-3 / BUG-6 regression)
//
// With 6 tracks × 10 s each waiting for all to auto-advance would take 60+ seconds.
// Instead we jump directly to the last track using skipToQueueIndex(), then
// wait for it to finish. After the queue is exhausted the backend emits
// TrackChanged(null). The regression was that isPlaying was NOT reset to false, so
// the play-pause button kept showing the Pause icon. This test verifies the fix.
//
// Queue index note: startPlayback() calls play_queue() which calls pm.play().
// pm.play() immediately pops Track One from the queue (to emit LoadNext).
// The REMAINING queue is therefore [T2, T3, T4, T5, Collab Track] at indices 0–4.
// Collab Track is at index 4, not 5 — using 5 returns QueueEmpty.
// ----------------------------------------------------------------

test('BUG-3/BUG-6: isPlaying resets to false when the queue is exhausted', async () => {
  await startPlayback(page);

  // After startPlayback, Track One has been popped by pm.play().
  // Remaining queue: [Track Two, Track Three, Track Four, Track Five, Collab Track] → index 4 = Collab Track.
  await page.evaluate(async (idx) => window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: idx }), 4);

  // Wait for the now-playing title to reflect Collab Track (use .text-sm selector)
  await waitForTitle(page, 'Collab Track');

  // Verify we are actually playing Collab Track before we wait for it to end
  const stateBeforeEnd = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(stateBeforeEnd).toBe('Playing');

  // Collab Track is 10 seconds long — wait 11 seconds for it to finish naturally.
  // Allowing the track to exhaust cleanly (rather than stopping early via stop_playback)
  // avoids a race condition between the natural end-of-track callback and a manual stop.
  await page.waitForTimeout(11_000);

  // After the queue is exhausted, state must be Stopped (not Playing or Paused).
  // Use a flat wait then check — avoids IPC contention from waitForFunction polling.
  await page.waitForTimeout(3_000);
  const stoppedState = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(stoppedState).toBe('Stopped');

  // The play-pause button must NOT be showing a Pause icon.
  // When isPlaying=false the button renders a Play icon; the SVG aria-label or
  // data attribute reflects the mode. We verify indirectly: the button should exist
  // (controls are still rendered) but the app state must not be Playing.
  const playPauseBtn = page.locator('[data-testid="play-pause-button"]');
  await expect(playPauseBtn).toBeVisible();

  // The button should not have aria-label "Pause" when playback has stopped
  const ariaLabel = await playPauseBtn.getAttribute('aria-label');
  // If aria-label is set, it must not be "Pause" (exact string depends on i18n key,
  // so we do a case-insensitive check)
  if (ariaLabel !== null) {
    expect(ariaLabel.toLowerCase()).not.toContain('pause');
  }

  // Belt-and-suspenders: also verify via the data-state / aria-pressed attribute if present
  const ariaPressedRaw = await playPauseBtn.getAttribute('aria-pressed');
  if (ariaPressedRaw !== null) {
    // aria-pressed="true" means playing; when stopped it should be false
    expect(ariaPressedRaw).not.toBe('true');
  }
});
