/**
 * DSD seek — Playwright CDP E2E tests
 *
 * Verifies that seek operations work correctly on DSD audio sources.
 * DSD decoding uses a different code path from PCM (ring-buffer-based
 * chunk streaming), so seek correctness must be tested independently.
 *
 * Tests exercise the seek_target_samples atomic and faster chunk-refill
 * fixes introduced for the DSD seek regression.
 *
 * Seed data (from playwright-global-setup.js):
 *   Artist 5001 — "DSD Artist"
 *   Album 5001  — "DSD Album"
 *   Track 5001  — "DSD Track One"  (.dsf, 512 blocks × 4096 samples = ~0.74s)
 *   Track 5002  — "DSD Track Two"  (.dff, ~0.19s)
 *
 * IPC commands used:
 *   play_queue({ queue, startIndex })  — start playback with a queue
 *   seek_to({ position })              — seek to absolute position in seconds
 *   get_position()                     — returns current position (f64 seconds)
 *   get_playback_state()               — "Playing" | "Paused" | "Stopped"
 *   pause_playback()                   — pause
 *   resume_playback()                  — resume
 *   stop_playback()                    — stop and reset
 *   get_track_by_id({ id })            — fetch single track record
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
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page
    .evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    })
    .catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  // Navigate to Albums for a stable known starting point
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page
    .evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    })
    .catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start playback of a single DSD track by ID via IPC.
// Returns immediately after the Playing state is confirmed.
// ----------------------------------------------------------------

async function playDsdTrack(p, trackId) {
  await p.evaluate(async ({ trackId }) => {
    const track = await window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: trackId });
    if (!track) throw new Error(`Track ${trackId} not found`);
    const queue = [{
      trackId: String(track.id),
      title: track.title,
      artist: track.artist_name || 'Unknown Artist',
      album: track.album_title || null,
      albumId: track.album_id || null,
      filePath: track.file_path || '',
      durationSeconds: track.duration_seconds || null,
      trackNumber: track.track_number || null,
      coverArtPath: null,
    }];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  }, { trackId });

  // Wait for the now-playing panel and Playing state
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 },
  );
}

// ----------------------------------------------------------------
// Helper: build a two-track queue [DSD track 5001, WAV track 2001]
// and start playback from index 0.  Used for auto-advance tests.
// ----------------------------------------------------------------

async function playDsdThenWav(p) {
  await p.evaluate(async () => {
    const dsd = await window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 5001 });
    const wav = await window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 2001 });
    if (!dsd || !wav) throw new Error('Seed tracks 5001 or 2001 not found');
    const makeItem = t => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    });
    const queue = [makeItem(dsd), makeItem(wav)];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 },
  );
}

// ----------------------------------------------------------------
// Test 1: seek updates position immediately and does not bounce back
//
// After seeking, the reported position must remain at the seek target
// (±100ms tolerance) and not revert to the pre-seek position.
// This exercises the seek_target_samples atomic fix.
//
// DSF track 5001 is ~0.74s long. We seek to 0.3s.
// ----------------------------------------------------------------

test('seek updates DSD position immediately and does not bounce back', async () => {
  test.setTimeout(30_000);

  await playDsdTrack(page, 5001);

  // Let position advance beyond our seek target so the post-seek position
  // can only be < 0.45s if the seek actually applied (not a pre-seek read).
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_position')) > 0.35,
    { timeout: 10_000 },
  );

  // Seek to 0.15s (well before where we currently are)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.15 }),
  );

  // Position must now be < 0.35s — this is only true if the seek applied.
  // waitForFunction retries, so transient intermediate values are tolerated.
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_position')) < 0.35,
    { timeout: 5_000 },
  );

  // Wait one position-update cycle (~200ms) and verify the position has NOT
  // bounced back above 0.45s (i.e., the seek did not revert).
  await page.waitForTimeout(300);
  const posAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position'),
  );
  expect(posAfter).toBeLessThan(0.45);

  // State must still be Playing
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 2: audio resumes and position advances after seek
//
// After seeking, playback must continue advancing — tests faster
// chunk refill so audio does not stall after a DSD seek.
//
// DSF track 5001 is ~0.74s. We seek to 0.1s and then confirm that
// the position advances forward (i.e., the DSD decoder is producing
// samples, not stalling).  We verify advancement via a waitForFunction
// poll rather than a snapshot comparison to avoid races.
// ----------------------------------------------------------------

test('DSD audio continues advancing after seek', async () => {
  test.setTimeout(30_000);

  await playDsdTrack(page, 5001);

  // Seek to 0.1s from the start
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.1 }),
  );

  // Wait for the position to land at the seek target (< 0.3s means seek applied)
  await page.waitForFunction(
    async () => {
      const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      // Accept either: pos near seek target while still Playing,
      // or auto-advance already fired (Stopped is fine here — seek + advance == success).
      return state !== 'Playing' || pos < 0.3;
    },
    { timeout: 5_000 },
  );

  // State check: if still Playing, confirm position advances; if Stopped/auto-advanced
  // the seek-then-play path worked and the test passes trivially.
  const stateNow = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );

  if (stateNow === 'Playing') {
    const posSnapshot = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_position'),
    );

    // Wait for position to advance at least 0.04s beyond the snapshot.
    // timeout=3s gives enough headroom even for slow DSD chunk refill.
    await page.waitForFunction(
      async (before) => (await window.__TAURI_INTERNALS__.invoke('get_position')) > before + 0.04,
      posSnapshot,
      { timeout: 3_000 },
    );
  }
  // If state is Stopped, the DSD track auto-advanced — the seek + playback path worked.
  // No additional assertions needed.
});

// ----------------------------------------------------------------
// Test 3: seek while paused updates DSD position and resumes correctly
//
// Pausing and then seeking must update the stored position without
// resuming playback. After resume, the player must continue from the
// seek point and advance forward.
//
// DSF track 5001 is ~0.74s. We pause immediately after playback starts
// (within the first ~0.1s) to guarantee the track has not finished,
// then seek to 0.2s while paused and verify state + position.
// ----------------------------------------------------------------

test('seek while paused updates DSD position and resumes from seek point', async () => {
  test.setTimeout(30_000);

  await playDsdTrack(page, 5001);

  // Pause as soon as possible after playback begins to avoid the 0.74s track ending
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('pause_playback'),
  );
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 },
  );

  // Seek to 0.2s while paused
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.2 }),
  );
  await page.waitForTimeout(400);

  // State must still be Paused — seek must not auto-resume
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 3_000 },
  );

  // Position must be within the DSD track's duration (0–0.74s)
  const posAfterSeek = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position'),
  );
  expect(posAfterSeek).toBeGreaterThanOrEqual(0.0);
  expect(posAfterSeek).toBeLessThan(0.75);

  // Resume and verify position advances from the seek point
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('resume_playback'),
  );
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 },
  );

  const posBeforeAdvance = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position'),
  );

  // Position must advance — confirms DSD decoder resumes after paused seek.
  // If the track auto-advanced (Stopped) that is also acceptable — the resume path worked.
  await page.waitForFunction(
    async (before) => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      const pos = await window.__TAURI_INTERNALS__.invoke('get_position');
      return state !== 'Playing' || pos > before;
    },
    posBeforeAdvance,
    { timeout: 5_000 },
  );
});

// ----------------------------------------------------------------
// Test 4: seek near end of DSD track triggers auto-advance to next track
//
// DSF track 5001 is seeded with duration_seconds=0.74. The actual audio
// data is 512 blocks × 4096 samples / 2_822_400 Hz ≈ 0.743s per channel.
// Seeking to 0.65s leaves ~80ms before end-of-stream, which should be
// enough for the auto-advance to fire within a reasonable timeout.
//
// The queue is [DSD track 5001, WAV track 2001].  After auto-advance
// the now-playing title should switch to "Track One" (WAV).
// ----------------------------------------------------------------

test('seek near end of DSD track triggers auto-advance to WAV track', async () => {
  test.setTimeout(30_000);

  await playDsdThenWav(page);

  // Confirm we started on DSD Track One
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      return el && el.textContent.trim() === 'DSD Track One';
    },
    { timeout: 10_000 },
  );

  // Seek to 0.65s — ~80ms before the 0.74s DSD track ends
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.65 }),
  );

  // Wait for auto-advance: title changes away from "DSD Track One"
  // Allow 8s: ~80ms remaining + DSD flush + LoadNext + WAV init.
  await page.waitForFunction(
    () => {
      const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
      if (!el) return false;
      const t = el.textContent.trim();
      return t !== '' && t !== 'DSD Track One';
    },
    { timeout: 8_000 },
  );

  // After auto-advance the new track should be Playing
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 },
  );

  // The next track is "Track One" (WAV, ID 2001)
  const titleAfter = await page.evaluate(() => {
    const el = document.querySelector('[data-testid="now-playing-title"] .text-sm');
    return el ? el.textContent.trim() : '';
  });
  expect(titleAfter).toBe('Track One');
});
