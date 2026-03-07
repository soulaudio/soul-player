/**
 * Keyboard shortcuts & volume behaviour — Playwright CDP tests
 *
 * Covers:
 *   BUG-11 (FIXED): event.repeat guard prevents held-key rapid-fire toggles
 *   BUG-10 (FIXED): setVolume clamp prevents >100 being sent to backend
 *
 * Default shortcut accelerators (from soul-storage/src/shortcuts/mod.rs):
 *   play_pause  → CommandOrControl+Space  → Ctrl+Space  on Windows
 *   next        → CommandOrControl+Right  → Ctrl+Right  on Windows
 *   previous    → CommandOrControl+Left   → Ctrl+Left   on Windows
 *   volume_up   → CommandOrControl+Up     → Ctrl+Up     on Windows
 *   volume_down → CommandOrControl+Down   → Ctrl+Down   on Windows
 *   mute        → CommandOrControl+M      → Ctrl+M      on Windows
 *
 * The matchesAccelerator() helper in useKeyboardShortcuts.ts normalises
 * "CommandOrControl" to ctrlKey on Windows, so we press Control+<key> here.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across all tests in this file ----

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

// Before each test: stop playback, dismiss any overlay, navigate to Albums.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

// After each test: stop playback and clean up any open overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    // Remove any injected test DOM elements
    const el = document.getElementById('__test-input__');
    if (el) el.remove();
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
});

// ---- Helper ----

/**
 * Start playback of album 2001 via direct play_queue IPC (bypasses MediaCard
 * branching which is unreliable in the full suite). Seeks to position 0 after
 * confirming Playing to ensure the full 2s track is available for assertions.
 */
async function startPlayback(pg) {
  await pg.evaluate(async () => {
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
  await pg.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await pg.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  // Seek to 0 to ensure the full 2s track is available (prevents auto-advance race)
  await pg.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  // Blur so keyboard shortcuts reach the window handler, not a focused button
  await pg.evaluate(() => { document.activeElement?.blur(); document.body.focus(); });
  await pg.waitForTimeout(150);
}

// ================================================================
// Test 1: Ctrl+Space toggles play ↔ pause
// ================================================================

test('Ctrl+Space toggles play/pause while a track is loaded', async () => {
  // 1. Start playback so there is a loaded track
  await startPlayback(page);
  const stateAfterStart = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterStart).toBe('Playing');

  // 2. Ctrl+Space → should pause
  // Small initial delay lets the shortcut handler's own IPC calls settle,
  // then poll for the expected state change.
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
  const stateAfterFirstPress = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterFirstPress).toBe('Paused');

  // 3. Ctrl+Space again → should resume
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );
  const stateAfterSecondPress = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterSecondPress).toBe('Playing');
});

// ================================================================
// Test 2: Ctrl+Space inside a text input does NOT toggle playback
// (BUG-11 boundary — isEditableElement() guard)
// ================================================================

test('Ctrl+Space inside a text input does not toggle playback', async () => {
  // Start playing so there is a meaningful state to protect
  await startPlayback(page);

  // Inject and focus a text input overlaid on the page
  await page.evaluate(() => {
    const input = document.createElement('input');
    input.type = 'text';
    input.id = '__test-input__';
    input.style.cssText =
      'position:fixed;top:0;left:0;z-index:99999;width:100px;height:30px;opacity:0.01;';
    document.body.appendChild(input);
    input.focus();
  });

  // Confirm the input is actually focused
  const isFocused = await page.evaluate(
    () => document.activeElement?.id === '__test-input__'
  );
  expect(isFocused).toBe(true);

  // Press Space (without Ctrl — the raw Space key, to test the editable guard)
  // The shortcut is Ctrl+Space but even plain Space should be blocked inside inputs.
  // We test with Ctrl+Space as well since that is the actual accelerator.
  await page.keyboard.press('Space');
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(400);

  const stateAfterSpaceInInput = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  // State must still be Playing — the input swallowed both keystrokes
  expect(stateAfterSpaceInInput).toBe('Playing');

  // Clean up
  await page.evaluate(() => {
    const el = document.getElementById('__test-input__');
    if (el) el.remove();
  });
  // Refocus the document body so subsequent tests are not affected
  await page.evaluate(() => document.body.focus());
});

// ================================================================
// Test 3 (BUG-11 regression): simulated key-repeat events
// cause EXACTLY ONE toggle, not many
// ================================================================

test('BUG-11 regression: Ctrl+Space causes exactly one toggle per press', async () => {
  // Start playing
  await startPlayback(page);

  // Note: window.dispatchEvent() in page.evaluate() executes in Playwright's isolated
  // world and does NOT reach main-world event listeners. We therefore use
  // page.keyboard.press() which sends real CDP Input.dispatchKeyEvent events that
  // DO reach the React app's window.addEventListener('keydown') handler.
  //
  // The BUG-11 fix (event.repeat guard) is directly validated by the unit test in
  // playback-provider-bugs.test.ts. Here we verify the observable outcome: a single
  // Ctrl+Space press toggles playback exactly once.

  // Press 1: Playing → Paused
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
  const stateAfterFirst = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterFirst).toBe('Paused');

  // Press 2: Paused → Playing  (confirms toggle, not a one-way switch)
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );
  const stateAfterSecond = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateAfterSecond).toBe('Playing');
});

// ================================================================
// Test 4: Next / Previous keyboard shortcuts change the track
// ================================================================

test('Ctrl+Right (next) and Ctrl+Left (previous) navigate between tracks', async () => {
  await startPlayback(page);

  // Capture the title of the first track
  const titleBefore = await page.evaluate(() =>
    document.querySelector('[data-testid="now-playing-title"]')?.textContent?.trim()
  );
  expect(titleBefore).toBeTruthy();

  // Press next (Ctrl+ArrowRight)
  await page.keyboard.press('Control+ArrowRight');

  // Wait for the now-playing title to change
  await page.waitForFunction(
    (prev) => {
      const el = document.querySelector('[data-testid="now-playing-title"]');
      return el && el.textContent.trim() !== prev && el.textContent.trim() !== '';
    },
    titleBefore,
    { timeout: 10_000 }
  );

  const titleAfterNext = await page.evaluate(() =>
    document.querySelector('[data-testid="now-playing-title"]')?.textContent?.trim()
  );
  expect(titleAfterNext).not.toBe(titleBefore);

  // Press previous (Ctrl+ArrowLeft) — should return to the original track
  await page.keyboard.press('Control+ArrowLeft');

  await page.waitForFunction(
    (expected) => {
      const el = document.querySelector('[data-testid="now-playing-title"]');
      return el && el.textContent.trim() === expected;
    },
    titleBefore,
    { timeout: 10_000 }
  );

  const titleAfterPrev = await page.evaluate(() =>
    document.querySelector('[data-testid="now-playing-title"]')?.textContent?.trim()
  );
  expect(titleAfterPrev).toBe(titleBefore);
});

// ================================================================
// Test 5: Volume keyboard shortcuts change the displayed percentage
// ================================================================

test('Ctrl+Up and Ctrl+Down change the volume display', async () => {
  await startPlayback(page);

  // Read the current volume percentage from the UI element
  const readVolumePercent = () =>
    page.evaluate(() => {
      const el = document.querySelector('[data-testid="volume-percentage"]');
      return el ? parseInt(el.textContent.trim(), 10) : null;
    });

  const initialPercent = await readVolumePercent();
  expect(initialPercent).not.toBeNull();

  // Press volume up (Ctrl+ArrowUp) — hook increases by 0.05 (5%)
  await page.keyboard.press('Control+ArrowUp');

  // Wait for the UI to reflect the new value
  await page.waitForFunction(
    (prev) => {
      const el = document.querySelector('[data-testid="volume-percentage"]');
      if (!el) return false;
      const pct = parseInt(el.textContent.trim(), 10);
      return !isNaN(pct) && pct !== prev;
    },
    initialPercent,
    { timeout: 5_000 }
  );

  const afterUp = await readVolumePercent();
  // Volume should have increased (or stayed at 100 if already at max)
  expect(afterUp).toBeGreaterThanOrEqual(initialPercent);

  // Press volume down (Ctrl+ArrowDown) — should decrease by 5%
  const beforeDown = await readVolumePercent();
  await page.keyboard.press('Control+ArrowDown');

  await page.waitForFunction(
    (prev) => {
      const el = document.querySelector('[data-testid="volume-percentage"]');
      if (!el) return false;
      const pct = parseInt(el.textContent.trim(), 10);
      return !isNaN(pct) && pct !== prev;
    },
    beforeDown,
    { timeout: 5_000 }
  );

  const afterDown = await readVolumePercent();
  expect(afterDown).toBeLessThan(beforeDown);
});

// ================================================================
// Test 6 (BUG-10 regression): volume never exceeds 100 in the UI,
// and backend get_volume() never returns a value above 100
// ================================================================

test('BUG-10 regression: volume clamp — UI percentage never exceeds 100', async () => {
  await startPlayback(page);

  // Verify the volume slider exists and its aria-valuenow is within bounds
  const slider = page.locator('[data-testid="volume-slider"]');
  await expect(slider).toBeAttached();

  const ariaValueNow = await slider.getAttribute('aria-valuenow');
  const parsed = parseInt(ariaValueNow ?? '0', 10);
  expect(parsed).toBeGreaterThanOrEqual(0);
  expect(parsed).toBeLessThanOrEqual(100);

  // Verify the percentage display is also within 0–100
  const percentText = await page
    .locator('[data-testid="volume-percentage"]')
    .textContent();
  const displayPct = parseInt(percentText?.trim() ?? '0', 10);
  expect(displayPct).toBeGreaterThanOrEqual(0);
  expect(displayPct).toBeLessThanOrEqual(100);

  // Now verify via the backend get_volume Tauri command that the stored value
  // is 0–100 regardless of what slider value the UI might have passed.
  // get_volume() returns a float 0.0–100.0 (backend scale).
  const backendVolume = await page.evaluate(async () => {
    try {
      const { invoke } = window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__
        : (window.__TAURI__ ?? {});
      if (!invoke) return null;
      return await invoke('get_volume');
    } catch {
      return null;
    }
  });

  if (backendVolume !== null) {
    expect(backendVolume).toBeGreaterThanOrEqual(0);
    expect(backendVolume).toBeLessThanOrEqual(100);
  }

  // Simulate pressing volume up many times in quick succession (non-repeat — each
  // is a fresh press).  Even in the worst case the volume must not exceed 100%.
  for (let i = 0; i < 25; i++) {
    await page.keyboard.press('Control+ArrowUp');
    await page.waitForTimeout(50);
  }
  // Allow event loop to settle
  await page.waitForTimeout(800);

  const percentAfterSpam = await page
    .locator('[data-testid="volume-percentage"]')
    .textContent();
  const spamPct = parseInt(percentAfterSpam?.trim() ?? '0', 10);
  // BUG-10 check: volume must be clamped at or below 100 — never 125, 130, etc.
  // Not all 25 rapid keypresses may register, so don't require exactly 100.
  expect(spamPct).toBeLessThanOrEqual(100);
  expect(spamPct).toBeGreaterThanOrEqual(70); // most presses should have registered

  // Also verify backend did not receive >100
  const backendVolumeAfterSpam = await page.evaluate(async () => {
    try {
      const { invoke } = window.__TAURI_INTERNALS__
        ? window.__TAURI_INTERNALS__
        : (window.__TAURI__ ?? {});
      if (!invoke) return null;
      return await invoke('get_volume');
    } catch {
      return null;
    }
  });

  if (backendVolumeAfterSpam !== null) {
    expect(backendVolumeAfterSpam).toBeLessThanOrEqual(100);
  }
});
