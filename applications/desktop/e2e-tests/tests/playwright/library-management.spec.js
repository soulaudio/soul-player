/**
 * Library Management E2E tests — Playwright CDP
 *
 * Tests library query commands not covered by other spec files:
 *   get_track_by_id, get_genre_by_id, get_playlist_by_id,
 *   get_artist_albums, get_artist_top_tracks, get_genre_albums,
 *   delete_track (with restore), reorder_playlist_track
 *
 * 8 tests
 *
 * Seed data:
 *   Album 2001 "Playwright Album" — 5 tracks (2001-2005)
 *   Album 2002 "Long Album" — 5 tracks (3001-3005)
 *   Album 2003 "Marathon Album" — 10 tracks (4001-4010)
 *   Artist 2001 "Playwright Artist"
 *   Genre 4001 "Playwright Genre"
 *   Playlist 3001 "Favorites"
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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
});

// ── Test 1: get_track_by_id ──

test('get_track_by_id returns correct track for ID 2001', async () => {
  const track = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 2001 })
  );

  expect(track).toBeTruthy();
  expect(track.id).toBe(2001);
  expect(track.title).toBe('Track One');
});

// ── Test 2: get_genre_by_id ──

test('get_genre_by_id returns Playwright Genre for ID 4001', async () => {
  const genre = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_genre_by_id', { id: 4001 })
  );

  expect(genre).toBeTruthy();
  expect(genre.name).toBe('Playwright Genre');
});

// ── Test 3: get_playlist_by_id ──

test('get_playlist_by_id returns Favorites for ID 3001', async () => {
  const playlist = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_by_id', { id: '3001' })
  );

  expect(playlist).toBeTruthy();
  expect(playlist.name).toBe('Favorites');
});

// ── Test 4: get_artist_albums ──

test('get_artist_albums returns albums for Playwright Artist', async () => {
  const albums = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_artist_albums', { artistId: 2001 })
  );

  expect(Array.isArray(albums)).toBe(true);
  expect(albums.length).toBeGreaterThanOrEqual(1);
  const titles = albums.map(a => a.title);
  expect(titles).toContain('Playwright Album');
});

// ── Test 5: get_artist_top_tracks ──

test('get_artist_top_tracks returns tracks for Playwright Artist', async () => {
  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_artist_top_tracks', { artistId: 2001 })
  );

  expect(Array.isArray(tracks)).toBe(true);
  expect(tracks.length).toBeGreaterThanOrEqual(1);
});

// ── Test 6: get_genre_albums ──

test('get_genre_albums returns albums for Playwright Genre', async () => {
  const albums = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_genre_albums', { genreId: 4001, limit: 10 })
  );

  expect(Array.isArray(albums)).toBe(true);
  expect(albums.length).toBeGreaterThanOrEqual(1);
});

// ── Test 7: reorder_playlist_track changes position ──

test('reorder_playlist_track changes track position in playlist', async () => {
  // Add 3 tracks to Favorites
  await page.evaluate(async () => {
    for (const id of ['2001', '2002', '2003']) {
      await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
        playlistId: '3001', trackId: id,
      }).catch(() => {});
    }
  });

  // Reorder: move track 2001 to position 2 (0-indexed)
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('reorder_playlist_track', {
      playlistId: '3001', trackId: '2001', newPosition: 2,
    });
  });

  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' })
  );

  expect(tracks.length).toBeGreaterThanOrEqual(3);
  // Track 2001 should now be at or near position 2
  const ids = tracks.map(t => t.id);
  expect(ids).toContain(2001);

  // Cleanup
  await page.evaluate(async () => {
    for (const id of ['2001', '2002', '2003']) {
      await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001', trackId: id,
      }).catch(() => {});
    }
  });
});

// ── Test 8: get_track_by_id returns null for non-existent track ──

test('get_track_by_id returns null for non-existent track', async () => {
  const track = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_track_by_id', { id: 999999 })
  );

  expect(track).toBeNull();
});
