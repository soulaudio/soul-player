/**
 * Concurrent IPC Stress E2E tests — Playwright CDP
 *
 * Stress-tests firing multiple IPC commands simultaneously to verify
 * the backend handles concurrent access without crashes, deadlocks,
 * or data corruption.
 *
 * 8 tests
 *
 * Seed data:
 *   Album 2001 — 5 tracks, Album 2002 — 5 tracks, Album 2003 — 10 tracks
 *   Artist 2001, Genre 4001, Playlist 3001
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
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
});

// ── Test 1: Parallel library queries ──

test('5 library queries fired simultaneously all resolve correctly', async () => {
  const results = await page.evaluate(async () => {
    const [tracks, albums, artists, genres, playlists] = await Promise.all([
      window.__TAURI_INTERNALS__.invoke('get_all_tracks'),
      window.__TAURI_INTERNALS__.invoke('get_all_albums'),
      window.__TAURI_INTERNALS__.invoke('get_all_artists'),
      window.__TAURI_INTERNALS__.invoke('get_all_genres'),
      window.__TAURI_INTERNALS__.invoke('get_all_playlists'),
    ]);
    return {
      tracks: tracks.length,
      albums: albums.length,
      artists: artists.length,
      genres: genres.length,
      playlists: playlists.length,
    };
  });

  expect(results.tracks).toBeGreaterThanOrEqual(20);
  expect(results.albums).toBeGreaterThanOrEqual(3);
  expect(results.artists).toBeGreaterThanOrEqual(1);
  expect(results.genres).toBeGreaterThanOrEqual(1);
  expect(results.playlists).toBeGreaterThanOrEqual(1);
});

// ── Test 2: Parallel entity lookups ──

test('10 parallel get_*_by_id calls resolve without errors', async () => {
  const results = await page.evaluate(async () => {
    const promises = [
      window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 2001 }),
      window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 2002 }),
      window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 3001 }),
      window.__TAURI_INTERNALS__.invoke('get_album_by_id', { id: 2001 }),
      window.__TAURI_INTERNALS__.invoke('get_album_by_id', { id: 2002 }),
      window.__TAURI_INTERNALS__.invoke('get_artist_by_id', { id: 2001 }),
      window.__TAURI_INTERNALS__.invoke('get_genre_by_id', { id: 4001 }),
      window.__TAURI_INTERNALS__.invoke('get_playlist_by_id', { id: '3001' }),
      window.__TAURI_INTERNALS__.invoke('get_tracks_by_ids', { trackIds: [2001, 2003, 2005] }),
      window.__TAURI_INTERNALS__.invoke('check_database_health'),
    ];
    return Promise.all(promises);
  });

  expect(results).toHaveLength(10);
  expect(results[0]).toBeTruthy(); // track 2001
  expect(results[3]).toBeTruthy(); // album 2001
});

// ── Test 3: Concurrent playback state queries during playback ──

test('concurrent get_playback_state + get_position + get_current_track during playback', async () => {
  test.setTimeout(30_000);

  // Start playback
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

  // Fire concurrent state queries 10 times
  const results = await page.evaluate(async () => {
    const rounds = [];
    for (let i = 0; i < 10; i++) {
      rounds.push(
        Promise.all([
          window.__TAURI_INTERNALS__.invoke('get_playback_state'),
          window.__TAURI_INTERNALS__.invoke('get_position'),
          window.__TAURI_INTERNALS__.invoke('get_current_track'),
        ])
      );
    }
    return Promise.all(rounds);
  });

  expect(results).toHaveLength(10);
  // Each result should be [state, position, track]
  for (const [state, position, track] of results) {
    expect(['Playing', 'Paused', 'Stopped']).toContain(state);
  }
});

// ── Test 4: Parallel playlist operations ──

test('concurrent add/remove on different playlists does not corrupt data', async () => {
  test.setTimeout(30_000);

  // Create 3 test playlists
  const plIds = await page.evaluate(async () => {
    const ids = [];
    for (let i = 0; i < 3; i++) {
      const pl = await window.__TAURI_INTERNALS__.invoke('create_playlist', {
        name: `Concurrent Test ${i}`, description: null,
      });
      ids.push(pl.id);
    }
    return ids;
  });

  // Add tracks to all 3 playlists concurrently
  await page.evaluate(async (ids) => {
    await Promise.all(ids.map(plId =>
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: plId, trackId: '2001',
      })
    ));
  }, plIds);

  // Verify all have the track
  const counts = await page.evaluate(async (ids) => {
    const results = await Promise.all(ids.map(plId =>
      window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: plId })
    ));
    return results.map(t => t.length);
  }, plIds);

  for (const count of counts) {
    expect(count).toBeGreaterThanOrEqual(1);
  }

  // Cleanup
  await page.evaluate(async (ids) => {
    for (const id of ids) {
      await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id }).catch(() => {});
    }
  }, plIds);
});

// ── Test 5: Library queries + playback control mixed ──

test('library queries concurrent with playback commands do not deadlock', async () => {
  test.setTimeout(30_000);

  // Start playback
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

  // Fire mixed commands concurrently
  const result = await page.evaluate(async () => {
    const [tracks, albums, state, position, volume] = await Promise.all([
      window.__TAURI_INTERNALS__.invoke('get_all_tracks'),
      window.__TAURI_INTERNALS__.invoke('get_all_albums'),
      window.__TAURI_INTERNALS__.invoke('get_playback_state'),
      window.__TAURI_INTERNALS__.invoke('get_position'),
      window.__TAURI_INTERNALS__.invoke('get_volume'),
    ]);
    return { trackCount: tracks.length, albumCount: albums.length, state };
  });

  expect(result.trackCount).toBeGreaterThanOrEqual(20);
  expect(result.albumCount).toBeGreaterThanOrEqual(3);
  expect(['Playing', 'Paused', 'Stopped']).toContain(result.state);
});

// ── Test 6: Rapid settings reads + writes concurrent ──

test('concurrent settings reads and writes do not corrupt values', async () => {
  // Write 5 settings concurrently
  await page.evaluate(async () => {
    await Promise.all([
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'test.concurrent.1', value: 'a' }),
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'test.concurrent.2', value: 'b' }),
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'test.concurrent.3', value: 'c' }),
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'test.concurrent.4', value: 'd' }),
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'test.concurrent.5', value: 'e' }),
    ]);
  });

  // Read them all concurrently
  const values = await page.evaluate(async () => {
    const [v1, v2, v3, v4, v5] = await Promise.all([
      window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'test.concurrent.1' }),
      window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'test.concurrent.2' }),
      window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'test.concurrent.3' }),
      window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'test.concurrent.4' }),
      window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: 'test.concurrent.5' }),
    ]);
    return [v1, v2, v3, v4, v5];
  });

  expect(values).toEqual(['a', 'b', 'c', 'd', 'e']);
});

// ── Test 7: 20 concurrent context recordings ──

test('20 concurrent record_playback_context calls do not crash', async () => {
  await page.evaluate(async () => {
    const promises = [];
    for (let i = 0; i < 20; i++) {
      promises.push(
        window.__TAURI_INTERNALS__.invoke('record_playback_context', {
          input: {
            contextType: ['album', 'artist', 'genre', 'playlist'][i % 4],
            contextId: String(i),
            contextName: `Concurrent ${i}`,
            contextArtworkPath: null,
          },
        }).catch(() => {})
      );
    }
    await Promise.all(promises);
  });

  const contexts = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts', { limit: 50 })
  );

  // Some should have been recorded (exact count depends on upsert/timing)
  expect(contexts.length).toBeGreaterThanOrEqual(1);
});

// ── Test 8: Concurrent playback + queue + playlist during active play ──

test('concurrent queue and playlist operations during playback do not deadlock', async () => {
  test.setTimeout(30_000);

  // Start playback
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

  // Fire queue + playlist + state queries all at once
  await page.evaluate(async () => {
    await Promise.all([
      window.__TAURI_INTERNALS__.invoke('get_queue'),
      window.__TAURI_INTERNALS__.invoke('get_playback_state'),
      window.__TAURI_INTERNALS__.invoke('get_current_track'),
      window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', { playlistId: '3001', trackId: '3001' }).catch(() => {}),
      window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' }),
      window.__TAURI_INTERNALS__.invoke('get_all_tracks'),
    ]);
  });

  // App should still be responsive
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);

  // Cleanup
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
      playlistId: '3001', trackId: '3001',
    }).catch(() => {});
  });
});
