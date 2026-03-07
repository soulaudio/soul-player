/**
 * Screenshot audit script - takes screenshots of every page for visual review.
 * Run: node screenshot-audit.mjs
 */
import { chromium } from '@playwright/test';

const CDP_URL = 'http://localhost:9222';
const SCREENSHOT_DIR = 'screenshots';

async function main() {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  const page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  const screenshot = async (name) => {
    await page.waitForTimeout(500);
    await page.screenshot({ path: `${SCREENSHOT_DIR}/${name}.png`, fullPage: false });
    console.log(`  Captured: ${name}`);
  };

  // 1. Home page
  console.log('1. Home page');
  await page.click('[data-testid="nav-home"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('01-home');

  // 2. Albums page
  console.log('2. Albums page');
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('02-albums');

  // 3. Album detail page
  console.log('3. Album detail');
  const albumCard = page.locator('[data-testid^="media-card-album-"]').first();
  if (await albumCard.count() > 0) {
    await albumCard.click();
    await page.waitForTimeout(1000);
    await screenshot('03-album-detail');
  }

  // 4. Artists page
  console.log('4. Artists page');
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('04-artists');

  // 5. Artist detail
  console.log('5. Artist detail');
  const artistCard = page.locator('[data-testid^="media-card-artist-"]').first();
  if (await artistCard.count() > 0) {
    await artistCard.click();
    await page.waitForTimeout(1000);
    await screenshot('05-artist-detail');
  }

  // 6. Tracks page
  console.log('6. Tracks page');
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('06-tracks');

  // 7. Genres page
  console.log('7. Genres page');
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('07-genres');

  // 8. Playlists page
  console.log('8. Playlists page');
  await page.click('[data-testid="nav-playlists"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('08-playlists');

  // 9. Now Playing page (need to start playback first)
  console.log('9. Now Playing page');
  // Start playback via IPC
  await page.evaluate(async () => {
    try {
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
    } catch(e) { console.error(e); }
  });
  await page.waitForTimeout(1500);

  // Navigate to now playing
  const nowPlayingNav = page.locator('[data-testid="nav-now-playing"]');
  if (await nowPlayingNav.count() > 0) {
    await nowPlayingNav.click({ force: true });
    await page.waitForTimeout(1000);
    await screenshot('09-now-playing');
  }

  // 10. Settings pages
  console.log('10. Settings');
  const settingsBtn = page.locator('[data-testid="settings-button"]');
  if (await settingsBtn.count() > 0) {
    await settingsBtn.click();
    await page.waitForTimeout(1000);
    await screenshot('10-settings-general');

    // Music Data settings
    const musicDataNav = page.locator('[data-testid="nav-settings-musicData"]');
    if (await musicDataNav.count() > 0) {
      await musicDataNav.click();
      await page.waitForTimeout(1000);
      await screenshot('11-settings-musicdata');
    }

    // Audio settings
    const audioNav = page.locator('[data-testid="nav-settings-audio"]');
    if (await audioNav.count() > 0) {
      await audioNav.click();
      await page.waitForTimeout(1000);
      await screenshot('12-settings-audio');
    }

    // Close settings
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  }

  // 11. With sidebar visible (already playing)
  console.log('11. Sidebar with playback');
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForTimeout(1000);
  await screenshot('13-albums-with-sidebar');

  // Stop playback
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  });

  console.log('\nDone! Screenshots saved to screenshots/');
  await browser.close();
}

main().catch(console.error);
