/**
 * Context Auto-Advance E2E Tests — Playwright CDP
 *
 * Verifies that auto-advance (track naturally finishes → next track starts)
 * works correctly when playback is started from every possible context:
 *
 *   1. Album detail page — Play All button
 *   2. Album detail page — double-click track row
 *   3. Artist detail page — Play All button
 *   4. Artist detail page — double-click track row
 *   5. Genre detail page — Play All button
 *   6. Genre detail page — double-click track row
 *   7. Playlist detail page — Play All button
 *   8. Playlist detail page — double-click track row
 *   9. Tracks page — double-click track row
 *  10. Albums grid — MediaCard play button
 *  11. Artists grid — MediaCard play button
 *  12. Playlists grid — MediaCard play button
 *  13. Now Playing page — click track in list
 *  14. Direct IPC play_queue (baseline sanity check)
 *
 * Each test starts playback, seeks near the end of the first track, and
 * verifies that the next track loads and begins playing automatically.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 "Playwright Album" / Artist 2001 "Playwright Artist"
 *     5 tracks: IDs 2001–2005, titles "Track One"–"Track Five"
 *     10-second WAV files, 2.0s duration in DB
 *   Genre 4001 "Playwright Genre" — linked to tracks 2001–2005
 *   Playlist 3001 "Favorites" — tracks added in beforeAll
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

const TRACK_IDS = [2001, 2002, 2003, 2004, 2005];

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  // Ensure playlist 3001 has all 5 tracks
  await page.evaluate(async (ids) => {
    for (const id of ids) {
      await window.__TAURI_INTERNALS__
        .invoke('add_track_to_playlist', { playlistId: '3001', trackId: String(id) })
        .catch(() => {});
    }
  }, TRACK_IDS);
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
  // Navigate to stable base
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/** Wait for playback to be Playing and optionally showing the expected title. */
async function waitForPlaying(p, expectedTitle = null) {
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  if (expectedTitle) {
    await p.waitForFunction(
      (exp) => {
        const c = document.querySelector('[data-testid="now-playing-title"]');
        if (!c) return false;
        const el = c.querySelector('.text-sm');
        return el && el.textContent.trim() === exp;
      },
      expectedTitle,
      { timeout: 10_000 }
    );
  }
  await p.waitForTimeout(150);
}

/** Get the current track title from the NowPlayingPanel sidebar. */
async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

/** Wait for the title to change away from `notTitle` to something else. */
async function waitForTitleChange(p, notTitle, timeout = 20_000) {
  await p.waitForFunction(
    (reject) => {
      const c = document.querySelector('[data-testid="now-playing-title"]');
      if (!c) return false;
      const el = c.querySelector('.text-sm');
      if (!el) return false;
      const t = el.textContent.trim();
      return t !== '' && t !== reject;
    },
    notTitle,
    { timeout }
  );
}

/**
 * Core auto-advance assertion:
 * 1. Confirm the expected track is playing
 * 2. Seek to 1s before the end of the 10s WAV
 * 3. Wait for the track to naturally finish and advance
 * 4. Verify the next track is now playing
 */
async function assertAutoAdvance(p, currentTitle, expectedNextTitle) {
  // Confirm current track
  const title = await getNowPlayingTitle(p);
  expect(title).toBe(currentTitle);

  // Seek to near end (10s WAV file, seek to 9.0s)
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
  });
  await p.waitForTimeout(200);

  // Wait for auto-advance
  await waitForTitleChange(p, currentTitle);

  // Verify next track
  const nextTitle = await getNowPlayingTitle(p);
  expect(nextTitle).toBe(expectedNextTitle);

  // Verify still Playing
  const state = await p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
}

/** Double-click a track row by title text. */
async function dblclickTrackRow(p, title) {
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const row = p.locator('[data-testid="track-row"]').filter({ hasText: title });
  await row.waitFor({ state: 'visible', timeout: 10_000 });
  await row.dblclick();
}

// ── Navigation helpers ───────────────────────────────────────────────────

async function navigateToAlbumDetail(p) {
  await p.click('[data-testid="nav-albums"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-album-2001"]');
  await card.locator('p').filter({ hasText: 'Playwright Album' }).first().click();
  await p.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToArtistDetail(p) {
  await p.click('[data-testid="nav-artists"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-artist-2001"]');
  await card.locator('p').filter({ hasText: 'Playwright Artist' }).first().click();
  await p.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToGenrePage(p) {
  await p.evaluate(() => {
    window.history.pushState({}, '', '/genres/4001');
    window.dispatchEvent(new PopStateEvent('popstate', { state: {} }));
  });
  await p.waitForSelector('[data-testid="genre-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToPlaylistDetail(p) {
  await p.click('[data-testid="nav-playlists"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-playlist-3001"]', { timeout: 15_000 });
  await p.locator('[data-testid="media-card-playlist-3001"]').click();
  await p.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToTracksPage(p) {
  await p.click('[data-testid="nav-tracks"]', { force: true });
  await p.waitForSelector('[data-testid="tracks-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

/** Start playback via direct IPC (baseline, no UI navigation). */
async function startPlaybackViaIPC(p, albumId = 2001) {
  await p.evaluate(async (aid) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: aid });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map((t) => ({
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
}

// ═══════════════════════════════════════════════════════════════════════════
// Album detail page
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Album detail', () => {
  test('Play All → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await navigateToAlbumDetail(page);
    const btn = page.locator('[data-testid="album-play-all-button"]');
    await btn.waitFor({ state: 'visible', timeout: 5_000 });
    await btn.click();
    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });

  test('Double-click Track Two → auto-advance: Track Two finishes → Track Three starts', async () => {
    test.setTimeout(30_000);
    await navigateToAlbumDetail(page);
    await dblclickTrackRow(page, 'Track Two');
    await waitForPlaying(page, 'Track Two');
    await assertAutoAdvance(page, 'Track Two', 'Track Three');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Artist detail page
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Artist detail', () => {
  test('Play All → auto-advance: first track finishes → next track starts', async () => {
    test.setTimeout(30_000);
    await navigateToArtistDetail(page);
    const btn = page.locator('[data-testid="artist-play-all-button"]');
    await btn.waitFor({ state: 'visible', timeout: 5_000 });
    await btn.click();
    await waitForPlaying(page);

    // Artist top-tracks sort may differ from album order — just verify advance happens
    const firstTitle = await getNowPlayingTitle(page);
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
    });
    await page.waitForTimeout(200);
    await waitForTitleChange(page, firstTitle);

    const nextTitle = await getNowPlayingTitle(page);
    expect(nextTitle).not.toBe(firstTitle);
    const state = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playback_state')
    );
    expect(state).toBe('Playing');
  });

  test('Double-click Track Three → auto-advance to next track', async () => {
    test.setTimeout(30_000);
    await navigateToArtistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');

    // Artist top-tracks: after Track Three, next depends on sort order
    const firstTitle = await getNowPlayingTitle(page);
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
    });
    await page.waitForTimeout(200);
    await waitForTitleChange(page, firstTitle);

    const nextTitle = await getNowPlayingTitle(page);
    expect(nextTitle).not.toBe(firstTitle);
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Genre detail page
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Genre detail', () => {
  test('Play All → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await navigateToGenrePage(page);
    const btn = page.locator('[data-testid="genre-play-all-button"]');
    await btn.waitFor({ state: 'visible', timeout: 15_000 });
    await btn.click();
    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });

  test('Double-click Track Two → auto-advance: Track Two finishes → Track Three starts', async () => {
    test.setTimeout(30_000);
    await navigateToGenrePage(page);
    await dblclickTrackRow(page, 'Track Two');
    await waitForPlaying(page, 'Track Two');
    await assertAutoAdvance(page, 'Track Two', 'Track Three');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Playlist detail page
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Playlist detail', () => {
  test('Play All → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await navigateToPlaylistDetail(page);
    const btn = page.locator('[data-testid="playlist-play-all-button"]');
    await btn.waitFor({ state: 'visible', timeout: 5_000 });
    await btn.click();
    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });

  test('Double-click Track Two → auto-advance: Track Two finishes → Track Three starts', async () => {
    test.setTimeout(30_000);
    await navigateToPlaylistDetail(page);
    await dblclickTrackRow(page, 'Track Two');
    await waitForPlaying(page, 'Track Two');
    await assertAutoAdvance(page, 'Track Two', 'Track Three');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Tracks page (all tracks)
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Tracks page', () => {
  test('Double-click Track One → auto-advance to next track in list', async () => {
    test.setTimeout(30_000);
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track One');
    await waitForPlaying(page, 'Track One');

    // Tracks page order may not be album order — just verify advance
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
    });
    await page.waitForTimeout(200);
    await waitForTitleChange(page, 'Track One');

    const nextTitle = await getNowPlayingTitle(page);
    expect(nextTitle).not.toBe('Track One');
    const state = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playback_state')
    );
    expect(state).toBe('Playing');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Albums grid — MediaCard play button
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Albums grid MediaCard', () => {
  test('MediaCard play button → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await page.click('[data-testid="nav-albums"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

    // Hover + click the play button on the album card
    const card = page.locator('[data-testid="media-card-album-2001"]');
    await card.hover();
    const playBtn = card.locator('[data-testid="media-card-play-button"]');
    await playBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playBtn.click();

    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Artists grid — MediaCard play button
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Artists grid MediaCard', () => {
  test('MediaCard play button → auto-advance: first track finishes → next starts', async () => {
    test.setTimeout(30_000);
    await page.click('[data-testid="nav-artists"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });

    const card = page.locator('[data-testid="media-card-artist-2001"]');
    await card.hover();
    const playBtn = card.locator('[data-testid="media-card-play-button"]');
    await playBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playBtn.click();

    await waitForPlaying(page);
    const firstTitle = await getNowPlayingTitle(page);

    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 });
    });
    await page.waitForTimeout(200);
    await waitForTitleChange(page, firstTitle);

    const nextTitle = await getNowPlayingTitle(page);
    expect(nextTitle).not.toBe(firstTitle);
    const state = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playback_state')
    );
    expect(state).toBe('Playing');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Playlists grid — MediaCard play button
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Playlists grid MediaCard', () => {
  test('MediaCard play button → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await page.click('[data-testid="nav-playlists"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-playlist-3001"]', { timeout: 15_000 });

    const card = page.locator('[data-testid="media-card-playlist-3001"]');
    await card.hover();
    const playBtn = card.locator('[data-testid="media-card-play-button"]');
    await playBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playBtn.click();

    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Now Playing page — click track in track list
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Now Playing page', () => {
  test('Click track in now-playing list → auto-advance to next track', async () => {
    test.setTimeout(30_000);

    // Start playback via IPC first
    await startPlaybackViaIPC(page);
    await waitForPlaying(page, 'Track One');

    // Record context so NowPlayingPage has context to display
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
        input: {
          contextType: 'album',
          contextId: '2001',
          contextName: 'Playwright Album',
          contextArtworkPath: null,
        },
      });
    });

    // Navigate to Now Playing page
    await page.click('[data-testid="now-playing-title"]', { force: true });
    await page.waitForSelector('[data-testid="now-playing-page"]', { timeout: 10_000 });

    // Click Track Two in the now-playing queue list
    // Queue items use testid "now-playing-queue-item-{index}" — Track Two is index 1
    await page.waitForSelector('[data-testid="now-playing-queue-list"]', { timeout: 10_000 });
    const queueItem = page.locator('[data-testid="now-playing-queue-item-1"]');
    await queueItem.waitFor({ state: 'visible', timeout: 10_000 });
    await queueItem.click();

    await waitForPlaying(page, 'Track Two');
    await assertAutoAdvance(page, 'Track Two', 'Track Three');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// Direct IPC — baseline sanity check
// ═══════════════════════════════════════════════════════════════════════════

test.describe('Direct IPC baseline', () => {
  test('play_queue via IPC → auto-advance: Track One finishes → Track Two starts', async () => {
    test.setTimeout(30_000);
    await startPlaybackViaIPC(page);
    await waitForPlaying(page, 'Track One');
    await assertAutoAdvance(page, 'Track One', 'Track Two');
  });
});
