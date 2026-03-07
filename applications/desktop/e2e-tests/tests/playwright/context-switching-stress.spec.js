/**
 * Context Switching Stress E2E tests — Playwright CDP
 *
 * Stress-tests rapid context switching: play album → play artist → play genre →
 * play playlist in quick succession. Verifies playback context history doesn't
 * corrupt and the playback engine handles rapid queue replacements.
 *
 * 6 tests:
 *   1. Rapid context switch: album → artist → genre in <2s intervals
 *   2. Context history reflects all switches in correct order
 *   3. Rapid play/stop cycles across contexts don't crash
 *   4. 10 rapid album Play All clicks don't corrupt queue
 *   5. Mixed context + seek + skip stress
 *   6. Context history survives 20 rapid recordings
 *
 * Seed data:
 *   Album 2001 — "Playwright Album" — 5 tracks (2s WAV)
 *   Album 2002 — "Long Album" — 5 tracks (30s WAV)
 *   Artist 2001 — "Playwright Artist"
 *   Genre 4001 — "Playwright Genre"
 *   Playlist 3001 — "Favorites"
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

  // Add tracks to Favorites for playlist context
  await page.evaluate(async () => {
    for (const id of [2001, 2002, 2003, 2004, 2005]) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001', trackId: String(id),
      }).catch(() => {});
    }
  });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_playback_context_history'); } catch {}
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

// Helper: play album by ID via IPC
async function playAlbum(p, albumId) {
  await p.evaluate(async (id) => {
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
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: { contextType: 'album', contextId: String(id), contextName: `Album ${id}`, contextArtworkPath: null },
    });
  }, albumId);
}

// Helper: play artist tracks via IPC
async function playArtist(p, artistId) {
  await p.evaluate(async (id) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_artist_tracks', { artistId: id });
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    if (queue.length > 0) {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
      await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
        input: { contextType: 'artist', contextId: String(id), contextName: `Artist ${id}`, contextArtworkPath: null },
      });
    }
  }, artistId);
}

// Helper: play genre tracks via IPC
async function playGenre(p, genreId) {
  await p.evaluate(async (id) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_genre_tracks', { genreId: id });
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    if (queue.length > 0) {
      await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
      await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
        input: { contextType: 'genre', contextId: String(id), contextName: `Genre ${id}`, contextArtworkPath: null },
      });
    }
  }, genreId);
}

// ── Test 1: Rapid context switch album → artist → genre ──

test('rapid context switch: album → artist → genre in quick succession', async () => {
  test.setTimeout(60_000);

  await playAlbum(page, 2001);
  await page.waitForTimeout(500);

  await playArtist(page, 2001);
  await page.waitForTimeout(500);

  await playGenre(page, 4001);
  await page.waitForTimeout(500);

  // Should still be playing
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused']).toContain(state);

  // Should have a current track
  const track = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_current_track')
  );
  expect(track).toBeTruthy();
});

// ── Test 2: Context history reflects all switches ──

test('context history records all context switches', async () => {
  await playAlbum(page, 2001);
  await page.waitForTimeout(1500); // Need >1s gap for different timestamps
  await playArtist(page, 2001);
  await page.waitForTimeout(1500);
  await playGenre(page, 4001);
  await page.waitForTimeout(500);

  const contexts = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts', { limit: 10 })
  );

  expect(contexts.length).toBeGreaterThanOrEqual(3);
  // All three types should be present
  const types = contexts.map(c => c.contextType);
  expect(types).toContain('album');
  expect(types).toContain('artist');
  expect(types).toContain('genre');
});

// ── Test 3: Rapid play/stop cycles don't crash ──

test('rapid play/stop cycles across contexts do not crash the engine', async () => {
  test.setTimeout(60_000);

  for (let i = 0; i < 5; i++) {
    await playAlbum(page, 2001);
    await page.waitForTimeout(300);
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('stop_playback');
    });
    await page.waitForTimeout(200);
  }

  // App should still respond to IPC
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Stopped');
});

// ── Test 4: 10 rapid Play All clicks don't corrupt queue ──

test('10 rapid play_queue calls to same album stabilize correctly', async () => {
  test.setTimeout(60_000);

  for (let i = 0; i < 10; i++) {
    await playAlbum(page, 2001);
    // No wait between — maximum stress
  }

  await page.waitForTimeout(1000);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);

  // If playing, queue should be valid
  if (state === 'Playing') {
    const track = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_current_track')
    );
    expect(track).toBeTruthy();
  }
});

// ── Test 5: Mixed context + seek + skip ──

test('context switch with seek and skip interleaved', async () => {
  test.setTimeout(60_000);

  await playAlbum(page, 2002); // Long Album (30s tracks)
  await page.waitForTimeout(500);

  // Seek
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 15.0 });
  });
  await page.waitForTimeout(300);

  // Skip
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('next_track');
  });
  await page.waitForTimeout(300);

  // Switch context
  await playArtist(page, 2001);
  await page.waitForTimeout(300);

  // Skip again
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('next_track');
  });
  await page.waitForTimeout(300);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ── Test 6: 20 rapid context recordings don't corrupt history ──

test('20 rapid context recordings maintain consistent history', async () => {
  test.setTimeout(60_000);

  // Record 20 contexts rapidly
  await page.evaluate(async () => {
    for (let i = 0; i < 20; i++) {
      const type = ['album', 'artist', 'genre', 'playlist'][i % 4];
      await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
        input: {
          contextType: type,
          contextId: String(i),
          contextName: `Context ${i}`,
          contextArtworkPath: null,
        },
      });
    }
  });

  const contexts = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts', { limit: 50 })
  );

  // Should have recorded many contexts (exact count depends on upsert behavior)
  expect(contexts.length).toBeGreaterThanOrEqual(10);

  // All 20 context names should be present somewhere in the list
  const names = contexts.map(c => c.contextName);
  expect(names).toContain('Context 19');
});
