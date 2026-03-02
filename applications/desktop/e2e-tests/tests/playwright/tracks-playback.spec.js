/**
 * Tracks page playback — Playwright CDP E2E tests
 *
 * Covers the all-tracks library view and playback flows initiated from it:
 *
 *   1. Tracks page loads and shows library tracks
 *   2. Double-clicking Track One starts playback
 *   3. Double-clicking Track Three starts playback from Track Three
 *   4. Tracks page shows all 5 seeded tracks
 *   5. Playback from tracks page: next button advances to next track
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — Artist "Playwright Artist"
 *   Track IDs 2001–2005, titles: Track One … Track Five (2-second WAV files)
 *
 * Key testids used:
 *   nav-tracks          — NavBar button that navigates to /tracks
 *   tracks-page         — outer container rendered by LibraryPageLayout (pageTestId="tracks-page")
 *   track-list          — the TrackList container div
 *   track-row           — each individual track row inside TrackList
 *   now-playing-title   — NowPlayingPanel track info container
 *   play-pause-button   — PlaybackControls play/pause button
 *   next-button         — PlaybackControls next-track button
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  // Find the main window — it is already loaded by the time tests run.
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  // Short safety wait in case there is any residual animation or settling
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Before each test: stop any active playback, dismiss open overlays, navigate to Tracks.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Tracks list — use force:true so the click goes through even if a
  // backdrop overlay is still present from the previous test.
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 15_000 });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
});

// After each test: stop playback and clean up any open overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: read the current track title from NowPlayingPanel.
// The now-playing-title container holds a TrackItem with nested spans;
// the title is in the first .text-sm element.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to become the expected value.
// ----------------------------------------------------------------

async function waitForTitle(p, expected, timeout = 15_000) {
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

// ----------------------------------------------------------------
// Test 1: Tracks page loads and shows library tracks
// ----------------------------------------------------------------

test('Tracks page loads and shows library tracks', async () => {
  // The tracks-page container must be visible (rendered by LibraryPageLayout with pageTestId="tracks-page")
  const tracksPage = page.locator('[data-testid="tracks-page"]');
  await expect(tracksPage).toBeVisible({ timeout: 10_000 });

  // The track-list container must also be visible
  const trackList = page.locator('[data-testid="track-list"]');
  await expect(trackList).toBeVisible({ timeout: 10_000 });

  // There must be at least 5 rows (the 5 seeded tracks)
  const trackRows = page.locator('[data-testid="track-row"]');
  const count = await trackRows.count();
  expect(count).toBeGreaterThanOrEqual(5);

  // "Track One" must appear somewhere in the list
  const trackOneRow = trackRows.filter({ hasText: 'Track One' });
  await expect(trackOneRow).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 2: Double-clicking Track One starts playback
// ----------------------------------------------------------------

test('Double-clicking Track One starts playback', async () => {
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Find the row containing "Track One" and double-click it
  const trackRows = page.locator('[data-testid="track-row"]');
  const trackOneRow = trackRows.filter({ hasText: 'Track One' });
  await trackOneRow.waitFor({ state: 'visible', timeout: 10_000 });
  await trackOneRow.dblclick();

  // Wait for the now-playing panel to appear with a track title
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Now-playing title must be "Track One"
  await waitForTitle(page, 'Track One');

  // Poll until state is Playing — manager.rs emits StateChanged(Stopped) briefly between
  // tracks during auto-advance (play_next_in_queue). If T1 just ended when we check,
  // a one-shot IPC call would see 'Stopped'; polling retries until T2 starts, confirming
  // that the double-click did trigger active playback.
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );
});

// ----------------------------------------------------------------
// Test 3: Double-clicking Track Three starts playback from Track Three
// ----------------------------------------------------------------

test('Double-clicking Track Three starts playback from Track Three', async () => {
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Find the row containing "Track Three" and double-click it
  const trackRows = page.locator('[data-testid="track-row"]');
  const trackThreeRow = trackRows.filter({ hasText: 'Track Three' });
  await trackThreeRow.waitFor({ state: 'visible', timeout: 10_000 });
  await trackThreeRow.dblclick();

  // Wait for the now-playing panel to appear
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Now-playing title must be "Track Three"
  // TrackList passes the clicked index as startIndex, so Track Three plays first
  await waitForTitle(page, 'Track Three');

  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});

// ----------------------------------------------------------------
// Test 4: Tracks page shows all 5 seeded tracks
// ----------------------------------------------------------------

test('Tracks page shows all 5 seeded tracks', async () => {
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  const trackRows = page.locator('[data-testid="track-row"]');

  // Must have at least 5 rows
  const count = await trackRows.count();
  expect(count).toBeGreaterThanOrEqual(5);

  // Each of the 5 seeded titles must be visible
  const expectedTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
  for (const title of expectedTitles) {
    const row = trackRows.filter({ hasText: title });
    await expect(row).toBeVisible({ timeout: 5_000 });
  }
});

// ----------------------------------------------------------------
// Test 5: Playback from tracks page: next button advances to next track
// ----------------------------------------------------------------

test('Playback from tracks page: next button advances to next track', async () => {
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Start playback by double-clicking Track One
  const trackRows = page.locator('[data-testid="track-row"]');
  const trackOneRow = trackRows.filter({ hasText: 'Track One' });
  await trackOneRow.waitFor({ state: 'visible', timeout: 10_000 });
  await trackOneRow.dblclick();

  // Wait for now-playing panel and Playing state
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Confirm we are on Track One before advancing
  await waitForTitle(page, 'Track One');
  const initialTitle = await getNowPlayingTitle(page);
  expect(initialTitle).toBe('Track One');

  // Click the next button in the playback controls
  await page.click('[data-testid="next-button"]');

  // Wait for the title to change to Track Two
  // Use waitForFunction polling so we catch the change as soon as it happens
  await page.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === 'Track Two';
    },
    { timeout: 15_000 }
  );

  const newTitle = await getNowPlayingTitle(page);
  expect(newTitle).toBe('Track Two');

  // State should still be Playing after skipping
  const state = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
  expect(state).toBe('Playing');
});
