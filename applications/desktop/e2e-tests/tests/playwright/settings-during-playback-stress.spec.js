/**
 * Settings During Playback Stress E2E tests — Playwright CDP
 *
 * Stress-tests changing settings while audio is actively playing:
 *   theme switching, volume leveling mode changes, crossfade toggles,
 *   resampling changes, and crossfade adjustments during playback.
 *
 * 7 tests
 *
 * Seed data:
 *   Album 2002 "Long Album" — 5 tracks (30s WAV)
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

async function startLongPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2002 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
}

async function getPlaybackState(p) {
  return p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
}

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  // Restore default settings
  await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'ui.theme', value: 'dark' });
      await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'disabled' });
    } catch {}
  }).catch(() => {});
});

// ── Test 1: Theme switching during playback ──

test('switching themes 5 times during playback does not interrupt audio', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  const themes = ['dark', 'ocean', 'earth', 'nord', 'dark'];
  for (const theme of themes) {
    await page.evaluate(async (t) => {
      await window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'ui.theme', value: t });
    }, theme);
    await page.waitForTimeout(200);
  }

  const state = await getPlaybackState(page);
  expect(state).toBe('Playing');
});

// ── Test 2: Volume leveling mode cycling during playback ──

test('cycling volume leveling modes during playback does not crash', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  const modes = ['disabled', 'replaygain_track', 'replaygain_album', 'disabled'];
  for (const mode of modes) {
    await page.evaluate(async (m) => {
      await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: m });
    }, mode);
    await page.waitForTimeout(300);
  }

  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 3: Crossfade toggle during playback ──

test('toggling crossfade during playback does not interrupt audio', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  for (let i = 0; i < 5; i++) {
    await page.evaluate(async (enabled) => {
      await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled });
    }, i % 2 === 0);
    await page.waitForTimeout(200);
  }

  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Restore
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false });
  }).catch(() => {});
});

// ── Test 4: Multiple settings changes simultaneously during playback ──

test('changing theme + volume leveling + crossfade together during playback', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'ui.theme', value: 'ocean' });
    await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'replaygain_track' });
    await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: true });
    await window.__TAURI_INTERNALS__.invoke('set_crossfade_duration', { durationMs: 2000 });
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Restore
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false });
    await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'disabled' });
  }).catch(() => {});
});

// ── Test 5: Resampling quality change during playback ──

test('changing resampling quality during playback does not crash', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  const qualities = ['low', 'medium', 'high', 'medium'];
  for (const q of qualities) {
    await page.evaluate(async (quality) => {
      await window.__TAURI_INTERNALS__.invoke('set_resampling_quality', { quality }).catch(() => {});
    }, q);
    await page.waitForTimeout(300);
  }

  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 6: Volume leveling preamp adjustment during playback ──

test('adjusting volume leveling preamp during playback is stable', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  // Enable leveling first
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'replaygain_track' });
  });

  // Sweep preamp values
  for (let db = -6; db <= 6; db += 2) {
    await page.evaluate(async (preampDb) => {
      await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_preamp', { preampDb });
    }, db);
  }

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Restore
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_volume_leveling_mode', { mode: 'disabled' });
  }).catch(() => {});
});

// ── Test 7: Navigation + settings changes + playback combined ──

test('navigating settings pages while playing and changing settings', async () => {
  test.setTimeout(60_000);
  await startLongPlayback(page);

  // Navigate through settings pages
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });

  // About page
  await page.click('[data-testid="nav-settings-about"]');
  await page.waitForTimeout(500);
  let state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Appearance page + change theme
  await page.click('[data-testid="nav-settings-appearance"]');
  await page.waitForTimeout(300);
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: 'ui.theme', value: 'earth' });
  });
  state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Close settings
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // Navigate to albums
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForTimeout(300);

  state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});
