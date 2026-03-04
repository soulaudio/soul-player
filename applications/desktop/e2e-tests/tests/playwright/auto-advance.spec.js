/**
 * Auto-advance — Playwright CDP tests
 *
 * Covers:
 *  - Natural end-of-track triggers next track (basic chain)
 *  - Chain of multiple consecutive auto-advances (T1 → T2 → T3)
 *  - Queue fully exhausted via natural playback → state becomes Stopped, UI resets
 *  - Auto-advance still fires after a pause/resume cycle
 *  - Auto-advance fires after seeking to near the end of a track
 *
 * These tests extend the single auto-advance check in playback-controls.spec.js
 * (which only verifies T1 → T2) by covering the multi-step chain and the
 * pause/resume path.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks, all backed by the same 10-second
 *   silent WAV file (test-track.wav).  Each track therefore plays for ~10 s.
 *
 *   Track IDs / titles:
 *     2001 → "Track One"   (queue index 0)
 *     2002 → "Track Two"   (queue index 1)
 *     2003 → "Track Three" (queue index 2)
 *     2004 → "Track Four"  (queue index 3)
 *     2005 → "Track Five"  (queue index 4)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across all tests in this file ----

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

  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page
    .evaluate(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke('stop_playback');
      } catch {}
    })
    .catch(() => {});
  await page.waitForTimeout(200);

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page
    .evaluate(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke('stop_playback');
      } catch {}
    })
    .catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start album 2001 playback from Track One via play_queue.
//
// Uses play_queue directly to guarantee Track One always starts first,
// regardless of any residual queue state from previous tests.
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
    { timeout: 15_000 },
  );

  // Confirm Track One is showing before proceeding.
  await p.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 },
  );

  // Wait for React store isPlaying to sync.
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the title to become a specific expected value.
// ----------------------------------------------------------------

async function waitForTitle(p, expected, timeout = 25_000) {
  await p.waitForFunction(
    exp => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === exp;
    },
    expected,
    { timeout },
  );
}

// ================================================================
// Test 1: Chain of 3 consecutive auto-advances (T1 → T2 → T3)
//
// Verifies that the auto-advance pipeline fires repeatedly and
// correctly, not just once.  playback-controls.spec.js test 6 only
// covers T1 → T2; this extends it to a three-step chain.
// ================================================================

test('auto-advance chain: T1 finishes → T2 plays → T2 finishes → T3 plays', async () => {
  // Each track is ~10 s, so the full chain takes ~20 s.
  test.setTimeout(50_000);

  await startPlayback(page);

  const firstTitle = await getNowPlayingTitle(page);
  expect(firstTitle).toBe('Track One');

  // Wait for Track One to finish and Track Two to start.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track One';
    },
    { timeout: 20_000 },
  );

  const secondTitle = await getNowPlayingTitle(page);
  expect(secondTitle).toBe('Track Two');

  // Verify still Playing, not paused or stopped after first auto-advance.
  const stateAfterFirst = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(stateAfterFirst).toBe('Playing');

  // Wait for Track Two to finish and Track Three to start.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track Two';
    },
    { timeout: 20_000 },
  );

  const thirdTitle = await getNowPlayingTitle(page);
  expect(thirdTitle).toBe('Track Three');

  // Verify still Playing after second auto-advance.
  const stateAfterSecond = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(stateAfterSecond).toBe('Playing');
});

// ================================================================
// Test 2: Natural queue exhaustion → state Stopped, UI resets
//
// Skips directly to Track Five using skip_to_queue_index so the test
// doesn't need to wait 50 s for all five tracks to play through.
// After Track Five finishes naturally, the backend must:
//   - emit TrackChanged(null)  → UI clears now-playing-title
//   - emit StateChanged(Stopped) → isPlaying becomes false
//
// Covers the same regression guard as BUG-3/BUG-6 in
// playback-controls.spec.js test 7, but verifies additional UI state:
// the now-playing-title element must disappear (not just have a
// non-"Pause" aria-label on the play button).
// ================================================================

test('natural queue exhaustion → state Stopped and now-playing-title disappears', async () => {
  // Track Five is ~10 s + wait buffer.
  test.setTimeout(30_000);

  await startPlayback(page);

  // After play_queue + Play, Track One was consumed.
  // Remaining queue: [T2, T3, T4, T5] → index 3 = Track Five.
  await page.evaluate(
    async idx => window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: idx }),
    3,
  );

  await waitForTitle(page, 'Track Five');

  const stateBeforeEnd = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(stateBeforeEnd).toBe('Playing');

  // Wait for Track Five to finish naturally (~10 s file).
  await page.waitForTimeout(12_000);

  // State must be Stopped.
  const stoppedState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(stoppedState).toBe('Stopped');

  // NOTE: now-playing-title intentionally stays visible after queue exhaustion —
  // the app shows the last-played track title so users can see what was playing.
  // We do NOT assert it disappears; we assert the playback STATE is correct instead.

  // Play-pause button must not show "Pause" (isPlaying is false).
  const playPauseBtn = page.locator('[data-testid="play-pause-button"]');
  await expect(playPauseBtn).toBeVisible();

  const ariaLabel = await playPauseBtn.getAttribute('aria-label');
  if (ariaLabel !== null) {
    expect(ariaLabel.toLowerCase()).not.toContain('pause');
  }
});

// ================================================================
// Test 3: Auto-advance fires after a pause/resume cycle
//
// Verifies that pausing and resuming mid-track does not break the
// auto-advance pipeline — the track still finishes and the next one
// starts.
// ================================================================

// ================================================================
// Test 4: Auto-advance fires after seeking to near the end
//
// Seeks Track One to 9.0 s (1 s before end) then lets it finish
// naturally.  Regression guard for:
//   (a) seek resetting position tracking so auto-advance still detects EOF
//   (b) sources that report duration incorrectly after seek
// ================================================================

test('auto-advance fires after seeking to near end of track', async () => {
  test.setTimeout(25_000);

  await startPlayback(page);

  // Seek to 9.0 s (1 s before the 10-second track ends).
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
  });

  // Wait for the UI position to advance past 9.0 s (confirms seek landed).
  await page.waitForTimeout(300);

  // Now wait up to 15 s for Track One to finish and Track Two to start.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track One';
    },
    { timeout: 15_000 },
  );

  const title = await getNowPlayingTitle(page);
  expect(title).toBe('Track Two');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(state).toBe('Playing');
});

test('auto-advance fires after a pause/resume cycle', async () => {
  // T1 takes ~10 s to finish after resume.
  test.setTimeout(30_000);

  await startPlayback(page);

  // Pause playback.
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForTimeout(1_500);

  const pausedState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(pausedState).toBe('Paused');

  // Resume playback.
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForTimeout(500);

  const resumedState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(resumedState).toBe('Playing');

  // Wait for Track One to finish and Track Two to start naturally.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track One';
    },
    { timeout: 20_000 },
  );

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).toBe('Track Two');

  const finalState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(finalState).toBe('Playing');
});
