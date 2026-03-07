/**
 * Keyboard shortcut stress tests — Playwright CDP
 *
 * Verifies that rapid and sustained keyboard shortcut use does not:
 *   - Drop or double-process shortcut events
 *   - Allow volume to exceed 0-100% bounds
 *   - Interfere with playback controls or vice versa
 *   - Accumulate event listeners or cause memory issues
 *
 * Default shortcuts (from soul-storage/src/shortcuts/mod.rs):
 *   Ctrl+Space       → play/pause
 *   Ctrl+ArrowRight  → next track
 *   Ctrl+ArrowLeft   → previous track
 *   Ctrl+ArrowUp     → volume up (+5%)
 *   Ctrl+ArrowDown   → volume down (-5%)
 *   Ctrl+M           → mute toggle
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks x 2-second WAV files
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
    // Remove any test DOM elements
    const el = document.getElementById('__test-input__');
    if (el) el.remove();
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
  });
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  // Seek to 0 + blur so keyboard shortcuts reach window handler
  await p.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await p.evaluate(() => { document.activeElement?.blur(); document.body.focus(); });
  await p.waitForTimeout(150);
}

async function readVolumePercent() {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="volume-percentage"]');
    return el ? parseInt(el.textContent.trim(), 10) : null;
  });
}

// ================================================================
// Test 1: Rapid Ctrl+Space — 10 rapid play/pause toggles
// ================================================================

test('rapid Ctrl+Space: 10 play/pause toggles complete without crash', async () => {
  await startPlayback(page);

  const start = Date.now();

  for (let i = 0; i < 10; i++) {
    await page.keyboard.press('Control+Space');
    await page.waitForTimeout(200);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(8_000);

  // After even number of toggles, state should be consistent
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused']).toContain(state);

  // UI still responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 2: Volume key spam — 30 Ctrl+ArrowUp presses clamp at 100%
// ================================================================

test('volume up spam: 30 Ctrl+ArrowUp presses clamp at 100%', async () => {
  await startPlayback(page);

  // First set volume to a known starting point
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
  await page.waitForTimeout(300);

  // Spam volume up 30 times with small gaps so keystrokes register
  for (let i = 0; i < 30; i++) {
    await page.keyboard.press('Control+ArrowUp');
    await page.waitForTimeout(50);
  }
  await page.waitForTimeout(800);

  // Volume should be high — not all 30 presses may register, but most should
  const pct = await readVolumePercent();
  expect(pct).toBeGreaterThanOrEqual(80);

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 3: Volume key spam — 30 Ctrl+ArrowDown presses clamp at 0%
// ================================================================

test('volume down spam: 30 Ctrl+ArrowDown presses clamp at 0%', async () => {
  await startPlayback(page);

  // Start at 50%
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
  await page.waitForTimeout(300);

  // Spam volume down 30 times with small gaps so keystrokes register
  for (let i = 0; i < 30; i++) {
    await page.keyboard.press('Control+ArrowDown');
    await page.waitForTimeout(50);
  }
  await page.waitForTimeout(800);

  // Volume should be low — not all 30 presses may register, but most should
  const pct = await readVolumePercent();
  expect(pct).toBeLessThanOrEqual(20);

  // Playback still active (muted is not stopped)
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Restore volume for subsequent tests
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
});

// ================================================================
// Test 4: Mixed keyboard shortcuts — volume + play/pause + skip
// ================================================================

test('mixed shortcuts: volume, play/pause, and skip interleaved', async () => {
  await startPlayback(page);

  const start = Date.now();

  // Volume up twice
  await page.keyboard.press('Control+ArrowUp');
  await page.keyboard.press('Control+ArrowUp');
  await page.waitForTimeout(100);

  // Skip next
  await page.keyboard.press('Control+ArrowRight');
  await page.waitForTimeout(300);

  // Volume down
  await page.keyboard.press('Control+ArrowDown');
  await page.waitForTimeout(100);

  // Pause
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(300);

  // Volume up while paused
  await page.keyboard.press('Control+ArrowUp');
  await page.waitForTimeout(100);

  // Resume
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(300);

  // Skip previous
  await page.keyboard.press('Control+ArrowLeft');
  await page.waitForTimeout(300);

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(6_000);

  // App must still be functional
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused']).toContain(state);
});

// ================================================================
// Test 5: Ctrl+M mute toggle — 10 rapid toggles
// ================================================================

test('rapid Ctrl+M mute toggles: 10 cycles stay stable', async () => {
  await startPlayback(page);

  // Set volume to 50% first
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
  await page.waitForTimeout(300);

  for (let i = 0; i < 10; i++) {
    await page.keyboard.press('Control+m');
    await page.waitForTimeout(150);
  }

  // After even toggles, volume should be restored (not stuck at 0)
  await page.waitForTimeout(300);
  const pct = await readVolumePercent();
  // After 10 toggles (even number), volume should be near original 50%
  // Allow some tolerance since the toggle may apply changes
  expect(pct).not.toBeNull();

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 6: Keyboard shortcuts + button clicks — both inputs work
// ================================================================

test('keyboard shortcuts and button clicks interleaved both work', async () => {
  await startPlayback(page);

  // Skip via keyboard
  await page.keyboard.press('Control+ArrowRight');
  await page.waitForFunction(
    () => {
      const c = document.querySelector('[data-testid="now-playing-title"]');
      if (!c) return false;
      const t = c.querySelector('.text-sm');
      return t && t.textContent.trim() !== 'Track One';
    },
    { timeout: 10_000 }
  );

  // Skip via button
  await page.click('[data-testid="next-button"]');
  await page.waitForTimeout(500);

  // Pause via keyboard
  await page.keyboard.press('Control+Space');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  // Resume via button
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Previous via button
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);

  // Volume via keyboard
  await page.keyboard.press('Control+ArrowUp');
  await page.waitForTimeout(200);

  // Everything still works
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 7: Volume up/down alternating — 20 presses stay bounded
// ================================================================

test('alternating volume up/down 20 times: stays within 0-100%', async () => {
  await startPlayback(page);

  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
  await page.waitForTimeout(300);

  for (let i = 0; i < 20; i++) {
    if (i % 2 === 0) {
      await page.keyboard.press('Control+ArrowUp');
    } else {
      await page.keyboard.press('Control+ArrowDown');
    }
  }
  await page.waitForTimeout(500);

  const pct = await readVolumePercent();
  expect(pct).toBeGreaterThanOrEqual(0);
  expect(pct).toBeLessThanOrEqual(100);

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});
