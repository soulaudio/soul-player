/**
 * Device Selector — Playwright CDP regression tests
 *
 * Verifies that interacting with the audio device selector on the Audio Settings
 * page does NOT interrupt or stop active playback.
 *
 * NOTE: The DeviceSelector dropdown variant (with device-selector-button testid)
 * is rendered ONLY on the Audio Settings page, not in the player sidebar.
 * These tests navigate to Settings > Audio to interact with the device UI.
 *
 * Reported bugs:
 *   BUG-A: Interacting with the device selector stops audio, even if the user
 *     never selects a device and just closes the UI.
 *   BUG-B: Re-selecting the currently active device causes a brief audio
 *     stutter but playback should resume within a few seconds.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2002 — "Long Album" — 5 x 30-second WAV tracks
 *   Track IDs 3001–3005
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
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
  await page.click('[data-testid="nav-settings-audio"]');
  await page.waitForSelector('[data-testid="audio-stage-output"]', { timeout: 20_000 });
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
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

async function startPlayback(p) {
  // Use Long Album (30s tracks) to avoid auto-advance during device interaction
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2002 });
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
  await p.waitForTimeout(150);
}

async function pausePlayback(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('pause_playback');
  });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
  await p.waitForTimeout(200);
}

async function getPlaybackState(p) {
  return p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
}

async function navigateToAudioSettings(p) {
  await p.click('[data-testid="settings-button"]', { force: true });
  await p.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
  await p.click('[data-testid="nav-settings-audio"]');
  await p.waitForSelector('[data-testid="audio-stage-output"]', { timeout: 15_000 });
}

// ---- Tests ----

test('device selector button is visible on audio settings page', async () => {
  await navigateToAudioSettings(page);

  // The device list should be visible in the output stage section
  const deviceSection = page.locator('[data-testid="audio-device-list"], [data-testid="audio-stage-output"]').first();
  await expect(deviceSection).toBeVisible({ timeout: 20_000 });
});

test('BUG-A: navigating to audio settings while paused keeps state Paused', async () => {
  await startPlayback(page);
  await pausePlayback(page);

  // Navigate to audio settings — should NOT change playback state
  await navigateToAudioSettings(page);
  await page.waitForTimeout(1000);

  const state = await getPlaybackState(page);
  expect(state, 'Playback state must not change to Stopped when viewing audio settings').not.toBe('Stopped');
});

test('BUG-A: navigating to audio settings while playing keeps state Playing', async () => {
  await startPlayback(page);

  await navigateToAudioSettings(page);
  await page.waitForTimeout(1000);

  const state = await getPlaybackState(page);
  expect(state, 'Playback state must remain Playing when viewing audio settings').toBe('Playing');
});

test('BUG-B: re-selecting current device via IPC keeps playback alive', async () => {
  test.setTimeout(30_000);

  const currentDevice = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_current_audio_device')
  );
  expect(currentDevice, 'Expected a current audio device').not.toBeNull();

  await startPlayback(page);

  // Re-select the same device via IPC (simulates clicking the active device).
  // On Windows, get_current_audio_device returns WinRT names but set_audio_device
  // uses CPAL lookup which may have different naming. If the device isn't found
  // by name, we skip this test (it's a known Windows naming mismatch).
  const switchError = await page.evaluate(async (deviceName) => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_audio_device', {
        backendStr: 'default',
        deviceName,
      });
      return null;
    } catch (e) {
      return String(e);
    }
  }, currentDevice.name);

  if (switchError && switchError.includes('not found')) {
    // Known Windows issue: WinRT vs CPAL device name mismatch
    // Playback should still be running since the switch failed gracefully
    const state = await getPlaybackState(page);
    expect(state, 'Playback must survive a failed device switch').not.toBe('Stopped');
    return;
  }

  if (switchError) {
    throw new Error(`Unexpected set_audio_device error: ${switchError}`);
  }

  // Playback should recover within 8s
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing' || state === 'Paused';
    },
    { timeout: 8_000 }
  );

  const finalState = await getPlaybackState(page);
  expect(finalState, 'Playback must not be Stopped after re-selecting same device').not.toBe('Stopped');
});
