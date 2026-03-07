/**
 * Screenshot audit — captures screenshots of key pages for visual review.
 * Run: npx playwright test --config playwright.cdp.config.js tests/playwright/screenshot-audit.spec.js
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';
import * as fs from 'fs';

const DIR = 'screenshots';

let browser;
let page;

test.beforeAll(async () => {
  fs.mkdirSync(DIR, { recursive: true });
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

const shot = async (name) => {
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: false });
};

test('capture all pages', async () => {
  test.setTimeout(180_000);

  // Start playback so sidebar has content
  await page.evaluate(async () => {
    const albums = await window.__TAURI_INTERNALS__.invoke('get_all_albums');
    if (albums.length > 0) {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: albums[0].id });
      if (tracks.length > 0) {
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
      }
    }
  });
  await page.waitForTimeout(1500);

  // 01 — Albums grid
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 10_000 });
  await shot('01-albums');

  // 02 — Album detail
  const albumCard = page.locator('[data-testid^="media-card-album-"]').first();
  if (await albumCard.count() > 0) {
    await albumCard.click();
    await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 10_000 });
    await shot('02-album-detail');
  }

  // 03 — Artists grid
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-artist-"]', { timeout: 10_000 });
  await shot('03-artists');

  // 04 — Artist detail
  const artistCard = page.locator('[data-testid^="media-card-artist-"]').first();
  if (await artistCard.count() > 0) {
    await artistCard.click();
    await page.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 10_000 });
    await page.waitForTimeout(500);
    await shot('04-artist-detail');
  }

  // 04b — Artist detail discography (scroll down)
  const discographySection = page.locator('text=Discography').first();
  if (await discographySection.count() > 0) {
    await discographySection.scrollIntoViewIfNeeded();
    await page.waitForTimeout(500);
    await shot('04b-artist-discography');
  }

  // 05 — Genres list
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForSelector('[data-testid^="genre-card-"]', { timeout: 10_000 });
  await shot('05-genres');

  // 05b — Genre detail
  const genreCard = page.locator('[data-testid^="genre-card-"]').first();
  if (await genreCard.count() > 0) {
    await genreCard.click();
    await page.waitForSelector('[data-testid="genre-detail-page"]', { timeout: 10_000 });
    await shot('05b-genre-detail');
    // Go back to genres
    await page.click('[data-testid="nav-genres"]', { force: true });
    await page.waitForTimeout(500);
  }

  // 06 — Tracks list
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 10_000 });
  await page.waitForTimeout(500);
  await shot('06-tracks');

  // 07 — Playlists grid
  await page.click('[data-testid="nav-playlists"]', { force: true });
  await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 10_000 });
  await page.waitForTimeout(500);
  await shot('07-playlists');

  // 07b — Playlist detail (click Favorites)
  const playlistCard = page.locator('[data-testid^="media-card-playlist-"]').first();
  if (await playlistCard.count() > 0) {
    await playlistCard.click();
    await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });
    await shot('07b-playlist-detail');
  }

  // 08 — Now Playing
  const npTitle = page.locator('[data-testid="now-playing-title"]');
  if (await npTitle.count() > 0) {
    await npTitle.click({ force: true });
    await page.waitForSelector('[data-testid="now-playing-page"]', { timeout: 10_000 });
    await shot('08-now-playing');
  }

  // 09 — Home
  const homeNav = page.locator('[data-testid="nav-home"]');
  if (await homeNav.count() > 0) {
    await homeNav.click({ force: true });
    await page.waitForSelector('[data-testid="home-page"]', { timeout: 10_000 });
    await page.waitForTimeout(500);
    await shot('09-home');
  }

  // 10 — Settings
  const settingsBtn = page.locator('[data-testid="nav-settings"]');
  if (await settingsBtn.count() > 0) {
    await settingsBtn.click({ force: true });
    await page.waitForTimeout(1000);
    await shot('10-settings');
  }

  // Stop playback
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  });
});
