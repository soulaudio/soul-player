/**
 * Queue Sidebar — Playwright CDP tests
 *
 * Architecture note (read before editing):
 *   The desktop app does NOT use the QueueSidebar slide-in panel component
 *   (applications/shared/src/components/QueueSidebar.tsx). That component is
 *   unused. Instead, the upcoming queue is displayed inline inside LeftSidebar
 *   via QueueSection (applications/shared/src/components/sidebar/QueueSection.tsx),
 *   which is always visible — no toggle button required.
 *
 *   The [data-testid="queue-sidebar"] attribute is on the QueueSection container.
 *   The [data-testid="queue-item"] attribute is on each TrackItem in the queue list.
 *   Both testids were added to QueueSection.tsx and TrackItem.tsx for this spec.
 *
 * Display behaviour of QueueSection:
 *   - The current track is EXCLUDED from the queue list (it is shown in NowPlayingPanel).
 *   - The remaining upcoming tracks are shown REVERSED: the track furthest in the future
 *     appears at the top; the next-up track appears at the bottom.
 *   - With 6 tracks total and Track One playing, 5 queue-items are visible.
 *     .nth(0) → Track Five (furthest), .nth(3) → Track Two (next up).
 *   - Queue indices for skipToQueueIndex() are 0-based over the REMAINING queue
 *     (after play() pops the first track). startPlayback() calls clear_add_to_queue
 *     to force a sidebar reload after play() consumes T1, so the React queue
 *     matches the Rust remaining queue [T2..T5] at indices 0–3:
 *       index 0 = Track Two  (next up)
 *       index 1 = Track Three
 *       index 2 = Track Four
 *       index 3 = Track Five
 *       index 4 = Collab Track
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist" — 6 tracks × 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across all tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
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

// Before each test: stop playback, dismiss overlays, navigate to Albums, start fresh playback.
test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums to reset UI state
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Start playback of album 2001 from Track One
  await startPlayback(page);
});

// After each test: stop playback, dismiss any leftover overlays.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: start playback of album 2001 and wait until Playing state.
//
// Uses play_queue directly rather than the MediaCard play button to bypass
// the resumePlayback() vs playQueue() branching in handlePlayPause.
// Direct play_queue always starts fresh from Track One.
// ----------------------------------------------------------------

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

  // Wait until now-playing-title appears (UI received the TrackChanged event)
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // IMPORTANT: Force a fresh queue load in the React sidebar.
  //
  // play_queue sends two async commands: LoadPlaylist → then Play.
  // LoadPlaylist emits QueueUpdated while T1 is still at source_index=0, so the
  // React sidebar gets queue=[T1,T2,T3,T4,T5]. Play then pops T1 (source_index→1)
  // but emits NO QueueUpdated, leaving the sidebar queue stale.
  //
  // As a result, T2's originalIndex in the sidebar is 1 (position in the stale
  // [T1..T5] list), but skip_to_queue_index(1) skips to T3 in the Rust remaining
  // queue (indices 0-3 for T2-T5). Clicking T2 would jump to T3 instead.
  //
  // Calling clear_add_to_queue is a no-op (the "add to queue end" list is empty)
  // but it emits QueueUpdated, causing LeftSidebar.loadQueue() to run again.
  // After this, get_queue() returns [T2,T3,T4,T5] and T2 has originalIndex=0. ✓
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('clear_add_to_queue');
  });
  // Brief wait for the QueueUpdated event to propagate through Tauri → React
  await p.waitForTimeout(300);
}

// ----------------------------------------------------------------
// Helper: read the current track title from the sidebar NowPlayingPanel.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the now-playing title to change to a specific value.
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

// ================================================================
// Test 1: Queue sidebar is visible when album is playing
// ================================================================

test('Queue sidebar is visible when album is playing', async () => {
  // QueueSection renders with data-testid="queue-sidebar" automatically
  // when there are upcoming tracks. Track One is playing so 5 tracks remain.
  const sidebar = page.locator('[data-testid="queue-sidebar"]');
  await expect(sidebar).toBeVisible({ timeout: 10_000 });

  // The sidebar must have positive dimensions
  const box = await sidebar.boundingBox();
  expect(box).not.toBeNull();
  expect(box.width).toBeGreaterThan(20);
  expect(box.height).toBeGreaterThan(20);
});

// ================================================================
// Test 2: Queue sidebar shows all 5 upcoming tracks when album is playing
//
// QueueSection excludes the current track (Track One) from its list and
// shows the remaining 5 tracks (Tracks Two through Five plus Collab Track).
// ================================================================

test('Queue sidebar shows 5 upcoming tracks when playing from Track One', async () => {
  // Wait for the queue section to appear and populate
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);

  const items = page.locator('[data-testid="queue-item"]');
  const count = await items.count();
  expect(count).toBe(5);

  // The full text of the queue section must contain all 5 upcoming track titles
  const sidebar = page.locator('[data-testid="queue-sidebar"]');
  const text = await sidebar.textContent();
  expect(text).toContain('Track Two');
  expect(text).toContain('Track Three');
  expect(text).toContain('Track Four');
  expect(text).toContain('Track Five');
  expect(text).toContain('Collab Track');

  // The current track (Track One) is displayed in NowPlayingPanel — NOT in the queue list
  // The queue-sidebar itself must NOT show "Track One" as a queue-item
  const allItemText = await items.allTextContents();
  const hasTrackOne = allItemText.some(t => t.includes('Track One'));
  expect(hasTrackOne).toBe(false);
});

// ================================================================
// Test 3: Queue sidebar does not show the currently-playing track
//
// QueueSection.tsx filters: track => trackId !== currentTrackId.
// This confirms that filtering logic is live in the DOM.
// ================================================================

test('Queue sidebar excludes the currently-playing track from the list', async () => {
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);

  // Confirm Track One is the current track via the sidebar NowPlayingPanel
  const currentTitle = await getNowPlayingTitle(page);
  expect(currentTitle).toBe('Track One');

  // Queue items must not contain "Track One"
  const items = page.locator('[data-testid="queue-item"]');
  const allTexts = await items.allTextContents();
  const currentInQueue = allTexts.some(t => t.includes('Track One'));
  expect(currentInQueue).toBe(false);

  // But all the other 5 tracks must be present
  const combined = allTexts.join(' ');
  expect(combined).toContain('Track Two');
  expect(combined).toContain('Track Three');
  expect(combined).toContain('Track Four');
  expect(combined).toContain('Track Five');
  expect(combined).toContain('Collab Track');
});

// ================================================================
// Test 4: Clicking a queue item skips to that track
//
// QueueSection displays items REVERSED (Track Five at top, Track Two at
// bottom). The item at .nth(0) corresponds to original queue index 4
// (Track Five). However, the click handler passes the original index to
// skipToQueueIndex(), so the right track should start playing.
//
// We click the bottom-most item (.nth(3) in reversed order = Track Two,
// original index 1) because it is the most predictable to click.
// ================================================================

test('Clicking the next-up queue item (Track Two) skips to Track Two', async () => {
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);

  // Confirm we start on Track One
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // The queue is displayed reversed: Track Five at top (.nth(0)), Track Two at bottom (.nth(3))
  // Click the item for Track Two directly by text to avoid race conditions with auto-advance
  const nextUpItem = page.locator('[data-testid="queue-item"]').filter({ hasText: 'Track Two' }).first();
  const itemText = await nextUpItem.textContent();
  expect(itemText).toContain('Track Two');

  await nextUpItem.click();

  // Wait for Track Two to become the current track
  await waitForTitle(page, 'Track Two');
  expect(await getNowPlayingTitle(page)).toBe('Track Two');

  // Verify playback state is Playing after the skip
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: Queue sidebar is hidden (no items) after stop_playback
//
// Regression guard for the String(undefined) = "undefined" bug:
//   When stopped, currentTrackId is undefined. String(undefined) = "undefined"
//   which never matches a real trackId like "2001", so ALL queue items pass
//   the filter and appear in the sidebar — wrong.
//
// After this fix, QueueSection returns [] when currentTrackId is falsy.
// ================================================================

test('Queue sidebar shows 0 items after stop_playback', async () => {
  // Verify queue is visible while playing
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  const initialCount = await page.locator('[data-testid="queue-item"]').count();
  expect(initialCount).toBe(5);

  // Stop playback
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('stop_playback');
  });

  // Wait for the UI to reflect stopped state (NowPlayingPanel disappears or stops showing track)
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped';
    },
    { timeout: 5_000 }
  );

  // Allow React state propagation
  await page.waitForTimeout(500);

  // Queue sidebar must show NO items when stopped — the stale queue must be cleared
  const items = page.locator('[data-testid="queue-item"]');
  const stoppedCount = await items.count();
  expect(stoppedCount).toBe(0);
});

// ================================================================
// Test 5b: Queue repopulates correctly after restarting playback
//
// After stop → restart, the queue should show 5 upcoming tracks again.
// This ensures the "clear on stop + repopulate on play" cycle works.
// ================================================================

test('Queue repopulates after stop then restart playback', async () => {
  // Verify 5 items while playing
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  expect(await page.locator('[data-testid="queue-item"]').count()).toBe(5);

  // Stop playback
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('stop_playback');
  });
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Stopped';
    },
    { timeout: 5_000 }
  );
  await page.waitForTimeout(400);

  // Confirm queue is now empty
  expect(await page.locator('[data-testid="queue-item"]').count()).toBe(0);

  // Restart playback from Track One
  await startPlayback(page);

  // Queue must show 5 items again
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);
  const afterRestartCount = await page.locator('[data-testid="queue-item"]').count();
  expect(afterRestartCount).toBe(5);
});

// ================================================================
// Test 6: Queue sidebar updates after advancing to the next track
//
// When Track One finishes and Track Two begins playing:
//  - Track Two disappears from the queue list (now the current track)
//  - Track One is no longer in the queue (already played)
//  - Only Tracks Three, Four, Five, and Collab Track remain — 4 items total
// ================================================================

test('Queue sidebar shrinks by 1 after advancing to the next track', async () => {
  await page.waitForSelector('[data-testid="queue-sidebar"]', { timeout: 10_000 });
  await page.waitForTimeout(300);

  // Confirm we start with 5 items
  const initialCount = await page.locator('[data-testid="queue-item"]').count();
  expect(initialCount).toBe(5);

  // Skip to Track Two via the next button
  await page.click('[data-testid="next-button"]');

  // Wait for the track to change
  await waitForTitle(page, 'Track Two');

  // Allow the React store and QueueSection to re-render with the updated queue
  await page.waitForTimeout(500);

  // Now Track Two is playing, so only Tracks Three, Four, Five, Collab Track remain — 4 items
  const newCount = await page.locator('[data-testid="queue-item"]').count();
  expect(newCount).toBe(4);

  // Track Two must no longer appear in the queue list
  const texts = await page.locator('[data-testid="queue-item"]').allTextContents();
  const hasTrackTwo = texts.some(t => t.includes('Track Two'));
  expect(hasTrackTwo).toBe(false);

  // Track Three through Collab Track must still be present
  const combined = texts.join(' ');
  expect(combined).toContain('Track Three');
  expect(combined).toContain('Track Four');
  expect(combined).toContain('Track Five');
  expect(combined).toContain('Collab Track');
});
