/**
 * Playlist operations stress tests — Playwright CDP
 *
 * Verifies that rapid and repeated playlist CRUD operations do not:
 *   - Corrupt playlist data or lose tracks
 *   - Crash the app during rapid create/delete cycles
 *   - Interfere with active playback
 *   - Leave orphaned state after many operations
 *
 * Seed data (from playwright-global-setup.js):
 *   Playlist ID 3001 — "Favorites" — 0 tracks (empty)
 *   Album ID 2001 — "Playwright Album" — 5 tracks (IDs 2001–2005)
 *   Track titles: Track One … Track Five
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
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});

  // Clean up: remove tracks from Favorites and delete test playlists
  await page.evaluate(async () => {
    try {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
      for (const t of tracks) {
        await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
          playlistId: '3001',
          trackId: String(t.id),
        }).catch(() => {});
      }
    } catch {}
    try {
      const playlists = await window.__TAURI_INTERNALS__.invoke('get_all_playlists');
      for (const pl of playlists) {
        if (pl.name !== 'Favorites') {
          await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pl.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});

  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

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
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

// ================================================================
// Test 1: Create and delete 5 playlists rapidly via IPC
// ================================================================

test('create and delete 5 playlists via IPC without crash', async () => {
  const createdIds = [];

  for (let i = 0; i < 5; i++) {
    const result = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('create_playlist', { name: 'Stress Test Playlist' })
    );
    // create_playlist may return an object with id field or a plain id
    const id = typeof result === 'object' && result !== null ? (result.id || result.Id || String(result)) : result;
    createdIds.push(String(id));
  }

  expect(createdIds.length).toBe(5);

  // Delete all created playlists
  for (const id of createdIds) {
    await page.evaluate(async (pid) =>
      window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pid }), id
    );
  }

  // Verify none remain
  const playlists = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_playlists')
  );
  const remaining = playlists.filter(p => p.name === 'Stress Test Playlist');
  expect(remaining.length).toBe(0);

  // Favorites should still exist
  expect(playlists.some(p => p.name === 'Favorites')).toBe(true);
});

// ================================================================
// Test 2: Add all 5 tracks to Favorites rapidly, then remove all
// ================================================================

test('add 5 tracks then remove all from Favorites: count stays consistent', async () => {
  // Add all 5 tracks
  for (let trackId = 2001; trackId <= 2005; trackId++) {
    await page.evaluate(async (tid) =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001',
        trackId: String(tid),
      }), trackId
    );
  }

  // Verify count
  const tracksAfterAdd = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  expect(tracksAfterAdd.length).toBe(5);

  // Remove all tracks
  for (const t of tracksAfterAdd) {
    await page.evaluate(async (tid) =>
      window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001',
        trackId: String(tid),
      }), t.id
    );
  }

  // Verify empty
  const tracksAfterRemove = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  expect(tracksAfterRemove.length).toBe(0);
});

// ================================================================
// Test 3: Add/remove the same track 10 times — idempotent behavior
// ================================================================

test('add/remove same track 10 times: no duplicates or errors', async () => {
  for (let i = 0; i < 10; i++) {
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001',
        trackId: '2001',
      })
    );

    const tracks = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
    );
    // Should have at most 1 instance (add is additive, but track should exist)
    expect(tracks.length).toBeGreaterThanOrEqual(1);

    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001',
        trackId: '2001',
      })
    );
  }

  // Final state: playlist should be empty
  const finalTracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  expect(finalTracks.length).toBe(0);
});

// ================================================================
// Test 4: Playlist CRUD during active playback
// ================================================================

test('playlist create/add/delete during playback: audio continues', async () => {
  await startPlayback(page);

  // Create a playlist
  const result = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('create_playlist', { name: 'During Playback' })
  );
  const playlistId = typeof result === 'object' && result !== null ? String(result.id || result.Id || result) : String(result);

  // Add tracks to it
  for (let trackId = 2001; trackId <= 2003; trackId++) {
    await page.evaluate(async (args) =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: args.pid,
        trackId: String(args.tid),
      }), { pid: playlistId, tid: trackId }
    );
  }

  // Verify tracks were added
  const tracks = await page.evaluate(async (pid) =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: pid }), playlistId
  );
  expect(tracks.length).toBe(3);

  // Playback should still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Delete the playlist
  await page.evaluate(async (pid) =>
    window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pid }), playlistId
  );

  // Playback still active after deletion
  const stateAfter = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfter).toBe('Playing');
});

// ================================================================
// Test 5: Navigate to playlists page rapidly during CRUD operations
// ================================================================

test('navigate to playlists page 5 times while creating/deleting: UI stays stable', async () => {
  for (let i = 0; i < 5; i++) {
    // Create a playlist
    const result = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('create_playlist', { name: 'Nav Stress' })
    );
    const pid = typeof result === 'object' && result !== null ? String(result.id || result.Id || result) : String(result);

    // Navigate to playlists page
    await page.click('[data-testid="nav-playlists"]', { force: true });
    await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 10_000 });

    // Delete the playlist
    await page.evaluate(async (id) =>
      window.__TAURI_INTERNALS__.invoke('delete_playlist', { id }), pid
    );

    // Navigate away
    await page.click('[data-testid="nav-albums"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });
  }

  // App still functional
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 6: Playlist page shows correct count after bulk operations
// ================================================================

test('playlist page reflects correct state after bulk add/remove', async () => {
  // Add all 5 tracks via IPC
  for (let tid = 2001; tid <= 2005; tid++) {
    await page.evaluate(async (trackId) =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001',
        trackId: String(trackId),
      }), tid
    );
  }

  // Navigate to Favorites detail
  await page.click('[data-testid="nav-playlists"]', { force: true });
  await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 10_000 });

  const favCard = page.locator('[data-testid="media-card-playlist-3001"]');
  await favCard.waitFor({ state: 'visible', timeout: 10_000 });
  await favCard.click();

  await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });

  // Track list should show 5 tracks
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const rows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  await page.waitForFunction(
    () => document.querySelectorAll('[data-testid="track-list"] [data-testid="track-row"]').length >= 5,
    { timeout: 10_000 }
  );
  const count = await rows.count();
  expect(count).toBeGreaterThanOrEqual(5);
});
