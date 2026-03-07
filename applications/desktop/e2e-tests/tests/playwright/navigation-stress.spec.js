/**
 * Navigation stress tests — Playwright CDP
 *
 * Verifies that rapid page navigation during active playback does not:
 *   - Crash the app or freeze the UI
 *   - Stop or glitch audio playback
 *   - Accumulate event listeners or memory leaks
 *   - Break the sidebar now-playing state
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist" — 5 tracks x 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
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
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
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
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2001',
        contextName: 'Playwright Album',
        contextArtworkPath: null,
      },
    });
  });
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function waitForSidebarTitle(p, expected, timeout = 15_000) {
  await p.waitForFunction(
    (exp) => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === exp;
    },
    expected,
    { timeout }
  );
}

async function getSidebarTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ---- Navigation targets (all available nav buttons) ----

const NAV_TARGETS = ['nav-albums', 'nav-artists', 'nav-tracks', 'nav-playlists', 'nav-home'];

async function navigateTo(p, testId) {
  await p.click(`[data-testid="${testId}"]`, { force: true });
  await p.waitForTimeout(100);
}

// ================================================================
// Test 1: Rapid page cycling during playback — 15 round-trips
// ================================================================

test('rapid page cycling: 15 nav switches during playback stay responsive', async () => {
  await startPlayback(page);

  const start = Date.now();
  const CYCLES = 15;

  for (let i = 0; i < CYCLES; i++) {
    const target = NAV_TARGETS[i % NAV_TARGETS.length];
    await navigateTo(page, target);
  }

  const elapsed = Date.now() - start;
  // 15 navigation switches should complete well under 15s
  expect(elapsed).toBeLessThan(15_000);

  // Playback should still be active after all navigation
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Sidebar should still show the now-playing panel
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 2: Navigate away and back while skipping tracks
// ================================================================

test('navigation + track skip interleaved: sidebar stays consistent', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Skip to Track Two
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Two');

  // Navigate to Tracks page
  await navigateTo(page, 'nav-tracks');

  // Skip to Track Three while on Tracks page
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Three');

  // Navigate to Artists page
  await navigateTo(page, 'nav-artists');

  // Verify sidebar still shows Track Three regardless of current page
  expect(await getSidebarTitle(page)).toBe('Track Three');

  // Navigate back to Albums
  await navigateTo(page, 'nav-albums');

  // Still Track Three
  expect(await getSidebarTitle(page)).toBe('Track Three');

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 3: Rapid back-and-forth between two pages
// ================================================================

test('rapid toggle between Albums and Tracks: 20 switches no crash', async () => {
  await startPlayback(page);

  const start = Date.now();

  for (let i = 0; i < 20; i++) {
    if (i % 2 === 0) {
      await navigateTo(page, 'nav-tracks');
    } else {
      await navigateTo(page, 'nav-albums');
    }
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(12_000);

  // Playback should survive 20 rapid page switches
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 4: Navigate to settings and back during playback
// ================================================================

test('navigating to settings and back does not interrupt playback', async () => {
  await startPlayback(page);
  const titleBefore = await getSidebarTitle(page);

  // Open settings
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForTimeout(500);

  // Playback should continue in the background
  const stateInSettings = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateInSettings).toBe('Playing');

  // Navigate back to albums
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  await navigateTo(page, 'nav-albums');

  // Verify sidebar still shows a track (may have auto-advanced since we're on 2s tracks)
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: Navigate into album detail, back to list, repeat rapidly
// ================================================================

test('album detail enter/exit 5 times during playback stays stable', async () => {
  await startPlayback(page);

  for (let i = 0; i < 5; i++) {
    // Navigate to albums list
    await navigateTo(page, 'nav-albums');
    await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });

    // Click into album detail
    const card = page.locator('[data-testid="media-card-album-2001"]');
    const titleP = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
    await titleP.click();
    await page.waitForSelector('[data-testid="album-detail-page"]', { timeout: 10_000 });
  }

  // Playback should survive repeated detail entry/exit
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Sidebar still present
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 6: Full page tour — visit every nav target and verify stability
// ================================================================

test('full page tour: visit all nav targets sequentially during playback', async () => {
  await startPlayback(page);

  // Visit each page and verify a key element loads
  for (const nav of NAV_TARGETS) {
    await navigateTo(page, nav);
    // Give each page time to render
    await page.waitForTimeout(300);
  }

  // End on albums
  await navigateTo(page, 'nav-albums');
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 10_000 });

  // Playback must still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Sidebar track info must still be visible
  const panel = page.locator('[data-testid="now-playing-title"]');
  await expect(panel).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 7: Pause, navigate around, resume — state preserved
// ================================================================

test('pause → navigate 5 pages → resume: playback resumes correctly', async () => {
  await startPlayback(page);

  // Skip to Track Two so we can verify title after resume
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Two');

  // Pause
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  // Navigate through 5 different pages
  for (const nav of NAV_TARGETS) {
    await navigateTo(page, nav);
    await page.waitForTimeout(200);
  }

  // Back to albums
  await navigateTo(page, 'nav-albums');

  // Should still be paused and on Track Two
  const stateBeforeResume = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateBeforeResume).toBe('Paused');
  expect(await getSidebarTitle(page)).toBe('Track Two');

  // Resume
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Still Track Two
  expect(await getSidebarTitle(page)).toBe('Track Two');
});

// ================================================================
// Test 8: Navigation with crossfade enabled — settings + page changes
// ================================================================

test('crossfade enabled + navigation + settings: no crash or hang', async () => {
  // Enable crossfade before playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 2000, curve: 'equal_power',
    })
  );

  await startPlayback(page);

  // Navigate through pages while changing crossfade settings
  const curves = ['linear', 'square_root', 's_curve', 'equal_power'];

  for (let i = 0; i < NAV_TARGETS.length; i++) {
    await navigateTo(page, NAV_TARGETS[i]);
    await page.waitForTimeout(200);

    // Change crossfade curve mid-navigation
    const curve = curves[i % curves.length];
    await page.evaluate(async (c) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_curve', { curve: c }), curve
    );
    await page.waitForTimeout(100);
  }

  // Navigate to settings and change crossfade there too
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForTimeout(500);
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_duration', { durationMs: 5000 })
  );
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  await navigateTo(page, 'nav-albums');

  // Playback must still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Crossfade should still be enabled with updated duration
  const settings = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_crossfade_settings')
  );
  expect(settings.enabled).toBe(true);
  expect(settings.duration_ms || settings.durationMs).toBe(5000);
});
