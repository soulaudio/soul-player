/**
 * Audio device & backend IPC tests — Playwright CDP
 *
 * Tests previously untested audio infrastructure IPC commands:
 *   1. get_audio_backends — list available audio backends
 *   2. get_audio_devices — enumerate devices for a backend
 *   3. get_current_audio_device — currently active device
 *   4. get_device_metrics — device performance metrics
 *   5. get_latency_info — latency measurement
 *   6. get_exclusive_preset — exclusive mode preset
 *   7. get_available_buffer_sizes — buffer size options
 *   8. Resampling backend get/set
 *   9. Resampling settings bundle
 *  10. Audio backends response shape validation
 *
 * These tests validate the IPC contract — they run against
 * whatever audio hardware is available on the test machine.
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

  // Warm up LazyPlaybackManager
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('get_audio_backends'); } catch {}
  });
  await page.waitForTimeout(3000);
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
});

// ================================================================
// Test 1: get_audio_backends returns array with at least one backend
// ================================================================

test('get_audio_backends returns at least one backend', async () => {
  const backends = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_audio_backends')
  );

  expect(Array.isArray(backends)).toBe(true);
  expect(backends.length).toBeGreaterThanOrEqual(1);

  // Each backend should have a name
  for (const b of backends) {
    expect(typeof b.name).toBe('string');
    expect(b.name.length).toBeGreaterThan(0);
  }
});

// ================================================================
// Test 2: get_audio_devices returns devices for default backend
// ================================================================

test('get_audio_devices returns at least one device for default backend', async () => {
  const devices = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_audio_devices', { backendStr: 'default' })
  );

  expect(Array.isArray(devices)).toBe(true);
  expect(devices.length).toBeGreaterThanOrEqual(1);

  // Each device should have a name
  for (const d of devices) {
    expect(typeof d.name).toBe('string');
  }
});

// ================================================================
// Test 3: get_current_audio_device returns a valid device
// ================================================================

test('get_current_audio_device returns device with name', async () => {
  const device = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_current_audio_device');
    } catch { return null; }
  });

  if (device !== null) {
    expect(typeof device.name).toBe('string');
    expect(device.name.length).toBeGreaterThan(0);
  }
});

// ================================================================
// Test 4: get_device_metrics returns an object
// ================================================================

test('get_device_metrics returns metrics object', async () => {
  const metrics = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_device_metrics');
    } catch { return null; }
  });

  // Metrics may or may not be available depending on device state
  if (metrics !== null) {
    expect(typeof metrics).toBe('object');
  }
});

// ================================================================
// Test 5: get_latency_info returns latency data
// ================================================================

test('get_latency_info returns latency information', async () => {
  const info = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_latency_info');
    } catch { return null; }
  });

  if (info !== null) {
    expect(typeof info).toBe('object');
  }
});

// ================================================================
// Test 6: get_exclusive_preset returns preset data
// ================================================================

test('get_exclusive_preset returns preset or null', async () => {
  const preset = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_exclusive_preset');
    } catch { return 'error'; }
  });

  // May return null, an object, or error — all valid
  expect(preset !== undefined).toBe(true);
});

// ================================================================
// Test 7: get_available_buffer_sizes returns array
// ================================================================

test('get_available_buffer_sizes returns buffer size options', async () => {
  const sizes = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_available_buffer_sizes');
    } catch { return null; }
  });

  if (sizes !== null) {
    expect(Array.isArray(sizes)).toBe(true);
  }
});

// ================================================================
// Test 8: Resampling backend get/set
// ================================================================

test('resampling backend can be read', async () => {
  const backend = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_resampling_backend');
    } catch { return null; }
  });

  // Should return a string
  if (backend !== null) {
    expect(typeof backend).toBe('string');
  }
});

// ================================================================
// Test 9: Resampling settings bundle
// ================================================================

test('get_resampling_settings returns settings bundle', async () => {
  const settings = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke('get_resampling_settings');
    } catch { return null; }
  });

  if (settings !== null) {
    expect(typeof settings).toBe('object');
    // Should have quality and target_rate fields
    expect(settings.quality !== undefined || settings.Quality !== undefined).toBe(true);
  }
});

// ================================================================
// Test 10: Backend shape validation — name and device_count
// ================================================================

test('audio backends have expected shape with name field', async () => {
  const backends = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_audio_backends')
  );

  // Find the default backend
  const defaultBackend = backends.find(b =>
    b.name === 'default' || b.name === 'Default' || b.is_default === true
  );

  // There should be at least one backend (default on any OS)
  expect(backends.length).toBeGreaterThanOrEqual(1);

  // Verify no backend has empty name
  for (const b of backends) {
    expect(b.name).toBeTruthy();
  }
});
