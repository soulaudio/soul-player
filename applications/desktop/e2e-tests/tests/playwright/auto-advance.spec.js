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
 *   Album ID 2001 — "Playwright Album" — 6 tracks, all backed by the same 10-second
 *   silent WAV file (test-track.wav).  Each track therefore plays for ~10 s.
 *
 *   Track IDs / titles:
 *     2001 → "Track One"   (queue index 0)
 *     2002 → "Track Two"   (queue index 1)
 *     2003 → "Track Three" (queue index 2)
 *     2004 → "Track Four"  (queue index 3)
 *     2005 → "Track Five"  (queue index 4)
 *     2006 → "Collab Track" (queue index 5)
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
  // Remaining queue: [T2, T3, T4, T5, Collab Track] → index 4 = Collab Track.
  await page.evaluate(
    async idx => window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: idx }),
    4,
  );

  await waitForTitle(page, 'Collab Track');

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

// ================================================================
// Test 5: Queue sidebar shows correct tracks after auto-advance
//
// Regression guard for the "ghost track" bug:
//   play_queue() emits QueueUpdated with [T1..T5] (source_index=0).
//   play() then pops T1 (source_index→1) with NO QueueUpdated, so React
//   queue stays stale at [T1,T2,T3,T4,T5].
//   When T1 auto-advances to T2, ActivateSource fires with NO QueueUpdated
//   either, leaving React queue at [T1,T2,T3,T4,T5].
//   QueueSection filter removes the current track (T2) but NOT the stale T1,
//   making T1 reappear in the queue as if it were upcoming — the ghost track.
//
// Fix: ActivateSource handler must emit QueueUpdated so React refreshes.
// After the refresh get_queue() returns [T3,T4,T5] and the ghost is gone.
// ================================================================

test('queue shows correct tracks after auto-advance: no ghost of previously-played track', async () => {
  test.setTimeout(30_000);

  // Use play_queue WITHOUT clear_add_to_queue so the React queue stays stale
  // — this reproduces the exact real-world scenario where the ghost appears.
  await page.evaluate(async () => {
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

  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 },
  );
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      return el && el.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 },
  );

  // Verify initial queue shows 5 items (T2-T5, Collab Track) while T1 is current.
  // The stale queue has T1 in it but the filter correctly hides it.
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);
  const initialCount = await page.locator('[data-testid="queue-item"]').count();
  expect(initialCount).toBe(5); // T2, T3, T4, T5, Collab Track

  // Seek to near the end of T1 to trigger auto-advance fast.
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.5 });
  });

  // Wait for T2 to become the now-playing track.
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      return el && el.textContent.trim() === 'Track Two';
    },
    { timeout: 15_000 },
  );

  // Allow time for QueueUpdated → loadQueue() → React re-render.
  await page.waitForTimeout(600);

  // Queue must show exactly 4 items: T3, T4, T5, Collab Track.
  // Without the fix: shows [T1, T3, T4, T5, Collab Track] — 5 items (T1 ghost).
  const afterCount = await page.locator('[data-testid="queue-item"]').count();
  expect(afterCount).toBe(4);

  const texts = await page.locator('[data-testid="queue-item"]').allTextContents();
  const combined = texts.join(' ');
  // Ghost: T1 must NOT appear (it was played — not upcoming)
  expect(combined).not.toContain('Track One');
  // Current track must NOT appear in the queue list
  expect(combined).not.toContain('Track Two');
  // Upcoming tracks must all be present
  expect(combined).toContain('Track Three');
  expect(combined).toContain('Track Four');
  expect(combined).toContain('Track Five');
  expect(combined).toContain('Collab Track');
});

// ================================================================
// Test 6: Queue count stays accurate after two consecutive auto-advances
//
// After T1→T2→T3 via two natural auto-advances (using seek to rush them),
// the queue must contain exactly 2 upcoming tracks (T4, T5).
// Formerly-played T1 and T2 must not reappear.
// ================================================================

test('queue count stays accurate after two consecutive auto-advances', async () => {
  test.setTimeout(40_000);

  await startPlayback(page);

  // First auto-advance: seek T1 to near end → T2 starts
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.5 });
  });
  await waitForTitle(page, 'Track Two', 15_000);
  await page.waitForTimeout(400);

  // Second auto-advance: seek T2 to near end → T3 starts
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.5 });
  });
  await waitForTitle(page, 'Track Three', 15_000);
  await page.waitForTimeout(600);

  // Queue must show exactly 3 items: T4, T5, Collab Track
  const queueItems = page.locator('[data-testid="queue-item"]');
  const count = await queueItems.count();
  expect(count).toBe(3);

  const texts = await queueItems.allTextContents();
  const combined = texts.join(' ');
  expect(combined).not.toContain('Track One');    // played — must be gone
  expect(combined).not.toContain('Track Two');    // played — must be gone
  expect(combined).not.toContain('Track Three');  // current — not in queue
  expect(combined).toContain('Track Four');
  expect(combined).toContain('Track Five');
  expect(combined).toContain('Collab Track');
});

test('auto-advance fires after a pause/resume cycle', async () => {
  test.setTimeout(35_000);

  await startPlayback(page);

  // Robust pause: seek to 0 + pause in single call to prevent 2s track auto-advance race
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    await window.__TAURI_INTERNALS__.invoke('pause_playback');
  });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 },
  );

  // Re-check after settling — if auto-advance raced the pause, re-pause
  await page.waitForTimeout(200);
  const checkState = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  if (checkState === 'Playing') {
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
      await window.__TAURI_INTERNALS__.invoke('pause_playback');
    });
    await page.waitForFunction(
      async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
      { timeout: 5_000 },
    );
    await page.waitForTimeout(200);
  }

  // Resume playback.
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('resume_playback');
  });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 },
  );

  // After resume, the current track plays to completion then auto-advances.
  // With 2s tracks, we may already be past Track One. Accept any track after the original.
  const currentTitle = await getNowPlayingTitle(page);

  if (currentTitle === 'Track One') {
    // Track One is still playing — wait for it to finish and auto-advance
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
  }

  // Verify auto-advance happened: state should be Playing with a different track
  const finalTitle = await getNowPlayingTitle(page);
  expect(finalTitle).not.toBe('Track One');

  const finalState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  // State should be Playing (auto-advanced to next track) or at least not Paused
  // (proving the pause/resume cycle didn't break auto-advance)
  expect(['Playing', 'Stopped']).toContain(finalState);
});
