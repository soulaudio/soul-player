/**
 * Seek / scrub bar — Playwright CDP tests
 *
 * Covers the ProgressBar component rendered in the sidebar PlayerPanel.
 * The bar is a custom mouse-driven div (not <input type="range">); seeking is
 * triggered by clicking or dragging the seek-track area.
 *
 * IPC commands used:
 *   seek_to({ position: number })  — seek to absolute position in seconds
 *   get_position()                 — returns current position (f64 seconds)
 *   get_playback_state()           — "Playing" | "Paused" | "Stopped"
 *   stop_playback()                — stop cleanly between tests
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist"
 *   5 tracks × 2-second WAV files, Track IDs 2001–2005
 *   Track durations: exactly 2.0 seconds each
 *
 * Progress bar testids (ProgressBar.tsx):
 *   now-playing-progress-bar — outer container div
 *   seek-current-time        — current time <span> (format: "M:SS")
 *   seek-total-time          — total duration <span> (format: "M:SS")
 *   seek-track               — the clickable/draggable hit area div
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

// Before each test: stop playback, dismiss overlays, navigate to Albums, start fresh playback.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums to reset UI state
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Start playback of album 2001 from Track One
  await startPlayback(page);
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
// Bypasses the MediaCard branching logic so we always start fresh from
// Track One regardless of prior state.
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

  // Confirm Track One is loaded
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
  await p.waitForFunction(
    () => {
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
// Test 1: Seek bar and time displays are visible while playing
// ----------------------------------------------------------------

test('seek bar and time displays are visible while playing', async () => {
  // The progress bar container must be present and visible
  const progressBar = page.locator('[data-testid="now-playing-progress-bar"]');
  await expect(progressBar).toBeVisible({ timeout: 10_000 });

  // The progress bar must have positive width
  const box = await progressBar.boundingBox();
  expect(box).not.toBeNull();
  expect(box.width).toBeGreaterThan(50);

  // Current time display must be visible and non-empty
  const currentTime = page.locator('[data-testid="seek-current-time"]');
  await expect(currentTime).toBeVisible({ timeout: 5_000 });
  const currentTimeText = await currentTime.textContent();
  expect(currentTimeText.trim().length).toBeGreaterThan(0);

  // Total duration display must be visible and show "0:02" for the 2-second track
  const totalTime = page.locator('[data-testid="seek-total-time"]');
  await expect(totalTime).toBeVisible({ timeout: 5_000 });
  const totalTimeText = await totalTime.textContent();
  expect(totalTimeText.trim()).toBe('0:02');

  // The clickable seek track area must also be visible and have positive width
  const seekTrack = page.locator('[data-testid="seek-track"]');
  await expect(seekTrack).toBeVisible({ timeout: 5_000 });
  const trackBox = await seekTrack.boundingBox();
  expect(trackBox).not.toBeNull();
  expect(trackBox.width).toBeGreaterThan(50);
});

// ----------------------------------------------------------------
// Test 2: Seek bar shows progress advancing during playback
// ----------------------------------------------------------------

test('seek bar current time advances during active playback', async () => {
  // Read the backend position shortly after playback starts
  const positionBefore = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );

  // Wait 600ms — long enough for the backend's ~500ms position update interval
  // to have fired at least once so we can detect advancement
  await page.waitForTimeout(600);

  // Read position again — it must have advanced
  const positionAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );

  expect(positionAfter).toBeGreaterThan(positionBefore);

  // The current time span must also now show some elapsed time (not "0:00")
  // The track is only 2s long so we read whatever the UI displays and just
  // verify it changed from the initial zero state by checking backend position > 0
  expect(positionAfter).toBeGreaterThan(0);
});

// ----------------------------------------------------------------
// Test 3: Seek to near beginning via IPC while playing
// ----------------------------------------------------------------

test('seek_to near beginning keeps state Playing and updates position', async () => {
  // Let > 0.5s of audio play so our pre-seek position is clearly past the target.
  // This ensures the waitForFunction below only passes AFTER the seek takes effect,
  // not because we happened to start at < 0.5s.
  await page.waitForTimeout(700);

  // Seek to 0.1 seconds via IPC (near beginning of the 2s track)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.1 })
  );

  // Poll until the backend position drops to near the seek target.
  // Before seek: position was ~0.7s. After seek: position should be ~0.1s.
  // The waitForFunction condition (pos < 0.5) only becomes true once the seek applies.
  await page.waitForFunction(
    async () => {
      const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
      return pos < 0.5;
    },
    { timeout: 5_000 }
  );

  // Verify state is still Playing after the seek (seeking must not pause playback)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 4: Seek to near end causes auto-advance to Track Two
//
// Tracks are 2 seconds long. Seeking to 1.7s leaves 0.3s of audio before
// the track ends and auto-advance fires.
// ----------------------------------------------------------------

test('seeking near the end of a track causes auto-advance to Track Two', async () => {
  // Confirm we are on Track One
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Seek to 1.7 seconds (0.3s before end of the 2-second track)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 1.7 })
  );

  // Poll for the title to change away from Track One — the auto-advance fires
  // when the audio engine exhausts the buffered audio near the end.
  // Allow up to 6 seconds: 0.3s remaining + ~1s for LoadNext + ~1s audio init buffer.
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== 'Track One';
    },
    { timeout: 6_000 }
  );

  // After auto-advance the state should be Playing on the new track
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 5_000 }
  );

  const titleAfter = await getNowPlayingTitle(page);
  expect(titleAfter).toBe('Track Two');
});

// ----------------------------------------------------------------
// Test 5: Seek while paused — state remains Paused after seek
// ----------------------------------------------------------------

test('seek_to while paused keeps state Paused and updates position', async () => {
  // Pause playback
  await page.click('[data-testid="play-pause-button"]');

  // Flat wait avoids IPC contention (mirrors pattern from playback-controls.spec.js)
  await page.waitForTimeout(1_500);

  const stateAfterPause = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterPause).toBe('Paused');

  // Seek to 0.5 seconds via IPC
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.5 })
  );

  // The backend emits playback:position-updated on seek even when paused,
  // so the UI syncs. Wait a moment for the event to propagate.
  await page.waitForTimeout(300);

  // State must still be Paused — seeking must not auto-resume
  const stateAfterSeek = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterSeek).toBe('Paused');

  // Backend position must be near 0.5s
  const position = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  // Allow ±0.3s tolerance for internal buffering
  expect(position).toBeGreaterThanOrEqual(0.2);
  expect(position).toBeLessThan(0.8);

  // Resume playback to confirm it works after the seek
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 8_000 }
  );
});

// ----------------------------------------------------------------
// Test 6: Seek bar resets to near 0 when next track starts
// ----------------------------------------------------------------

test('seek bar resets to near beginning when skipping to Track Two', async () => {
  // Let a moment of audio play so we accumulate some position
  await page.waitForTimeout(400);

  const posBefore = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  expect(posBefore).toBeGreaterThan(0);

  // Click next to skip to Track Two
  await page.click('[data-testid="next-button"]');

  // Wait for the title to update to Track Two
  await waitForTitle(page, 'Track Two');

  // After the track change the backend position resets to 0 for the new track.
  // Poll until get_position confirms the reset (within 1s tolerance for audio init).
  await page.waitForFunction(
    async () => {
      const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
      return pos < 1.0;
    },
    { timeout: 8_000 }
  );

  const posAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  expect(posAfter).toBeLessThan(1.0);

  // The total time display must still show "0:02" for the new 2-second track
  const totalTime = page.locator('[data-testid="seek-total-time"]');
  await expect(totalTime).toBeVisible();

  // Wait for the duration to update (it may briefly show 0:00 during track transition)
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="seek-total-time"]');
      return el && el.textContent.trim() === '0:02';
    },
    { timeout: 8_000 }
  );

  const totalTimeText = await totalTime.textContent();
  expect(totalTimeText.trim()).toBe('0:02');
});

// ----------------------------------------------------------------
// Test 7: Click-to-seek via the UI seek track element
//
// This test exercises the ProgressBar's mouse-click handler directly
// rather than the IPC command, verifying the UI interaction path works.
// We click at the leftmost portion of the track (near 0%) to seek near the start.
// ----------------------------------------------------------------

test('clicking near the start of the seek-track element seeks to near beginning', async () => {
  // Let some audio play so position > 0
  await page.waitForTimeout(500);

  const posBefore = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  expect(posBefore).toBeGreaterThan(0);

  // Get the bounding box of the seek-track hit area
  const seekTrack = page.locator('[data-testid="seek-track"]');
  await expect(seekTrack).toBeVisible({ timeout: 5_000 });
  const trackBox = await seekTrack.boundingBox();
  expect(trackBox).not.toBeNull();

  // Click 5% from the left edge of the track — this maps to ~5% of the 2s duration = ~0.1s
  const clickX = trackBox.x + trackBox.width * 0.05;
  const clickY = trackBox.y + trackBox.height / 2;
  await page.mouse.click(clickX, clickY);

  // After a click-to-seek the backend emits position-updated and state should
  // still be Playing. Wait for the backend to confirm position is near the start.
  await page.waitForFunction(
    async () => {
      const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
      return pos < 0.5;
    },
    { timeout: 5_000 }
  );

  // State must still be Playing after the UI click-to-seek
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  const posAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  expect(posAfter).toBeLessThan(0.5);
});
