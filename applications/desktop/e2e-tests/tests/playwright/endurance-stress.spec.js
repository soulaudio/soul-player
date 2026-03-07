/**
 * Endurance / long-running stress tests — Playwright CDP
 *
 * Uses 30-second and 15-second tracks to exercise sustained playback without
 * the constant auto-advance race conditions of 2-second tracks.
 *
 * Verifies:
 *   - Extended continuous playback (30s+) without position drift or freeze
 *   - Position reporting accuracy over sustained playback
 *   - UI responsiveness during long uninterrupted playback
 *   - Queue progression through many tracks with real elapsed time
 *   - Concurrent UI operations during sustained playback
 *   - Memory stability: no listener accumulation over extended sessions
 *   - Seek accuracy on longer tracks (large seek distances)
 *   - Pause/resume after extended playback periods
 *   - Crossfade toggle during sustained playback
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks × 2s WAV (existing)
 *   Album 2002 — "Long Album" — 5 tracks × 30s WAV (IDs 3001–3005)
 *   Album 2003 — "Marathon Album" — 10 tracks × 15s WAV (IDs 4001–4010)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// Increase default test timeout for long-running tests
test.setTimeout(120_000);

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

async function playAlbum(p, albumId) {
  await p.evaluate(async (aid) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: aid });
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
  }, albumId);
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function getPosition(p) {
  return p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
}

async function getPlaybackState(p) {
  return p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
}

async function getSidebarTitle(p) {
  return p.evaluate(() => {
    const container = document.querySelector('[data-testid="now-playing-title"]');
    if (!container) return null;
    const titleEl = container.querySelector('.text-sm');
    return titleEl ? titleEl.textContent.trim() : null;
  });
}

async function waitForSidebarTitle(p, expected, timeout = 30_000) {
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

// ================================================================
// Test 1: Sustained playback — play 30s track for 10s, verify position advances
// ================================================================

test('sustained playback: position advances correctly over 10 seconds', async () => {
  await playAlbum(page, 2002); // Long Album — 30s tracks
  await waitForSidebarTitle(page, 'Long One');

  // Seek to 0 for a clean start and wait for it to take effect
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(500);

  // Read position baseline — may not be exactly 0 due to async processing
  const pos1 = await getPosition(page);
  const t1 = Date.now();

  // Wait 5 seconds
  await page.waitForTimeout(5_000);

  const pos2 = await getPosition(page);

  // Wait another 5 seconds
  await page.waitForTimeout(5_000);

  const pos3 = await getPosition(page);
  const realElapsed = (Date.now() - t1) / 1000;

  // Position should be advancing — not stuck
  expect(pos2).toBeGreaterThan(pos1 + 2); // at least 2s of progress after 5s real time
  expect(pos3).toBeGreaterThan(pos2 + 2);
  // Position delta should roughly track real elapsed time (generous tolerance)
  const posDelta = pos3 - pos1;
  expect(posDelta).toBeGreaterThan(realElapsed * 0.5); // at least half of real time
  expect(posDelta).toBeLessThan(realElapsed * 2.5);    // no more than 2.5x real time

  // Still playing
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One'); // 30s track shouldn't have advanced
});

// ================================================================
// Test 2: UI responsive during sustained playback — navigate while playing
// ================================================================

test('UI stays responsive during 15s of continuous playback with navigation', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Navigate through all pages while audio is playing
  const navTargets = ['nav-tracks', 'nav-artists', 'nav-playlists', 'nav-home', 'nav-albums'];

  for (let round = 0; round < 3; round++) {
    for (const nav of navTargets) {
      await page.click(`[data-testid="${nav}"]`, { force: true });
      await page.waitForTimeout(500);
      await expect(page.locator(`[data-testid="${nav}"]`)).toBeVisible();
    }
  }

  // After all that navigation, playback should still be going
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');

  // Position should have advanced during navigation
  const pos = await getPosition(page);
  expect(pos).toBeGreaterThan(5);
});

// ================================================================
// Test 3: Seek accuracy on 30s track — seek to specific positions and verify
// ================================================================

test('seek accuracy on 30s track: seeking to 5 positions and reading back', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Pause first so position doesn't drift during verification
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('pause_playback')
  );
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  const seekTargets = [5.0, 15.0, 25.0, 10.0, 0.5];

  for (const target of seekTargets) {
    await page.evaluate(async (pos) =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: pos }), target
    );
    await page.waitForTimeout(300);

    const actual = await getPosition(page);
    // Position should be within ±2s of the target
    expect(actual).toBeGreaterThan(target - 2);
    expect(actual).toBeLessThan(target + 2);
  }

  // Resume and verify it still plays
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('resume_playback')
  );
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );
});

// ================================================================
// Test 4: Queue progression — auto-advance through 3 tracks of Marathon Album
// ================================================================

test('auto-advance through 3 marathon tracks (15s each) with position checks', async () => {
  await playAlbum(page, 2003); // Marathon Album — 15s tracks
  await waitForSidebarTitle(page, 'Marathon 01');

  // Seek near end of track 1
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );

  // Wait for auto-advance to Track 2
  await waitForSidebarTitle(page, 'Marathon 02', 30_000);

  // Verify position reset near 0 on new track
  await page.waitForTimeout(500);
  const pos1 = await getPosition(page);
  expect(pos1).toBeLessThan(5);

  // Seek near end of track 2
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );

  await waitForSidebarTitle(page, 'Marathon 03', 30_000);

  // Verify position reset
  await page.waitForTimeout(500);
  const pos2 = await getPosition(page);
  expect(pos2).toBeLessThan(5);

  // Seek near end of track 3
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );

  await waitForSidebarTitle(page, 'Marathon 04', 30_000);

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 5: Extended pause/resume — pause for 10s, resume, verify continuation
// ================================================================

test('pause for 10 seconds then resume: position stable and playback continues', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Play for 3s
  await page.waitForTimeout(3_000);

  const posBeforePause = await getPosition(page);
  expect(posBeforePause).toBeGreaterThan(1);

  // Pause
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  const posPaused1 = await getPosition(page);

  // Wait 10 seconds while paused
  await page.waitForTimeout(10_000);

  const posPaused2 = await getPosition(page);

  // Position should NOT have advanced while paused (within ±0.5s tolerance)
  expect(Math.abs(posPaused2 - posPaused1)).toBeLessThan(1.0);

  // Resume
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Wait 3s for playback to advance
  await page.waitForTimeout(3_000);

  const posAfterResume = await getPosition(page);
  // Position should have advanced from where it was paused
  expect(posAfterResume).toBeGreaterThan(posPaused2 + 1);

  // Still on same track (30s track with ~6s of actual playback)
  expect(await getSidebarTitle(page)).toBe('Long One');
});

// ================================================================
// Test 6: Sustained UI interaction — interact with UI every second for 20s
// ================================================================

test('sustained UI interaction for 20s: clicking controls every second', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  const actions = [
    // Volume changes
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 30 })),
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 70 })),
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })),
    // Navigation
    () => page.click('[data-testid="nav-tracks"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
    () => page.click('[data-testid="nav-artists"]', { force: true }),
    () => page.click('[data-testid="nav-home"]', { force: true }),
    () => page.click('[data-testid="nav-playlists"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
    // Seek operations
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 5.0 }); } catch {} }),
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 10.0 }); } catch {} }),
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 3.0 }); } catch {} }),
    // Volume again
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 80 })),
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 40 })),
    // More navigation
    () => page.click('[data-testid="nav-tracks"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
    () => page.click('[data-testid="nav-home"]', { force: true }),
    () => page.click('[data-testid="nav-artists"]', { force: true }),
    () => page.click('[data-testid="nav-playlists"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
  ];

  for (const action of actions) {
    await action();
    await page.waitForTimeout(1_000);
  }

  // After 20s of sustained interaction, playback should still be going
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');

  // UI still responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 7: Memory stability — count event listeners before and after extended session
// ================================================================

test('event listener count stable after 30s of playback and operations', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Count initial listeners
  const listenersBefore = await page.evaluate(() => {
    if (typeof window.__TAURI_INTERNALS__._listeners === 'object') {
      return Object.values(window.__TAURI_INTERNALS__._listeners).reduce(
        (sum, arr) => sum + (Array.isArray(arr) ? arr.length : 0), 0
      );
    }
    return -1;
  });

  // Do 15s of mixed operations: seek, navigate, volume, skip, back
  const ops = [
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 5.0 }); } catch {} }),
    () => page.click('[data-testid="nav-tracks"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 60 })),
    () => page.click('[data-testid="next-button"]'),
    () => page.click('[data-testid="previous-button"]'),
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 2.0 }); } catch {} }),
    () => page.click('[data-testid="nav-artists"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
    () => page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })),
    () => page.click('[data-testid="next-button"]'),
    () => page.click('[data-testid="previous-button"]'),
    () => page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 8.0 }); } catch {} }),
    () => page.click('[data-testid="nav-home"]', { force: true }),
    () => page.click('[data-testid="nav-albums"]', { force: true }),
  ];

  for (const op of ops) {
    await op();
    await page.waitForTimeout(1_000);
  }

  // Count listeners after
  const listenersAfter = await page.evaluate(() => {
    if (typeof window.__TAURI_INTERNALS__._listeners === 'object') {
      return Object.values(window.__TAURI_INTERNALS__._listeners).reduce(
        (sum, arr) => sum + (Array.isArray(arr) ? arr.length : 0), 0
      );
    }
    return -1;
  });

  // If measurable, listeners should not have grown significantly
  if (listenersBefore >= 0 && listenersAfter >= 0) {
    expect(listenersAfter).toBeLessThanOrEqual(listenersBefore + 5);
  }

  // Playback still healthy
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 8: Skip through 10-track queue — Marathon Album full progression
// ================================================================

test('skip through 8 marathon tracks and back: queue progression works', async () => {
  await playAlbum(page, 2003); // Marathon Album — 10 × 15s
  await waitForSidebarTitle(page, 'Marathon 01');

  // Skip forward through 7 tracks (stop at Marathon 08 — not the last track)
  const expectedTracks = [
    'Marathon 02', 'Marathon 03', 'Marathon 04', 'Marathon 05',
    'Marathon 06', 'Marathon 07', 'Marathon 08'
  ];

  for (const expected of expectedTracks) {
    // Seek to 0 before skipping to prevent auto-advance of current track
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(100);
    await page.click('[data-testid="next-button"]');
    await waitForSidebarTitle(page, expected);
  }

  expect(await getSidebarTitle(page)).toBe('Marathon 08');
  expect(await getPlaybackState(page)).toBe('Playing');

  // Seek to 0 then skip back
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  await page.click('[data-testid="previous-button"]');
  await waitForSidebarTitle(page, 'Marathon 07');

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 9: Seek + skip + pause/resume on long tracks — complex interaction chain
// ================================================================

test('complex chain: seek, skip, pause, resume across long tracks', async () => {
  await playAlbum(page, 2002); // Long Album — 30s tracks
  await waitForSidebarTitle(page, 'Long One');

  // Seek to 20s into the track
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 20.0 })
  );
  await page.waitForTimeout(500);

  let pos = await getPosition(page);
  expect(pos).toBeGreaterThan(18);

  // Skip to next track
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Long Two');

  // Position should reset
  await page.waitForTimeout(500);
  pos = await getPosition(page);
  expect(pos).toBeLessThan(5);

  // Play for 3 seconds
  await page.waitForTimeout(3_000);

  // Pause
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  const posBeforePause = await getPosition(page);

  // Seek while paused
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 15.0 })
  );
  await page.waitForTimeout(300);

  pos = await getPosition(page);
  expect(pos).toBeGreaterThan(13);
  expect(pos).toBeLessThan(17);

  // Resume
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Wait and verify position advances from seek point
  await page.waitForTimeout(2_000);
  pos = await getPosition(page);
  expect(pos).toBeGreaterThan(15);

  // Seek to start so previous-button goes to previous track (not restart current)
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Go back to previous track
  await page.click('[data-testid="previous-button"]');
  await waitForSidebarTitle(page, 'Long One');

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 10: Real-time auto-advance — let a 15s track play to completion
// ================================================================

test('real-time auto-advance: let marathon track play from 10s to completion', async () => {
  await playAlbum(page, 2003); // Marathon Album — 15s
  await waitForSidebarTitle(page, 'Marathon 01');

  // Seek to 10s in (only ~5s left)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 10.0 })
  );

  // Wait for auto-advance — should happen within ~8s
  await waitForSidebarTitle(page, 'Marathon 02', 15_000);

  // Verify position is near start of new track
  await page.waitForTimeout(500);
  const pos = await getPosition(page);
  expect(pos).toBeLessThan(5);

  // Let it play a bit to confirm stability
  await page.waitForTimeout(3_000);

  const pos2 = await getPosition(page);
  expect(pos2).toBeGreaterThan(pos);
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Marathon 02');
});

// ================================================================
// Test 11: Playlist operations during long playback
// ================================================================

test('playlist CRUD during sustained 30s track playback', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Play for 3s to establish stable playback
  await page.waitForTimeout(3_000);

  // Create a playlist
  const result = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('create_playlist', { name: 'Endurance Test' })
  );
  const playlistId = typeof result === 'object' && result !== null
    ? String(result.id || result.Id || result)
    : String(result);

  // Add long tracks to it
  for (let tid = 3001; tid <= 3005; tid++) {
    await page.evaluate(async (args) =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: args.pid,
        trackId: String(args.tid),
      }), { pid: playlistId, tid }
    );
  }

  // Verify tracks were added
  const tracks = await page.evaluate(async (pid) =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: pid }), playlistId
  );
  expect(tracks.length).toBe(5);

  // Navigate to playlists page while still playing
  await page.click('[data-testid="nav-playlists"]', { force: true });
  await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 10_000 });

  // Playback should still be going
  expect(await getPlaybackState(page)).toBe('Playing');

  // Navigate back
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForTimeout(500);

  // Clean up playlist
  await page.evaluate(async (pid) =>
    window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pid }), playlistId
  );

  // Still playing after all that
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');
});

// ================================================================
// Test 12: Rapid volume changes over 15 seconds — no audio glitches
// ================================================================

test('rapid volume sweeps over 15s: 30 changes without crash', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Sweep volume up and down 30 times over 15 seconds
  for (let i = 0; i < 30; i++) {
    const volume = i % 2 === 0 ? Math.min(100, (i * 7) % 100) : Math.max(0, 100 - (i * 7) % 100);
    await page.evaluate(async (v) =>
      window.__TAURI_INTERNALS__.invoke('set_volume', { volume: v }), volume
    );
    await page.waitForTimeout(500);
  }

  // Restore volume
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );

  // Still playing, no crash
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');

  // Position should have advanced during 15s of volume changes
  const pos = await getPosition(page);
  expect(pos).toBeGreaterThan(10);
});

// ================================================================
// Test 13: Crossfade toggle during sustained 30s playback
// ================================================================

test('crossfade toggling during sustained playback: 10 toggles over 10s', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  const curves = ['linear', 'square_root', 's_curve', 'equal_power', 'exponential'];

  for (let i = 0; i < 10; i++) {
    // Toggle crossfade on/off with different settings each time
    if (i % 2 === 0) {
      const curve = curves[i % curves.length];
      await page.evaluate(async (c) =>
        window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
          enabled: true, durationMs: 2000, curve: c,
        }), curve
      );
    } else {
      await page.evaluate(async () =>
        window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false })
      );
    }
    await page.waitForTimeout(1_000);
  }

  // Playback survived all the toggles
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');

  const pos = await getPosition(page);
  expect(pos).toBeGreaterThan(5);

  // Cleanup
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false }); } catch {}
  });
});
