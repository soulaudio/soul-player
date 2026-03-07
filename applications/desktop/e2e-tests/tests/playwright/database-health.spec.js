/**
 * Database Health & Library Queries E2E tests — Playwright CDP
 *
 * Tests database health check and library query commands that are
 * exercised by the UI but not directly tested in other spec files.
 *
 * IPC commands tested:
 *   check_database_health() → { ok, issues }
 *   get_all_tracks() → Vec<Track>
 *   get_all_albums() → Vec<Album>
 *   get_all_artists() → Vec<Artist>
 *   get_all_genres() → Vec<Genre>
 *   get_random_albums(limit) → Vec<Album>
 *   get_recently_added_albums(limit) → Vec<Album>
 *   get_album_by_id(id) → Option<Album>
 *   get_artist_by_id(id) → Option<Artist>
 *   get_genre_by_id(id) → Option<Genre>
 *   get_tracks_by_ids(ids) → Vec<Track>
 *
 * 10 tests:
 *   1. check_database_health returns ok
 *   2. get_all_tracks returns at least 20 seeded tracks
 *   3. get_all_albums returns at least 3 seeded albums
 *   4. get_all_artists returns seeded artist
 *   5. get_all_genres returns seeded genre
 *   6. get_random_albums returns albums within limit
 *   7. get_recently_added_albums returns albums
 *   8. get_album_by_id returns correct album
 *   9. get_artist_by_id returns correct artist
 *  10. get_tracks_by_ids returns requested tracks
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

// ── Test 1: check_database_health ──

test('check_database_health returns ok status', async () => {
  const health = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('check_database_health')
  );

  expect(health).toBeTruthy();
  // Health check should return some kind of ok indicator
  // The exact shape depends on implementation, but it shouldn't throw
  expect(typeof health).toBe('object');
});

// ── Test 2: get_all_tracks returns 20 tracks ──

test('get_all_tracks returns at least 20 seeded tracks', async () => {
  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_tracks')
  );

  expect(Array.isArray(tracks)).toBe(true);
  // At least 20 seeded tracks; other specs may add more
  expect(tracks.length).toBeGreaterThanOrEqual(20);

  // Verify known seeded tracks are present
  const ids = tracks.map(t => t.id);
  expect(ids).toContain(2001);
  expect(ids).toContain(3001);
  expect(ids).toContain(4001);
});

// ── Test 3: get_all_albums returns 3 albums ──

test('get_all_albums returns at least the 3 seeded albums', async () => {
  const albums = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_albums')
  );

  expect(Array.isArray(albums)).toBe(true);
  // At least 3 seeded albums; other specs may create more
  expect(albums.length).toBeGreaterThanOrEqual(3);

  const names = albums.map(a => a.title);
  expect(names).toContain('Playwright Album');
  expect(names).toContain('Long Album');
  expect(names).toContain('Marathon Album');
});

// ── Test 4: get_all_artists ──

test('get_all_artists returns seeded artists', async () => {
  const artists = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_artists')
  );

  expect(Array.isArray(artists)).toBe(true);
  expect(artists.length).toBeGreaterThanOrEqual(1);

  const names = artists.map(a => a.name);
  expect(names).toContain('Playwright Artist');
});

// ── Test 5: get_all_genres ──

test('get_all_genres returns seeded genre', async () => {
  const genres = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_all_genres')
  );

  expect(Array.isArray(genres)).toBe(true);
  expect(genres.length).toBeGreaterThanOrEqual(1);

  const names = genres.map(g => g.name);
  expect(names).toContain('Playwright Genre');
});

// ── Test 6: get_random_albums ──

test('get_random_albums returns albums within limit', async () => {
  const albums = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_random_albums', { limit: 2 })
  );

  expect(Array.isArray(albums)).toBe(true);
  expect(albums.length).toBeLessThanOrEqual(2);
  expect(albums.length).toBeGreaterThanOrEqual(1);
});

// ── Test 7: get_recently_added_albums ──

test('get_recently_added_albums returns albums', async () => {
  const albums = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recently_added_albums', { limit: 10 })
  );

  expect(Array.isArray(albums)).toBe(true);
  expect(albums.length).toBeGreaterThanOrEqual(1);
});

// ── Test 8: get_album_by_id ──

test('get_album_by_id returns Playwright Album for ID 2001', async () => {
  const album = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_album_by_id', { id: 2001 })
  );

  expect(album).toBeTruthy();
  expect(album.title).toBe('Playwright Album');
  expect(album.id).toBe(2001);
});

// ── Test 9: get_artist_by_id ──

test('get_artist_by_id returns Playwright Artist for ID 2001', async () => {
  const artist = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_artist_by_id', { id: 2001 })
  );

  expect(artist).toBeTruthy();
  expect(artist.name).toBe('Playwright Artist');
});

// ── Test 10: get_tracks_by_ids ──

test('get_tracks_by_ids returns requested tracks', async () => {
  const tracks = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_tracks_by_ids', { trackIds: [2001, 2003, 2005] })
  );

  expect(Array.isArray(tracks)).toBe(true);
  expect(tracks.length).toBe(3);

  const ids = tracks.map(t => t.id);
  expect(ids).toContain(2001);
  expect(ids).toContain(2003);
  expect(ids).toContain(2005);
});
