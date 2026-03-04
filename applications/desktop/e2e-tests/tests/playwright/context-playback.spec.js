/**
 * Context Playback E2E Tests — Playwright CDP
 *
 * Verifies that playing from each navigation context (Album, Artist, Genre,
 * Playlist, Tracks) loads the correct current track and queue — in both normal
 * and shuffle modes.
 *
 * 20 tests total: 4 tests × 5 contexts
 *   - Play All: current=Track One, queue=[T2,T3,T4,T5] in order
 *   - Track Three click: current=T3, queue=[T4,T5] in order
 *   - Shuffle + Play All: all 5 IDs present in any order
 *   - Shuffle + Track Three: T3 is current, remaining 4 IDs in queue
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks (IDs 2001–2005, 2-second WAV)
 *   Artist 2001 — "Playwright Artist"
 *   Genre 4001 — "Playwright Genre" (all 5 tracks)
 *   Playlist 3001 — "Favorites" (beforeAll adds tracks 2001–2005)
 *
 * IPC queue notes:
 *   get_queue() returns UPCOMING tracks only (not current).
 *   After play_queue(5 tracks) + play starts: current=1, queue=4.
 *   Tracks are 2s long — pauseAfterPlay() is called before every assertion.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TRACK_IDS = [2001, 2002, 2003, 2004, 2005];

// ---------------------------------------------------------------------------
// CDP connection — shared across the entire spec file
// ---------------------------------------------------------------------------

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  // Add all 5 tracks to Favorites (playlist 3001). Runs once for the whole suite.
  // add_track_to_playlist is idempotent — if tracks are already there from a
  // previous run, the .catch(() => {}) swallows the "already exists" error.
  await page.evaluate(async trackIds => {
    for (const id of trackIds) {
      await window.__TAURI_INTERNALS__
        .invoke('add_track_to_playlist', { playlistId: '3001', trackId: String(id) })
        .catch(() => {});
    }
  }, TRACK_IDS);
});

test.afterAll(async () => {
  await browser.close();
});

// After every test: stop playback and restore shuffle to off.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// Before each test: stop any active playback, dismiss open overlays, navigate to stable base.
test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums as a stable base — force:true pierces any residual backdrop.
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------

/**
 * Returns the current track's numeric ID, or null if nothing is playing.
 * Uses get_current_track() IPC — QueueTrack serializes with field name 'id'.
 */
async function getCurrentTrackId(p) {
  return p.evaluate(async () => {
    const track = await window.__TAURI_INTERNALS__.invoke('get_current_track');
    return track ? parseInt(track.id, 10) : null;
  });
}

/**
 * Returns an array of upcoming track IDs in queue order.
 * Uses get_queue() IPC — TrackData serializes with camelCase → field 'trackId'.
 * IMPORTANT: does NOT include the current track.
 */
async function getQueueIds(p) {
  return p.evaluate(async () => {
    const queue = await window.__TAURI_INTERNALS__.invoke('get_queue');
    return queue.map(t => parseInt(t.trackId, 10));
  });
}

/**
 * Returns a Set of all active track IDs: current track + upcoming queue.
 * Use for shuffle tests where order is random but membership must be exact.
 */
async function getAllActiveIds(p) {
  const currentId = await getCurrentTrackId(p);
  const queueIds = await getQueueIds(p);
  const all = [...queueIds];
  if (currentId !== null) all.push(currentId);
  return new Set(all);
}

/** Enable shuffle (random mode) via IPC. */
async function enableShuffle(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' });
  });
}

// ---------------------------------------------------------------------------
// Playback helpers
// ---------------------------------------------------------------------------

/**
 * Wait for the app to be in Playing state and (optionally) the specified
 * track title to appear in the NowPlayingPanel.
 *
 * Do NOT call this without a title in normal-mode tests — it would be ambiguous.
 * Omit the title only for shuffle+Play All where the starting track is random.
 */
async function waitForPlaying(p, expectedTitle = null) {
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  if (expectedTitle) {
    await p.waitForFunction(
      exp => {
        const c = document.querySelector('[data-testid="now-playing-title"]');
        if (!c) return false;
        const el = c.querySelector('.text-sm');
        return el && el.textContent.trim() === exp;
      },
      expectedTitle,
      { timeout: 10_000 }
    );
  }
  // Wait for the play-pause button to be enabled (store has caught up)
  await p.waitForFunction(
    () => {
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  await p.waitForTimeout(150);
}

/**
 * Pause playback immediately and wait for the Paused state.
 * Call this right after waitForPlaying() to freeze the 2-second tracks
 * before they auto-advance during assertions.
 */
async function pauseAfterPlay(p) {
  await p.click('[data-testid="play-pause-button"]');
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
}

/**
 * Find the first track-row containing the given title text and double-click it.
 * Waits for the track-list to be visible first.
 */
async function dblclickTrackRow(p, title) {
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const row = p.locator('[data-testid="track-row"]').filter({ hasText: title });
  await row.waitFor({ state: 'visible', timeout: 10_000 });
  await row.dblclick();
}

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

async function navigateToAlbumDetail(p) {
  await p.click('[data-testid="nav-albums"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-album-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();
  await p.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToArtistDetail(p) {
  await p.click('[data-testid="nav-artists"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-artist-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Artist' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();
  await p.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToGenrePage(p) {
  // No NavBar genre link on the detail page route — navigate via history API
  // (same pattern as genre-page.spec.js)
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

// ---------------------------------------------------------------------------
// Album context (4 tests)
// ---------------------------------------------------------------------------

test.describe('Album context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToAlbumDetail(page);
    const playAllBtn = page.locator('[data-testid="album-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToAlbumDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToAlbumDetail(page);
    const playAllBtn = page.locator('[data-testid="album-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page);  // shuffle: any track may start first
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToAlbumDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});

// ---------------------------------------------------------------------------
// Artist context (4 tests)
// ---------------------------------------------------------------------------

test.describe('Artist context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToArtistDetail(page);
    const playAllBtn = page.locator('[data-testid="artist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToArtistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToArtistDetail(page);
    const playAllBtn = page.locator('[data-testid="artist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToArtistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});

// ---------------------------------------------------------------------------
// Genre context (4 tests)
// Note: navigateToGenrePage uses history.pushState — no NavBar link to /genres/:id.
// ---------------------------------------------------------------------------

test.describe('Genre context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToGenrePage(page);
    const playAllBtn = page.locator('[data-testid="genre-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToGenrePage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToGenrePage(page);
    const playAllBtn = page.locator('[data-testid="genre-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToGenrePage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});

// ---------------------------------------------------------------------------
// Playlist context (4 tests)
// Playlist 3001 "Favorites" has tracks 2001–2005 added in beforeAll.
// ---------------------------------------------------------------------------

test.describe('Playlist context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToPlaylistDetail(page);
    const playAllBtn = page.locator('[data-testid="playlist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToPlaylistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToPlaylistDetail(page);
    const playAllBtn = page.locator('[data-testid="playlist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToPlaylistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});

// ---------------------------------------------------------------------------
// Tracks context (4 tests)
// No Play All button — double-click Track One for the full-queue test.
// Test DB has exactly 5 tracks (2001–2005).
// ---------------------------------------------------------------------------

test.describe('Tracks context', () => {
  test('Track One double-click: Track One is current, queue is [2002, 2003, 2004, 2005]', async () => {
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track One');
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Track One double-click: all 5 track IDs present', async () => {
    await enableShuffle(page);
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track One');
    await waitForPlaying(page);  // shuffle: any track may be first
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
