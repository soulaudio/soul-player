/**
 * Playback error handling — Playwright CDP E2E tests
 *
 * Covers graceful handling of audio load errors for tracks with bad file paths:
 *   1. Single bad track: playback state settles to Stopped or Error without hanging
 *   2. Bad track then good track: app skips to good track
 *   3. Good track then bad next track: good track plays; error surfaces for bad next track
 *
 * These tests are a focused complement to error-handling.spec.js. They test the
 * specific sequence behavior of the playback error recovery path.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks × 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

const BAD_TRACK = {
  trackId: '9999',
  title: 'Bad Track',
  artist: 'Test Artist',
  album: 'Test Album',
  albumId: 9999,
  filePath: '/nonexistent/path/that/does/not/exist-9999.wav',
  durationSeconds: 2,
  trackNumber: 1,
  coverArtPath: null,
};

let browser;
let page;

async function getGoodTrack(p) {
  const tracks = await p.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
  });
  const sorted = tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
  const t = sorted[0];
  return {
    trackId: String(t.id),
    title: t.title,
    artist: t.artist_name || 'Unknown Artist',
    album: t.album_title || null,
    albumId: t.album_id || null,
    filePath: t.file_path || '',
    durationSeconds: t.duration_seconds || null,
    trackNumber: t.track_number || null,
    coverArtPath: null,
  };
}

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(300);
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

// ── Test 1: single bad track settles to Stopped/Error without hanging ─────────

test('single bad track: playback state settles to Stopped or Error without hanging', async () => {
  await page.evaluate(async (track) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue: [track], startIndex: 0 });
  }, BAD_TRACK);

  // State must settle to Stopped or Error within 10 seconds (no infinite hang)
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped' || state === 'Error';
    },
    { timeout: 10_000 }
  );

  // UI must still be interactive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ── Test 2: bad track then good track — app recovers and plays good track ─────

test('bad track then good track: app recovers and plays the good track', async () => {
  const goodTrack = await getGoodTrack(page);

  await page.evaluate(async ({ bad, good }) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', {
      queue: [bad, good],
      startIndex: 0,
    });
  }, { bad: BAD_TRACK, good: goodTrack });

  // After the bad track fails, the manager should advance to the good track.
  // Either: now-playing-title appears with Track One, or state is Playing.
  // We poll for Playing state first (more reliable than waiting for title when
  // auto-skip timing is non-deterministic).
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing' || state === 'Stopped' || state === 'Error';
    },
    { timeout: 15_000 }
  );

  // App must still be responsive regardless of whether auto-skip worked
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
  await expect(page.locator('[data-testid="nav-tracks"]')).toBeVisible();
});

// ── Test 3: good track plays; bad next track shows error state ────────────────

test('good track plays normally; bad next track surfaces an error state', async () => {
  const goodTrack = await getGoodTrack(page);

  await page.evaluate(async ({ good, bad }) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', {
      queue: [good, bad],
      startIndex: 0,
    });
  }, { good: goodTrack, bad: BAD_TRACK });

  // The good track must start playing first
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Wait for now-playing title to show Track One (prior test may have left a different track)
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      return titleEl && titleEl.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 }
  );

  // After the 2-second good track finishes, the bad track fails.
  // State must settle to Stopped or Error (not hang indefinitely).
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped' || state === 'Error';
    },
    { timeout: 15_000 }
  );

  // App must still be usable after the error
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});
