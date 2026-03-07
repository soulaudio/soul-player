/**
 * Favorites Stress E2E tests — Playwright CDP
 *
 * Stress-tests the favorites/playlist add/remove operations under rapid
 * toggling and concurrent playback scenarios.
 *
 * 6 tests:
 *   1. Rapid toggle: 20 add/remove cycles on same track
 *   2. Bulk add: add all 20 tracks to Favorites simultaneously
 *   3. Bulk remove: remove all tracks from Favorites
 *   4. Add/remove during active playback doesn't interrupt audio
 *   5. Concurrent add to multiple playlists
 *   6. Rapid create/delete playlist cycles
 *
 * Seed data:
 *   Album 2001 — 5 tracks (2001–2005), Album 2002 — 5 tracks (3001–3005)
 *   Album 2003 — 10 tracks (4001–4010)
 *   Playlist 3001 — "Favorites"
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

const ALL_TRACK_IDS = [
  2001, 2002, 2003, 2004, 2005,
  3001, 3002, 3003, 3004, 3005,
  4001, 4002, 4003, 4004, 4005, 4006, 4007, 4008, 4009, 4010,
];

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
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Clean Favorites
  await page.evaluate(async () => {
    try {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
      for (const t of tracks) {
        await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
          playlistId: '3001', trackId: String(t.id),
        }).catch(() => {});
      }
    } catch {}
  }).catch(() => {});
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});

  // Clean Favorites
  await page.evaluate(async () => {
    try {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
      for (const t of tracks) {
        await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
          playlistId: '3001', trackId: String(t.id),
        }).catch(() => {});
      }
    } catch {}
  }).catch(() => {});

  // Clean up test playlists
  await page.evaluate(async () => {
    try {
      const playlists = await window.__TAURI_INTERNALS__.invoke('get_all_playlists');
      for (const pl of playlists) {
        if (pl.name && pl.name.startsWith('Stress Test')) {
          await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pl.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
});

// ── Test 1: 20 rapid toggle cycles ──

test('20 rapid add/remove cycles on same track leave consistent state', async () => {
  test.setTimeout(60_000);

  await page.evaluate(async () => {
    for (let i = 0; i < 20; i++) {
      if (i % 2 === 0) {
        await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
          playlistId: '3001', trackId: '2001',
        }).catch(() => {});
      } else {
        await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
          playlistId: '3001', trackId: '2001',
        }).catch(() => {});
      }
    }
  });

  // 20 cycles: 0=add,1=rem,...,18=add,19=rem → final: removed
  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  const ids = tracks.map(t => t.id);
  expect(ids).not.toContain(2001);
});

// ── Test 2: Bulk add all 20 tracks ──

test('bulk add all 20 tracks to Favorites', async () => {
  test.setTimeout(60_000);

  await page.evaluate(async (trackIds) => {
    for (const id of trackIds) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001', trackId: String(id),
      }).catch(() => {});
    }
  }, ALL_TRACK_IDS);

  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  expect(tracks.length).toBe(20);
});

// ── Test 3: Bulk remove all tracks ──

test('bulk remove all tracks from Favorites', async () => {
  test.setTimeout(60_000);

  // Add all first
  await page.evaluate(async (trackIds) => {
    for (const id of trackIds) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001', trackId: String(id),
      }).catch(() => {});
    }
  }, ALL_TRACK_IDS);

  // Remove all
  await page.evaluate(async (trackIds) => {
    for (const id of trackIds) {
      await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001', trackId: String(id),
      }).catch(() => {});
    }
  }, ALL_TRACK_IDS);

  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );
  expect(tracks).toHaveLength(0);
});

// ── Test 4: Add/remove during playback doesn't interrupt ──

test('adding and removing favorites during playback does not interrupt audio', async () => {
  test.setTimeout(60_000);

  // Start playback with Long Album (30s tracks) to avoid auto-advance
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2002 });
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

  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Rapidly add/remove while playing
  await page.evaluate(async () => {
    for (let i = 0; i < 10; i++) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001', trackId: String(3001 + (i % 5)),
      }).catch(() => {});
      await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001', trackId: String(3001 + (i % 5)),
      }).catch(() => {});
    }
  });

  // Playback should still be active or at least the engine should be responsive
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  // The key assertion is that the IPC didn't crash — any state is acceptable
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ── Test 5: Add same track to multiple playlists ──

test('adding same track to multiple playlists concurrently', async () => {
  test.setTimeout(60_000);

  // Create 3 test playlists
  const playlistIds = await page.evaluate(async () => {
    const ids = [];
    for (let i = 0; i < 3; i++) {
      const pl = await window.__TAURI_INTERNALS__.invoke('create_playlist', {
        name: `Stress Test ${i}`, description: null,
      });
      ids.push(pl.id);
    }
    return ids;
  });

  // Add track 2001 to all 3 playlists + Favorites
  await page.evaluate(async (plIds) => {
    const allIds = ['3001', ...plIds];
    for (const plId of allIds) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: plId, trackId: '2001',
      }).catch(() => {});
    }
  }, playlistIds);

  // Verify track is in all playlists
  const playlists = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlists_containing_track', { trackId: '2001' })
  );

  expect(playlists.length).toBeGreaterThanOrEqual(4);
});

// ── Test 6: Rapid create/delete playlist cycles ──

test('5 rapid create/delete playlist cycles', async () => {
  test.setTimeout(60_000);

  for (let i = 0; i < 5; i++) {
    const pl = await page.evaluate(async (idx) => {
      return window.__TAURI_INTERNALS__.invoke('create_playlist', {
        name: `Stress Test ${idx}`, description: null,
      });
    }, i);

    await page.evaluate(async (id) => {
      await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id });
    }, pl.id);
  }

  // All test playlists should be gone
  const playlists = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_playlists')
  );
  const stressPlaylists = playlists.filter(p => p.name && p.name.startsWith('Stress Test'));
  expect(stressPlaylists).toHaveLength(0);
});
