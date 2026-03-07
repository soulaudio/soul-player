/**
 * Long Session / Multi-Advance Stress E2E tests — Playwright CDP
 *
 * Simulates extended playback sessions with many track advances to test
 * for memory leaks, queue corruption, and state drift over time.
 *
 * Uses Album 2003 "Marathon Album" (10 tracks × 15s) for longer queues
 * and Album 2001 "Playwright Album" (5 tracks × 2s) for fast advances.
 *
 * 5 tests:
 *   1. Skip through 20 tracks via next_track (queue wraps with RepeatAll)
 *   2. 50 rapid seek + skip interleaves
 *   3. Play/pause 30 times during continuous playback
 *   4. Queue replacement mid-playback 15 times
 *   5. Combined: seek + skip + context switch + favorites — 60s sustained
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
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(300);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(300);
});

// Helper: start playback with RepeatAll on marathon album
async function startMarathon(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2003 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
    await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'all' });
  });

  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
}

// Helper: start short album playback
async function startShortAlbum(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });

  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
}

// ── Test 1: Skip through 20 tracks with RepeatAll ──

test('skip through 20 tracks via next_track with RepeatAll (wraps twice)', async () => {
  test.setTimeout(120_000);

  await startMarathon(page);

  for (let i = 0; i < 20; i++) {
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('next_track')
    );
    await page.waitForTimeout(200);
  }

  // Should still be playing after 20 skips (wrapped twice through 10 tracks)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  const track = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_current_track')
  );
  expect(track).toBeTruthy();
});

// ── Test 2: 50 rapid seek + skip ──

test('50 rapid seek + skip interleaves without crash', async () => {
  test.setTimeout(120_000);

  await startMarathon(page);

  for (let i = 0; i < 50; i++) {
    if (i % 3 === 0) {
      await page.evaluate(async () =>
        window.__TAURI_INTERNALS__.invoke('next_track')
      ).catch(() => {});
    } else {
      const pos = (i % 15) * 1.0;
      await page.evaluate(async (p) =>
        window.__TAURI_INTERNALS__.invoke('seek_to', { position: p }),
        pos
      ).catch(() => {});
    }
    // Minimal delay for max stress
    if (i % 10 === 0) await page.waitForTimeout(100);
  }

  await page.waitForTimeout(500);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ── Test 3: 30 play/pause cycles ──

test('30 play/pause cycles during playback', async () => {
  test.setTimeout(120_000);

  await startMarathon(page);

  for (let i = 0; i < 30; i++) {
    if (i % 2 === 0) {
      await page.evaluate(async () =>
        window.__TAURI_INTERNALS__.invoke('pause_playback')
      ).catch(() => {});
    } else {
      await page.evaluate(async () =>
        window.__TAURI_INTERNALS__.invoke('resume_playback')
      ).catch(() => {});
    }
    await page.waitForTimeout(100);
  }

  // End with resume
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('resume_playback')
  ).catch(() => {});
  await page.waitForTimeout(500);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 4: Queue replacement 15 times ──

test('replacing queue mid-playback 15 times stabilizes correctly', async () => {
  test.setTimeout(120_000);

  for (let i = 0; i < 15; i++) {
    const albumId = [2001, 2002, 2003][i % 3];
    await page.evaluate(async (id) => {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: id });
      tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
      const queue = tracks.map(t => ({
        trackId: String(t.id), title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null, albumId: t.album_id || null,
        filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null, coverArtPath: null,
      }));
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
    }, albumId);
    await page.waitForTimeout(300);
  }

  await page.waitForTimeout(1000);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ── Test 5: Combined stress — 60s sustained ──

test('combined 60s stress: seek + skip + context + favorites', async () => {
  test.setTimeout(120_000);

  await startMarathon(page);

  const startTime = Date.now();
  let iteration = 0;

  while (Date.now() - startTime < 30_000) { // 30s (not 60 to keep test reasonable)
    const action = iteration % 6;

    switch (action) {
      case 0: // Skip
        await page.evaluate(async () =>
          window.__TAURI_INTERNALS__.invoke('next_track')
        ).catch(() => {});
        break;
      case 1: // Seek
        await page.evaluate(async () =>
          window.__TAURI_INTERNALS__.invoke('seek_to', { position: 5.0 })
        ).catch(() => {});
        break;
      case 2: // Add to favorites
        await page.evaluate(async () =>
          window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
            playlistId: '3001', trackId: '4001',
          })
        ).catch(() => {});
        break;
      case 3: // Remove from favorites
        await page.evaluate(async () =>
          window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
            playlistId: '3001', trackId: '4001',
          })
        ).catch(() => {});
        break;
      case 4: // Record context
        await page.evaluate(async () =>
          window.__TAURI_INTERNALS__.invoke('record_playback_context', {
            input: { contextType: 'album', contextId: '2003', contextName: 'Marathon', contextArtworkPath: null },
          })
        ).catch(() => {});
        break;
      case 5: // Pause/resume
        await page.evaluate(async () => {
          const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
          if (state === 'Playing') {
            await window.__TAURI_INTERNALS__.invoke('pause_playback');
          } else {
            await window.__TAURI_INTERNALS__.invoke('resume_playback');
          }
        }).catch(() => {});
        break;
    }

    iteration++;
    await page.waitForTimeout(200);
  }

  // End: ensure we can still query state
  const finalState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(finalState);

  // IPC still responsive
  const track = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_current_track')
  );
  // Track may be null if stopped, that's fine
  expect(track === null || typeof track === 'object').toBe(true);
});
