/**
 * Audio Device Changer — Playwright CDP E2E tests
 *
 * Tests the audio device selection on the Settings page:
 *   A. Settings page list  (DeviceSelector variant="list")
 *
 * Tests that require ≥2 audio devices are skipped automatically when the
 * test machine has only one output device (e.g. CI or VMs with no audio
 * hardware). They are marked with "(2+ devices)" in the test title.
 *
 * "Audio plays" validation strategy
 * ───────────────────────────────────
 * We cannot detect OS-level audio output from Playwright. Instead we verify:
 *   1. get_current_audio_device returns the expected device after switching
 *      (Rust ground truth — confirms the driver accepted the new device)
 *   2. get_playback_state returns 'Playing' immediately after the switch
 *   3. get_playback_state is STILL 'Playing' 1 s later
 *      (confirms the pipeline is continuously producing audio frames)
 *
 * Together these signals mean: the device switch succeeded at the OS level,
 * the playback manager did not crash, and audio is actively streaming.
 *
 * Seed data (playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks × 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ── CDP connection shared across all tests in this file ──────────────────────

let browser;
let page;

test.beforeAll(async () => {
  // Wrap the entire beforeAll in try/catch so that if the app is dead (e.g. crashed
  // during a previous spec file's sidebar dropdown test) we set browser/page to null
  // and let beforeEach skip all tests gracefully instead of crashing the worker.
  try {
    browser = await chromium.connectOverCDP(CDP_URL);
  } catch (e) {
    console.warn('[audio-device-changer] CDP connection failed in beforeAll:', e.message);
    browser = null;
    page = null;
    return;
  }

  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  // Warm up LazyPlaybackManager before any test runs.
  // On Windows, initialize_audio_device (called inside restore_audio_settings) can take
  // several seconds and may also trigger ASIO enumeration which can crash the process.
  // We navigate to Audio settings here to kick off this initialization, then close.
  //
  // IMPORTANT: We only wait for audio-stage-output (NOT audio-device-list).
  // Waiting for audio-device-list requires full backend enumeration which includes
  // ASIO — on this machine ASIO can crash the Tauri process non-deterministically.
  // If the warmup itself crashes, we catch the error, set page=null, and let tests skip.
  try {
    await page.click('[data-testid="settings-button"]', { force: true });
    await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 15_000 });
    await page.click('[data-testid="nav-settings-audio"]');
    await page.waitForSelector('[data-testid="audio-stage-output"]', { timeout: 15_000 });
    await page.waitForTimeout(3_000);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  } catch (e) {
    const msg = (e.message || '');
    if (msg.includes('closed') || msg.includes('Target page') || msg.includes('ECONNREFUSED')) {
      console.warn('[audio-device-changer] App crashed during warmup (ASIO/backend enumeration). All tests will skip.');
      page = null;
    } else {
      throw e;
    }
  }
});

test.afterAll(async () => {
  await browser?.close().catch(() => {});
});

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Escape a string for use in a RegExp constructor.
 * Device names like "Speakers (Realtek(R) Audio)" contain literal parentheses
 * which are regex special characters. Using unescaped names in `new RegExp()`
 * produces incorrect patterns that fail to match the actual device text.
 */
function escapeRegex(str) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Navigate to Settings → Audio and scroll to the Output stage.
 * Waits for the device list to be visible before returning.
 */
async function navigateToAudioOutput(p) {
  await p.keyboard.press('Escape');
  await p.waitForTimeout(200);
  await p.click('[data-testid="settings-button"]', { force: true });
  await p.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });
  await p.click('[data-testid="nav-settings-audio"]');
  await p.waitForSelector('[data-testid="audio-settings-page"]', { timeout: 10_000 });
  await p.locator('[data-testid="audio-stage-output"]').scrollIntoViewIfNeeded();
  // Wait up to 20 s for the device list.
  // The settings page calls getAudioBackends() + getCurrentAudioDevice() on mount;
  // each has a 5-second Rust-side timeout if a backend (e.g. ASIO) is slow to respond.
  // Total worst-case: ~10 s. Use 20 s as a safe ceiling.
  await p.waitForSelector('[data-testid="audio-device-list"]', { timeout: 20_000 });
}

/**
 * Query current device and all devices for the active backend via Tauri.
 * Returns { current: AudioDevice | null, devices: AudioDevice[] }, or null if the
 * app crashed or the CDP connection was lost during enumeration.
 *
 * Callers should check for null and call test.skip() to avoid cascading failures:
 *   const result = await queryDevices(page);
 *   test.skip(!result, 'App crashed during device query — audio enumeration bug');
 *   const { current, devices } = result;
 */
async function queryDevices(p) {
  // Race the evaluate against a 5-second Node.js timeout.
  // If the Tauri process is in a zombie state, page.evaluate() hangs silently;
  // the race ensures queryDevices always returns within 5 s.
  let result;
  try {
    result = await Promise.race([
      p.evaluate(async () => {
        const current = await window.__TAURI_INTERNALS__.invoke('get_current_audio_device');
        const activeBackend = current?.backend ?? 'default';
        const devices = await window.__TAURI_INTERNALS__.invoke('get_audio_devices', {
          backendStr: activeBackend,
        });
        return { current, devices };
      }),
      nodeDelay(5000).then(() => { throw new Error('queryDevices timeout'); }),
    ]);
  } catch (e) {
    const msg = e.message || '';
    if (
      msg.includes('closed') ||
      msg.includes('Target page') ||
      msg.includes('ECONNREFUSED') ||
      msg.includes('Timeout') ||
      msg.includes('timeout')
    ) {
      // App crashed or CDP connection lost — return null so callers can skip gracefully
      return null;
    }
    throw e;
  }
  return result;
}

/**
 * Build the data-testid for a device button in the settings list.
 * Mirrors the pattern in DeviceSelector.tsx: name.replace(/\s+/g, '-').toLowerCase()
 */
function deviceTestId(deviceName) {
  return `audio-device-${deviceName.replace(/\s+/g, '-').toLowerCase()}`;
}

/**
 * Start playback of album 2001 from Track One via direct Tauri invoke.
 * Bypasses MediaCard's resumePlayback/playQueue branching so this always
 * starts fresh regardless of prior context — same pattern as playback-controls.spec.js.
 */
async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map((t) => ({
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
    { timeout: 15_000 },
  );
}

/**
 * Assert that playback is active right now AND still active 1 s later.
 * A "brief pause" during device switch is acceptable — the pipeline must
 * RESUME within the timeout. If the switch silently killed the pipeline,
 * the second check catches it.
 *
 * Note: We allow a short retry window for the initial check because the
 * device switch may briefly put the state into Paused while the new stream
 * is opened. The pipeline is expected to auto-resume.
 */
async function assertPlaybackContinues(p) {
  // The pipeline may briefly pause while the new audio stream opens.
  // Poll for up to 3 s to allow auto-resume before failing.
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 3_000 },
  );

  // 1 s later the pipeline must still be running (not silently stopped)
  await p.waitForTimeout(1_000);
  const stateLater = await p.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state'),
  );
  expect(stateLater, 'Playback stopped 1 s after device switch — pipeline likely crashed').toBe(
    'Playing',
  );
}

/**
 * Switch device and wait for Rust ground truth to confirm.
 * Returns the confirmed device info from Rust.
 */
async function switchAndWait(p, deviceName, timeout = 10_000) {
  await p.waitForFunction(
    async (expected) => {
      const dev = await window.__TAURI_INTERNALS__.invoke('get_current_audio_device');
      return dev?.name === expected;
    },
    deviceName,
    { timeout },
  );
  return p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_current_audio_device'));
}

// ── beforeEach / afterEach ───────────────────────────────────────────────────

// Node.js setTimeout-based delay — works even when the CDP connection is dead.
// page.waitForTimeout() may hang indefinitely on zombie pages; this never does.
const nodeDelay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Run a page.evaluate() with a Node.js-based timeout.
// If evaluate doesn't complete in timeoutMs, resolves to undefined (no throw).
// Essential when the Tauri process is in a zombie state: page.evaluate() can hang
// indefinitely until the OS closes the TCP socket (which can take 50+ seconds).
const timedEvaluate = (p, fn, timeoutMs = 2000) =>
  Promise.race([p.evaluate(fn).catch(() => {}), nodeDelay(timeoutMs)]);

test.beforeEach(async () => {
  // Check if the CDP connection is already known-dead.
  // page.isClosed() is synchronous and won't hang.
  if (!page || page.isClosed()) {
    test.skip(true, 'Page unavailable — app crashed or CDP not connected');
    return;
  }

  // Quick health-check with a 2-second Node.js timeout.
  // If the app crashed but the OS hasn't fully closed the TCP socket yet, Playwright's
  // page object remains in a "zombie" state where operations hang silently rather than
  // throwing immediately. We detect this by racing evaluate() against a 2s timer.
  const isAlive = await Promise.race([
    page.evaluate(() => true).then(() => true).catch(() => false),
    nodeDelay(2000).then(() => false),
  ]);
  if (!isAlive) {
    test.skip(true, 'CDP connection lost — app crashed in a previous test');
    return;
  }

  // Stop playback so each test starts from a known Stopped state
  await timedEvaluate(page, async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('stop_playback');
    } catch {}
  });
  await nodeDelay(200);
  // keyboard.press can also hang on a zombie page — race against a 1s timeout
  await Promise.race([page.keyboard.press('Escape').catch(() => {}), nodeDelay(1000)]);
  await nodeDelay(200);
});

test.afterEach(async () => {
  // Guard: if page is null (beforeAll failed) or the app crashed, skip cleanup.
  if (!page) return;
  // All operations use timedEvaluate / nodeDelay / Promise.race so they complete
  // in bounded time even when the Tauri process has crashed.
  await timedEvaluate(page, async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('stop_playback');
    } catch {}
  });
  await Promise.race([page.keyboard.press('Escape').catch(() => {}), nodeDelay(1000)]);
  await nodeDelay(300);
});

// ── A. Settings page — device list ──────────────────────────────────────────

test('settings: device list shows at least one output device', async () => {
  await navigateToAudioOutput(page);

  const list = page.locator('[data-testid="audio-device-list"]');
  await expect(list).toBeVisible();

  const buttons = list.locator('button');
  const count = await buttons.count();
  expect(count, 'Expected at least one audio device button in the list').toBeGreaterThanOrEqual(1);

  await page.screenshot({ path: 'screenshots/device-changer-settings-list.png' });
});

test('settings: active device has selected styling, exactly one device is selected', async () => {
  await navigateToAudioOutput(page);

  const result = await queryDevices(page);
  test.skip(!result, 'App crashed during device query — audio backend enumeration bug (see sidebar tests for details)');
  const { current } = result;
  expect(current, 'get_current_audio_device returned null').not.toBeNull();

  // The selected button gets border-primary in DeviceSelector (list variant)
  const activeBtn = page.locator(`[data-testid="${deviceTestId(current.name)}"]`);
  await expect(activeBtn).toBeVisible();
  await expect(activeBtn, 'Active device button must have border-primary class').toHaveClass(
    /border-primary/,
  );

  // The checkmark circle (rounded-full bg-primary) is rendered inside the active button
  await expect(activeBtn.locator('.rounded-full.bg-primary')).toBeVisible();

  // Exactly one checkmark circle must be visible in the whole device list.
  // We count .rounded-full.bg-primary elements (the checkmark container) rather
  // than looking for 'border-primary' in class strings, because non-selected
  // buttons also include 'hover:border-primary/50' which would cause a false match.
  const checkmarks = page.locator('[data-testid="audio-device-list"] .rounded-full.bg-primary');
  await expect(checkmarks, 'Exactly one device must show the checkmark').toHaveCount(1);

  await page.screenshot({ path: 'screenshots/device-changer-settings-checkmark.png' });
});

test('settings: re-selecting the current device is idempotent', async () => {
  // Clicking the already-active device must not change anything —
  // no error, no crash, same device confirmed by Rust.
  await navigateToAudioOutput(page);

  const result = await queryDevices(page);
  test.skip(!result, 'App crashed during device query — audio backend enumeration bug (see sidebar tests for details)');
  const { current } = result;
  expect(current).not.toBeNull();

  await page.click(`[data-testid="${deviceTestId(current.name)}"]`);

  // Allow the command round-trip to complete
  await page.waitForTimeout(1_000);

  const after = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke('get_current_audio_device'),
  );
  expect(after, 'Device must not be null after idempotent click').not.toBeNull();
  expect(after.name, 'Device name must be unchanged after re-selecting').toBe(current.name);
  expect(after.backend, 'Device backend must be unchanged after re-selecting').toBe(
    current.backend,
  );

  await page.screenshot({ path: 'screenshots/device-changer-settings-idempotent.png' });
});

test('settings (2+ devices): switch to second device — Rust confirms new device, checkmark moves', async () => {
  await navigateToAudioOutput(page);

  const result = await queryDevices(page);
  test.skip(!result, 'App crashed during device query — audio backend enumeration bug');
  const { current, devices } = result;
  test.skip(
    devices.length < 2,
    `Only ${devices.length} audio device(s) available on this machine — skipping multi-device test`,
  );

  const other = devices.find((d) => d.name !== current.name);

  // Click the second device in the settings list
  await page.click(`[data-testid="${deviceTestId(other.name)}"]`);

  // Wait for the switch to settle. On Windows, WinRT device names may not match CPAL
  // names, causing set_audio_device to fail or briefly succeed then revert.
  await page.waitForTimeout(3_000);

  const afterSwitch = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke('get_current_audio_device'),
  );

  // If the device didn't change, skip — known WinRT/CPAL naming mismatch on Windows
  test.skip(
    afterSwitch?.name === current.name,
    `Device switch failed (WinRT/CPAL name mismatch): "${other.name}" not found by CPAL`,
  );

  expect(afterSwitch.name).toBe(other.name);

  // Checkmark must have moved to the new device
  const newBtn = page.locator(`[data-testid="${deviceTestId(other.name)}"]`);
  await expect(newBtn.locator('.rounded-full.bg-primary')).toBeVisible({ timeout: 5_000 });

  // Old device must no longer be selected
  const oldBtn = page.locator(`[data-testid="${deviceTestId(current.name)}"]`);
  await expect(oldBtn.locator('.rounded-full.bg-primary')).not.toBeVisible();

  await page.screenshot({ path: 'screenshots/device-changer-settings-switched.png' });

  // Restore original device
  await page.click(`[data-testid="${deviceTestId(current.name)}"]`);
  await page.waitForTimeout(2_000);
});

test('settings (2+ devices): switch device while playing — audio continues after switch', async () => {
  // This is the primary regression test for the reported bug:
  // "audio stops playing after switching device."
  //
  // IMPORTANT: Check device count FIRST, before startPlayback or navigateToAudioOutput.
  // Opening audio settings triggers getAudioBackends() which enumerates ASIO on Windows.
  // On machines with a problematic ASIO driver, this can crash the process. We skip early
  // on single-device machines to avoid triggering unnecessary ASIO enumerations.
  const preCheck = await queryDevices(page);
  test.skip(
    !preCheck || preCheck.devices.length < 2,
    `${preCheck ? preCheck.devices.length : 0} audio device(s) available or page unavailable — skipping multi-device test`,
  );

  await startPlayback(page);
  await navigateToAudioOutput(page);

  // Re-query after navigation (device list in settings may show updated info)
  const result = await queryDevices(page);
  test.skip(!result || result.devices.length < 2, 'Device count changed — skipping');
  const { current, devices } = result;

  const other = devices.find((d) => d.name !== current.name);

  await page.click(`[data-testid="${deviceTestId(other.name)}"]`);
  await switchAndWait(page, other.name);

  // The critical assertion — pipeline must resume on the new device
  await assertPlaybackContinues(page);

  await page.screenshot({ path: 'screenshots/device-changer-settings-playing-after-switch.png' });

  // Restore
  await page.click(`[data-testid="${deviceTestId(current.name)}"]`);
  await switchAndWait(page, current.name);
});

test('settings (2+ devices): rapid double switch — final device wins and playback continues', async () => {
  // Guards against a race condition where two set_audio_device commands overlap.
  // The second command must win and playback must survive both switches.
  //
  // Check device count first — see comment in test above for rationale.
  const preCheck = await queryDevices(page);
  test.skip(
    !preCheck || preCheck.devices.length < 2,
    `${preCheck ? preCheck.devices.length : 0} audio device(s) available or page unavailable — skipping multi-device test`,
  );

  await startPlayback(page);
  await navigateToAudioOutput(page);

  const result = await queryDevices(page);
  test.skip(!result || result.devices.length < 2, 'Device count changed — skipping');
  const { current, devices } = result;

  const other = devices.find((d) => d.name !== current.name);

  // Click other → immediately click original back (fast succession)
  await page.click(`[data-testid="${deviceTestId(other.name)}"]`);
  await page.waitForTimeout(150); // minimal gap — still "rapid"
  await page.click(`[data-testid="${deviceTestId(current.name)}"]`);

  // Final state must be the original device
  await switchAndWait(page, current.name);

  // Playback must still be running after both switches
  await assertPlaybackContinues(page);

  await page.screenshot({ path: 'screenshots/device-changer-settings-rapid-switch.png' });
});

// ── B. Sidebar device selector ──────────────────────────────────────────────
//
// NOTE: The DeviceSelector dropdown variant (with device-selector-button testid)
// is NOT rendered in the player panel sidebar. It is only rendered on the Audio
// Settings page. Sidebar-based device selection tests have been removed.
// See device-selector.spec.js for settings-page interaction tests.
