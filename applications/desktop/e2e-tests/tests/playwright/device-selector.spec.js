/**
 * Device Selector — Playwright CDP regression tests
 *
 * Verifies that interacting with the audio device selector dropdown in the
 * player panel (sidebar) does NOT interrupt or stop active playback.
 *
 * Reported bugs:
 *   BUG-A: Clicking the device selector button stops audio immediately, even
 *     if the user never selects a device and just closes the dropdown.
 *   BUG-B: Re-selecting the currently active device causes a brief audio
 *     stutter but playback should resume within a few seconds.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 × 2-second WAV tracks
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

  // Warm up LazyPlaybackManager + audio device system before any test runs.
  // On Windows, initialize_audio_device takes >3s. If we skip this, the first
  // test to start playback may hit an unresponsive WebView2.
  // Pattern from audio-effects.spec.js.
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
  await page.click('[data-testid="nav-settings-audio"]');
  await page.waitForSelector('[data-testid="audio-device-section"]', { timeout: 20_000 });
  await page.waitForTimeout(3_000);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(500);
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
  }).catch(() => {});
  // Close any open dropdown
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
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
  // Small wait for React store to fully reflect Playing state
  await p.waitForTimeout(150);
}

async function pausePlayback(p) {
  // Seek to 0 BEFORE pausing to prevent 2s track from finishing during pause fade
  // (the fade keeps state=Playing, so process_audio can still advance the source to EOF)
  await p.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    await window.__TAURI_INTERNALS__.invoke('pause_playback');
  });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
  // Settle and re-check — if auto-advance raced the pause, re-pause
  await p.waitForTimeout(200);
  const state = await p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  if (state === 'Playing') {
    await p.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
      await window.__TAURI_INTERNALS__.invoke('pause_playback');
    });
    await p.waitForFunction(
      async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
      { timeout: 5_000 }
    );
    await p.waitForTimeout(200);
  }
}

async function getPlaybackState(p) {
  return p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
}

async function openDeviceDropdown(p) {
  await p.click('[data-testid="device-selector-button"]');
  // Wait for the portal-rendered dropdown content to appear in the DOM.
  // data-dropdown-menu is hardcoded in the component (no prop forwarding needed).
  await p.waitForSelector('[data-dropdown-menu]', { timeout: 8_000 });
}

// ---- Tests ----

test('device selector button is visible in player panel', async () => {
  await startPlayback(page);

  const btn = await page.$('[data-testid="device-selector-button"]');
  expect(btn, 'device-selector-button must be present in the player panel').not.toBeNull();
  expect(await btn.isVisible()).toBe(true);
});

test('opening device selector while paused keeps state Paused', async () => {
  await startPlayback(page);
  // Pause immediately to freeze the 2-second track; paused state is stable for assertions
  await pausePlayback(page);

  // Open dropdown — BUG-A: this used to change state to Stopped
  await openDeviceDropdown(page);

  // State must not have changed to Stopped (the original BUG-A).
  // Use polling to tolerate brief transitional states from auto-advance race.
  const stateAfterOpen = await getPlaybackState(page);
  expect(stateAfterOpen, 'State must not change to Stopped when dropdown opens').not.toBe('Stopped');

  // Close without selecting
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  const stateAfterClose = await getPlaybackState(page);
  expect(stateAfterClose, 'State must not be Stopped after closing without selecting').not.toBe('Stopped');
});

test('opening device selector while playing keeps state Playing', async () => {
  await startPlayback(page);

  // Open the dropdown immediately — state should remain Playing while dropdown is open.
  // BUG-A: this used to change state to Stopped.
  await openDeviceDropdown(page);

  const stateAfterOpen = await getPlaybackState(page);
  expect(stateAfterOpen, 'State must remain Playing when dropdown opens').toBe('Playing');

  // Close without selecting
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  const stateAfterClose = await getPlaybackState(page);
  expect(stateAfterClose, 'State must remain Playing after closing without selecting').toBe('Playing');
});

test('re-selecting current device resumes playing within 8s', async () => {
  // Get the current device BEFORE starting playback (before the dropdown interferes)
  const currentDevice = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_current_audio_device');
  });
  expect(currentDevice, 'Expected a current audio device to be set').not.toBeNull();
  const deviceName = currentDevice.name;

  await startPlayback(page);

  // Seek to 0 to ensure the full 2s track is available during the device switch
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });

  // Open the dropdown while Playing — do NOT pause first, since switch_device
  // while paused keeps state Paused. The user's bug is: while playing, clicking
  // a device causes a brief stutter then resumes. Test that here.
  await openDeviceDropdown(page);

  // Find the device menuitem by its visible text content.
  // Using role="menuitem" + hasText is more robust than data-testid which
  // requires Vite to hot-reload shared workspace packages.
  const deviceItem = page.locator('[role="menuitem"]').filter({ hasText: deviceName }).first();
  await deviceItem.waitFor({ timeout: 5_000 });

  // Click the same device (re-select) — calls switch_device IPC.
  // BUG-B: audio stutters briefly, then should resume Playing.
  await deviceItem.click();

  // switch_device may briefly transition through Stopped; poll until Playing resumes.
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 8_000 }
  );

  const finalState = await getPlaybackState(page);
  expect(finalState).toBe('Playing');
}, { timeout: 20_000 });
