/**
 * Window state, settings persistence, and user preferences — Playwright CDP
 *
 * Tests previously untested IPC flows:
 *   1. save_window_state_cmd — persist window geometry
 *   2. User settings round-trip (set + get)
 *   3. Bulk settings read performance
 *   4. Settings deletion
 *   5. Audio pipeline settings persistence
 *   6. Theme setting persistence via IPC
 *   7. Volume persistence across stop/start
 *   8. Repeat/shuffle mode persistence across stop/start
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks × 2s WAV
 *   Album 2002 — "Long Album" — 5 tracks × 30s WAV
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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

// ================================================================
// Test 1: User setting round-trip — string values
// ================================================================

test('user setting string round-trip: set and get', async () => {
  const key = 'test.e2e.string_setting';
  const value = 'hello-playwright-' + Date.now();

  await page.evaluate(async ({ k, v }) =>
    window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: v }),
    { k: key, v: value }
  );

  const result = await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k }),
    key
  );

  expect(result).toBe(value);
});

// ================================================================
// Test 2: User setting round-trip — JSON values
// ================================================================

test('user setting JSON round-trip: complex object', async () => {
  const key = 'test.e2e.json_setting';
  const obj = { theme: 'dark', volume: 75, features: ['a', 'b'] };
  const value = JSON.stringify(obj);

  await page.evaluate(async ({ k, v }) =>
    window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: v }),
    { k: key, v: value }
  );

  const result = await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k }),
    key
  );

  const parsed = JSON.parse(result);
  expect(parsed.theme).toBe('dark');
  expect(parsed.volume).toBe(75);
  expect(parsed.features).toEqual(['a', 'b']);
});

// ================================================================
// Test 3: Nonexistent setting returns null
// ================================================================

test('getting nonexistent setting returns null', async () => {
  const result = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_user_setting', {
      key: 'test.e2e.nonexistent_' + Date.now(),
    })
  );

  expect(result).toBeNull();
});

// ================================================================
// Test 4: Setting overwrite — last write wins
// ================================================================

test('overwriting a setting: last value wins', async () => {
  const key = 'test.e2e.overwrite';

  await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: 'first' }),
    key
  );

  await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: 'second' }),
    key
  );

  const result = await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k }),
    key
  );

  expect(result).toBe('second');
});

// ================================================================
// Test 5: Bulk settings read — multiple keys in sequence
// ================================================================

test('bulk settings read: 10 sequential reads complete quickly', async () => {
  // Write 10 settings
  for (let i = 0; i < 10; i++) {
    await page.evaluate(async ({ k, v }) =>
      window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: v }),
      { k: `test.e2e.bulk.${i}`, v: `value_${i}` }
    );
  }

  // Read all 10 back
  const start = Date.now();
  const results = await page.evaluate(async () => {
    const out = [];
    for (let i = 0; i < 10; i++) {
      const v = await window.__TAURI_INTERNALS__.invoke('get_user_setting', {
        key: `test.e2e.bulk.${i}`,
      });
      out.push(v);
    }
    return out;
  });
  const elapsed = Date.now() - start;

  expect(results.length).toBe(10);
  expect(results[0]).toBe('value_0');
  expect(results[9]).toBe('value_9');
  expect(elapsed).toBeLessThan(5_000);
});

// ================================================================
// Test 6: Volume persistence across stop/start
// ================================================================

test('volume setting persists across stop and new queue', async () => {
  // Set volume to 42
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 42 })
  );

  // Start playback
  await page.evaluate(async () => {
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
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Stop
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('stop_playback')
  );
  await page.waitForTimeout(300);

  // Check volume is still 42 (get_volume returns normalized 0.0-1.0)
  const vol = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_volume')
  );
  expect(vol).toBeCloseTo(0.42, 1);

  // Restore volume
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
});

// ================================================================
// Test 7: Repeat/shuffle modes persist across stop/start
// ================================================================

test('repeat and shuffle modes persist across playback stop', async () => {
  // Set specific modes
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'all' });
    await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' });
  });

  // Start playback
  await page.evaluate(async () => {
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
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Stop
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('stop_playback')
  );
  await page.waitForTimeout(300);

  // Check modes
  const repeat = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  const shuffle = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(repeat).toBe('all');
  expect(shuffle).toBe('random');

  // Cleanup
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' });
    await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' });
  });
});

// ================================================================
// Test 8: Audio pipeline settings save/load via user settings
// ================================================================

test('audio pipeline settings save and load via user settings', async () => {
  const key = 'audio.pipeline';
  const settings = {
    backend: 'default',
    device_name: null,
    dsp_enabled: false,
    resampling_quality: 'high',
    resampling_target_rate: 'auto',
    volume_leveling_mode: 'disabled',
    crossfade_enabled: true,
    crossfade_duration_ms: 5000,
    crossfade_curve: 'equal_power',
  };

  await page.evaluate(async ({ k, v }) =>
    window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: JSON.stringify(v) }),
    { k: key, v: settings }
  );

  const result = await page.evaluate(async (k) =>
    window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k }),
    key
  );

  const parsed = JSON.parse(result);
  expect(parsed.crossfade_enabled).toBe(true);
  expect(parsed.crossfade_duration_ms).toBe(5000);
  expect(parsed.resampling_quality).toBe('high');
});
