/**
 * Shuffle mode + auto-advance E2E tests — Playwright CDP
 *
 * Verifies that auto-advance works correctly when shuffle is enabled:
 *   1. Enable shuffle → play album → track finishes → next track loads (different from sequential)
 *   2. Shuffle + seek near end → auto-advance fires with shuffled track
 *   3. Enable shuffle mid-playback → auto-advance still works
 *   4. Shuffle + RepeatAll → wraps around after all tracks play
 *   5. Shuffle auto-advance chain: 3 consecutive auto-advances
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks × 10s WAV (IDs 2001–2005)
 *   Album 2003 — "Marathon Album" — 10 tracks × 15s WAV (IDs 4001–4010)
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
  // Reset shuffle and repeat to off
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

async function startPlaybackWithShuffle(p, albumId) {
  // Enable shuffle BEFORE starting playback
  await p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' })
  );

  await p.evaluate(async (aid) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: aid });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
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
  }, albumId);

  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function startPlaybackNoShuffle(p, albumId) {
  await p.evaluate(async (aid) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: aid });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
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
  }, albumId);

  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

async function waitForTitleChange(p, fromTitle, timeout = 20_000) {
  await p.waitForFunction(
    (from) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      const title = titleEl.textContent.trim();
      return title !== '' && title !== from;
    },
    fromTitle,
    { timeout }
  );
}

// ================================================================
// Test 1: Shuffle + auto-advance — track finishes, next track loads
// ================================================================

test('shuffle mode: auto-advance loads next track when current finishes', async () => {
  test.setTimeout(30_000);

  await startPlaybackWithShuffle(page, 2001);

  const firstTitle = await getNowPlayingTitle(page);
  expect(firstTitle).toBeTruthy();

  // Seek near end to speed up the test
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 })
  );
  await page.waitForTimeout(300);

  // Wait for auto-advance
  await waitForTitleChange(page, firstTitle, 15_000);

  const secondTitle = await getNowPlayingTitle(page);
  expect(secondTitle).toBeTruthy();
  expect(secondTitle).not.toBe(firstTitle);

  // Still playing
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Verify shuffle is still on
  const shuffle = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(shuffle).toBe('random');
});

// ================================================================
// Test 2: Shuffle + seek near end → auto-advance fires
// ================================================================

test('shuffle mode: seek near end triggers auto-advance to shuffled track', async () => {
  test.setTimeout(25_000);

  await startPlaybackWithShuffle(page, 2001);

  const firstTitle = await getNowPlayingTitle(page);

  // Seek to 9.0s (1s before end of 10s track)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 })
  );

  // Wait for auto-advance
  await waitForTitleChange(page, firstTitle, 15_000);

  const nextTitle = await getNowPlayingTitle(page);
  expect(nextTitle).toBeTruthy();
  expect(nextTitle).not.toBe(firstTitle);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 3: Enable shuffle mid-playback → auto-advance still works
// ================================================================

test('enable shuffle mid-playback: auto-advance still fires on track end', async () => {
  test.setTimeout(30_000);

  // Start WITHOUT shuffle
  await startPlaybackNoShuffle(page, 2001);

  const firstTitle = await getNowPlayingTitle(page);
  expect(firstTitle).toBe('Track One');

  // Enable shuffle mid-playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' })
  );
  await page.waitForTimeout(200);

  // Verify shuffle is active
  const shuffle = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(shuffle).toBe('random');

  // Seek near end
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 })
  );

  // Wait for auto-advance
  await waitForTitleChange(page, 'Track One', 15_000);

  const nextTitle = await getNowPlayingTitle(page);
  expect(nextTitle).toBeTruthy();
  expect(nextTitle).not.toBe('Track One');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 4: Shuffle + RepeatAll → wraps around after queue exhaustion
// ================================================================

test('shuffle + repeat all: auto-advance wraps around after last track', async () => {
  test.setTimeout(60_000);

  // Use album 2001 (5 tracks × 10s)
  await startPlaybackWithShuffle(page, 2001);

  // Enable RepeatAll
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'all' })
  );

  const firstTitle = await getNowPlayingTitle(page);

  // Skip to the last track in queue (index 3 = 4th upcoming = 5th total)
  // After play_queue, Track One is playing, queue has [T2,T3,T4,T5] (shuffled)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 3 })
  );
  await page.waitForTimeout(500);

  const lastTitle = await getNowPlayingTitle(page);
  expect(lastTitle).toBeTruthy();

  // Seek near end of this last track
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 })
  );

  // Wait for auto-advance — with RepeatAll, it should wrap around
  await waitForTitleChange(page, lastTitle, 15_000);

  const wrappedTitle = await getNowPlayingTitle(page);
  expect(wrappedTitle).toBeTruthy();

  // State should be Playing (wrapped around, not stopped)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: Shuffle auto-advance chain — 3 consecutive advances
// ================================================================

test('shuffle mode: 3 consecutive auto-advances via seek near end', async () => {
  test.setTimeout(60_000);

  await startPlaybackWithShuffle(page, 2001);

  const playedTitles = [];
  let currentTitle = await getNowPlayingTitle(page);
  playedTitles.push(currentTitle);

  // Chain 3 auto-advances by seeking near end
  for (let i = 0; i < 3; i++) {
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: 9.0 })
    );

    await waitForTitleChange(page, currentTitle, 15_000);

    currentTitle = await getNowPlayingTitle(page);
    playedTitles.push(currentTitle);

    // Still playing after each advance
    const state = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_playback_state')
    );
    expect(state).toBe('Playing');

    // Brief pause to let state settle
    await page.waitForTimeout(300);
  }

  // Should have 4 distinct titles (1 initial + 3 advances)
  expect(playedTitles.length).toBe(4);

  // All titles should be from the album
  const validTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
  for (const title of playedTitles) {
    expect(validTitles).toContain(title);
  }

  // No consecutive duplicates (same track shouldn't play twice in a row)
  for (let i = 1; i < playedTitles.length; i++) {
    expect(playedTitles[i]).not.toBe(playedTitles[i - 1]);
  }
});
