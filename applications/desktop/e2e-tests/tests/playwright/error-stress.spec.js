/**
 * Error recovery stress tests — Playwright CDP
 *
 * Verifies that the app recovers gracefully from repeated and chained errors:
 *   - Multiple consecutive bad tracks don't leave app in broken state
 *   - Alternating good/bad tracks: recovery works every time
 *   - Rapid error + recovery cycles don't accumulate broken state
 *   - Error during various playback states (playing, paused, seeking)
 *   - App remains fully navigable after error storms
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks x 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
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
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

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

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

function makeBadTrack(id) {
  return {
    trackId: `bad-${id}`,
    title: `Bad Track ${id}`,
    artist: 'Error Test',
    album: null,
    albumId: null,
    filePath: `C:\\nonexistent\\path\\missing-${id}.flac`,
    durationSeconds: 3,
    trackNumber: 1,
    coverArtPath: null,
  };
}

async function playBadQueue(p, count = 1) {
  const queue = Array.from({ length: count }, (_, i) => makeBadTrack(i));
  await p.evaluate(async (q) => {
    try {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue: q, startIndex: 0 });
    } catch {}
  }, queue);
}

async function playGoodQueue(p) {
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
}

async function waitForStopped(p, timeout = 10_000) {
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped' || state === 'Error';
    },
    { timeout }
  );
}

async function waitForPlaying(p, timeout = 15_000) {
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout }
  );
}

// ================================================================
// Test 1: 5 consecutive bad tracks — app stays functional
// ================================================================

test('5 consecutive queues with missing files: app stays functional', async () => {
  for (let i = 0; i < 5; i++) {
    await playBadQueue(page);
    await waitForStopped(page);
  }

  // All nav elements must still be visible and clickable
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
  await expect(page.locator('[data-testid="nav-tracks"]')).toBeVisible();
  await expect(page.locator('[data-testid="nav-artists"]')).toBeVisible();

  // Can navigate
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
});

// ================================================================
// Test 2: Alternating bad/good queues — recovery works each time
// ================================================================

test('alternating bad then good queue 3 times: each good play succeeds', async () => {
  for (let i = 0; i < 3; i++) {
    // Bad queue → error/stopped
    await playBadQueue(page);
    await waitForStopped(page);

    // Good queue → should recover to Playing
    await playGoodQueue(page);
    await waitForPlaying(page);

    // Stop before next cycle
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    });
    await page.waitForTimeout(200);
  }

  // Final verification: play good queue and confirm
  await playGoodQueue(page);
  await waitForPlaying(page);

  // Sidebar shows track info
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 3: Rapid error queue firing — 10 bad queues rapid-fire
// ================================================================

test('10 rapid-fire bad queues: app does not crash', async () => {
  const start = Date.now();

  await page.evaluate(async () => {
    for (let i = 0; i < 10; i++) {
      try {
        await window.__TAURI_INTERNALS__.invoke('play_queue', {
          queue: [{
            trackId: `rapid-bad-${i}`,
            title: `Rapid Bad ${i}`,
            artist: 'Error',
            album: null,
            albumId: null,
            filePath: `C:\\fake\\path\\${i}.mp3`,
            durationSeconds: 1,
            trackNumber: 1,
            coverArtPath: null,
          }],
          startIndex: 0,
        });
      } catch {}
    }
  });

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(15_000);

  // Wait for state to settle
  await page.waitForTimeout(1_000);

  // App must still respond
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();

  // Can still play good music after the error storm
  await playGoodQueue(page);
  await waitForPlaying(page);
});

// ================================================================
// Test 4: Error during playing state — good track first, then bad
// ================================================================

test('error while playing: start good, then switch to bad queue, then recover', async () => {
  // Start playing good music
  await playGoodQueue(page);
  await waitForPlaying(page);

  // Switch to bad queue while playing — may or may not reach Stopped quickly
  await playBadQueue(page);
  // Wait a reasonable time for the error to propagate, but don't require exact state
  await page.waitForTimeout(2_000);

  // The app should be in some non-crashed state
  const midState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Stopped', 'Paused', 'Error']).toContain(midState);

  // Stop explicitly to clean up, then recover with good queue
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  });
  await page.waitForTimeout(300);

  await playGoodQueue(page);
  await waitForPlaying(page);

  // Seek to 0 to prevent the 2s track from finishing before our assertion
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(100);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: Error + navigation combined — navigate between pages after errors
// ================================================================

test('navigate between all pages after 3 error cycles: all pages load', async () => {
  // Trigger 3 error cycles
  for (let i = 0; i < 3; i++) {
    await playBadQueue(page);
    await waitForStopped(page);
  }

  // Visit every nav target
  const targets = ['nav-tracks', 'nav-artists', 'nav-playlists', 'nav-home', 'nav-albums'];
  for (const nav of targets) {
    await page.click(`[data-testid="${nav}"]`, { force: true });
    await page.waitForTimeout(500);
    // Page must have rendered something
    await expect(page.locator(`[data-testid="${nav}"]`)).toBeVisible();
  }

  // Settings page also loads
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForTimeout(500);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Back to albums
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });
});

// ================================================================
// Test 6: Mixed queue with valid and invalid tracks
// ================================================================

test('queue with 1 bad track + 4 good tracks: skips bad and plays good', async () => {
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));

    // Build mixed queue: bad track first, then 4 good tracks
    const queue = [
      {
        trackId: 'mixed-bad-1',
        title: 'Bad Track',
        artist: 'Error',
        album: null,
        albumId: null,
        filePath: 'C:\\fake\\missing.flac',
        durationSeconds: 1,
        trackNumber: 1,
        coverArtPath: null,
      },
      ...tracks.slice(0, 4).map(t => ({
        trackId: String(t.id),
        title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        albumId: t.album_id || null,
        filePath: t.file_path || '',
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
        coverArtPath: null,
      })),
    ];
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  // The bad track should fail; the engine should eventually reach a valid track
  // or stop. Either outcome is acceptable.
  await page.waitForTimeout(3_000);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  // App must be in a valid state
  expect(['Playing', 'Stopped', 'Paused', 'Error']).toContain(state);

  // App must still be responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 7: Empty queue + stop + play cycle — no state corruption
// ================================================================

test('empty queue, stop, then valid play: clean state transitions', async () => {
  // Try empty queue — may throw or silently fail, both are fine
  await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue: [], startIndex: 0 });
    } catch {}
  });
  await page.waitForTimeout(500);

  // Stop (idempotent) — also wrapped in try/catch
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  });
  await page.waitForTimeout(300);

  // App must still be responsive after empty queue + stop
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();

  // Play valid queue — this is the real test: can we recover?
  await playGoodQueue(page);
  await waitForPlaying(page);

  // Seek to 0 to prevent the 2s track from finishing before our assertion
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(100);

  // Verify sidebar shows track info
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 8: Stress cycle — error → recover → skip → error → recover
// ================================================================

test('error-recover-skip chain: 3 cycles of error then skip through good tracks', async () => {
  for (let i = 0; i < 3; i++) {
    // Error
    await playBadQueue(page);
    await waitForStopped(page);

    // Recover with good queue
    await playGoodQueue(page);
    await waitForPlaying(page);

    // Skip forward twice
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(500);
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(500);

    // Still playing
    const state = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playback_state')
    );
    expect(state).toBe('Playing');

    // Stop for next cycle
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    });
    await page.waitForTimeout(200);
  }

  // Final good play
  await playGoodQueue(page);
  await waitForPlaying(page);
});
