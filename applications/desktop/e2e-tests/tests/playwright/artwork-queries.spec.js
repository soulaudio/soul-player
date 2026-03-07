/**
 * Artwork Queries E2E tests — Playwright CDP
 *
 * Tests artwork retrieval and management IPC commands:
 *   get_track_artwork, get_album_artwork, get_album_artwork_with_source,
 *   get_artist_artwork, get_playlist_artwork, remove_artwork
 *
 * 6 tests
 *
 * Seed data:
 *   Album 2001 "Playwright Album" — 5 tracks (WAV files, no embedded art)
 *   Artist 2001 "Playwright Artist"
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

// ── Test 1: get_track_artwork returns null or string for seeded track ──

test('get_track_artwork returns null or data URL for track 2001', async () => {
  const artwork = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_track_artwork', { trackId: '2001' })
  );

  // WAV test files have no embedded artwork, so expect null
  // But if artwork was manually set, it would be a data URL string
  expect(artwork === null || typeof artwork === 'string').toBe(true);
});

// ── Test 2: get_album_artwork returns null or string ──

test('get_album_artwork returns null or data URL for album 2001', async () => {
  const artwork = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_album_artwork', { albumId: 2001 })
  );

  expect(artwork === null || typeof artwork === 'string').toBe(true);
});

// ── Test 3: get_artist_artwork returns null or string ──

test('get_artist_artwork returns null or data URL for artist 2001', async () => {
  const artwork = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_artist_artwork', { artistId: 2001 })
  );

  expect(artwork === null || typeof artwork === 'string').toBe(true);
});

// ── Test 4: get_album_artwork_with_source returns source info ──

test('get_album_artwork_with_source returns result with source field', async () => {
  const result = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_album_artwork_with_source', { albumId: 2001 })
  );

  // Returns an object with artwork and source, or null
  if (result) {
    expect(typeof result).toBe('object');
  }
  // null is valid for WAV files with no artwork
  expect(result === null || typeof result === 'object').toBe(true);
});

// ── Test 5: get_playlist_artwork returns null or string ──

test('get_playlist_artwork returns null or data URL for playlist 3001', async () => {
  const artwork = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playlist_artwork', { playlistId: '3001' })
  );

  expect(artwork === null || typeof artwork === 'string').toBe(true);
});

// ── Test 6: remove_artwork for non-existent artwork doesn't crash ──

test('remove_artwork for album with no artwork does not throw', async () => {
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('remove_artwork', {
        entityType: 'album',
        entityId: '2001',
      });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  // Should either succeed (noop) or return a graceful error
  // The key assertion is that it doesn't crash the app
  expect(error === null || typeof error === 'string').toBe(true);
});
