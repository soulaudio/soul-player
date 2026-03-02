/**
 * TrackMenu "Play Next" and "Add to Queue" — Playwright CDP tests
 *
 * Covers the TrackMenu actions available on each track row in the album detail
 * page track list:
 *
 *   1. "Play Next" menu item is visible in track options menu
 *   2. "Add to Queue" menu item is visible in track options menu
 *   3. Both menu items are visible for a different track (3rd row)
 *   4. "Play Next" adds a track immediately after the current track
 *   5. "Add to Queue" appends a track at the end of the queue
 *   6. Closing the track menu via Escape dismisses it
 *
 * How the TrackMenu works:
 *   - Each track row (data-testid="track-row") renders a TrackMenu in its last cell.
 *   - The menu trigger is a button with aria-label="Track options".
 *   - Menu trigger is hidden by default (opacity-0) and revealed on row hover
 *     via the group-hover Tailwind class.
 *   - Radix UI DropdownMenu renders items with role="menuitem".
 *   - "Play Next" maps to: commands.addPlayNext(track) -> invoke('add_play_next', { track })
 *   - "Add to Queue" maps to: commands.addToQueueEnd(track) -> invoke('add_to_queue_end', { track })
 *   - Both items are enabled on desktop (features.hasPlaybackContext = true).
 *   - "Play Next"/"Add to Queue" require active playback context — they still render
 *     on the desktop regardless of whether playback is active (feature flag only).
 *
 * Queue shape (from queue-operations.spec.js comments):
 *   - get_queue() returns the *upcoming* queue, excluding the currently playing track.
 *   - While Track One plays: get_queue().length === 4 (Tracks Two–Five remaining).
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — "Playwright Artist"
 *   Track IDs 2001–2005, titles: Track One … Track Five (2-second WAV files)
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

// ----------------------------------------------------------------
// beforeEach: stop playback, dismiss overlays, navigate to Albums,
// then navigate into the album 2001 detail page and wait for the track list.
// ----------------------------------------------------------------

test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to Albums list — use force:true so the click goes through even if a
  // backdrop overlay is still present from the previous test.
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Navigate into album 2001 detail page
  await navigateToAlbumDetail(page);

  // Wait for the track list to be fully rendered
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
});

// ----------------------------------------------------------------
// afterEach: stop playback and clean up any open overlays.
// ----------------------------------------------------------------

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ----------------------------------------------------------------
// Helper: navigate from the Albums list to the album detail page.
//
// The MediaCard title <p> inside the card has cursor-pointer and calls
// handleClick(). We click the title text "Playwright Album" directly —
// it is the most reliable selector since the card outer div has no onClick.
// ----------------------------------------------------------------

async function navigateToAlbumDetail(p) {
  await p.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  const card = p.locator('[data-testid="media-card-album-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();

  await p.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
}

// ----------------------------------------------------------------
// Helper: start album 2001 playback via direct IPC.
//
// Bypasses the MediaCard play button branching logic so we always
// start fresh from Track One regardless of prior state.
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

  // Wait until the now-playing panel appears (TrackChanged event received)
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });

  // Poll until the playback state is Playing
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );

  // Confirm Track One is loaded in the sidebar
  await p.waitForFunction(
    () => {
      const container = document.querySelector('[data-testid="now-playing-title"]');
      if (!container) return false;
      const titleEl = container.querySelector('.text-sm');
      if (!titleEl) return false;
      return titleEl.textContent.trim() === 'Track One';
    },
    { timeout: 10_000 }
  );

  // Wait for the play-pause button to be ready so the store has settled
  await p.waitForFunction(
    () => {
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  await p.waitForTimeout(150);
}

// ----------------------------------------------------------------
// Helper: hover a track row and open its TrackMenu dropdown.
//
// The menu trigger button is rendered with opacity-0 by default and
// becomes visible on group-hover. We use hover() + a short wait to
// ensure the CSS transition has completed before querying the button.
// ----------------------------------------------------------------

async function openTrackMenu(p, row) {
  await row.hover();
  await p.waitForTimeout(300);

  const menuBtn = row.getByRole('button', { name: /track options/i });
  await menuBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await menuBtn.click();
  await p.waitForTimeout(300);
}

// ----------------------------------------------------------------
// Helper: read the current now-playing title from the sidebar.
// ----------------------------------------------------------------

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

// ----------------------------------------------------------------
// Helper: wait for the sidebar now-playing title to become the expected value.
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
// Test 1: "Play Next" menu item is visible in track options menu
// ================================================================

test('"Play Next" menu item is visible in the track options menu for the first track', async () => {
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  const firstRow = trackRows.first();
  await firstRow.waitFor({ state: 'visible' });

  await openTrackMenu(page, firstRow);

  // The menu must contain a "Play Next" menu item
  const playNextItem = page.getByRole('menuitem', { name: /play next/i });
  await expect(playNextItem).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 2: "Add to Queue" menu item is visible in track options menu
// ================================================================

test('"Add to Queue" menu item is visible in the track options menu for the first track', async () => {
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  const firstRow = trackRows.first();
  await firstRow.waitFor({ state: 'visible' });

  await openTrackMenu(page, firstRow);

  // The menu must contain an "Add to Queue" menu item
  const addToQueueItem = page.getByRole('menuitem', { name: /add to queue/i });
  await expect(addToQueueItem).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 3: Both menu items are visible for a different track (3rd row)
// ================================================================

test('both "Play Next" and "Add to Queue" are visible in the menu for the 3rd track row', async () => {
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  // nth(2) is the 3rd row (0-indexed) = "Track Three"
  const thirdRow = trackRows.nth(2);
  await thirdRow.waitFor({ state: 'visible' });

  await openTrackMenu(page, thirdRow);

  const playNextItem = page.getByRole('menuitem', { name: /play next/i });
  const addToQueueItem = page.getByRole('menuitem', { name: /add to queue/i });

  await expect(playNextItem).toBeVisible({ timeout: 5_000 });
  await expect(addToQueueItem).toBeVisible({ timeout: 5_000 });
});

// ================================================================
// Test 4: "Play Next" inserts a track immediately after the current track
//
// With Track One playing (get_queue returns [Two, Three, Four, Five]),
// clicking "Play Next" on Track Three inserts it at position 0 of the
// upcoming queue: [Three, Two, Four, Five].
// Clicking next-button must advance to Track Three, not Track Two.
// ================================================================

test('"Play Next" on Track Three causes the next skip to play Track Three', async () => {
  // Start playback from Track One
  await startPlayback(page);

  // The album detail page may have been unloaded while playback started —
  // re-navigate if needed (startPlayback doesn't move the UI away from detail page
  // because it uses IPC, but the nav state may need confirming)
  const detailPageVisible = await page.locator('[data-testid="album-detail-page"]').isVisible();
  if (!detailPageVisible) {
    await page.click('[data-testid="nav-albums"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
    await navigateToAlbumDetail(page);
    await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  }

  // Confirm Track One is playing before we modify the queue
  const currentTitle = await getNowPlayingTitle(page);
  expect(currentTitle).toBe('Track One');

  // Capture baseline queue size (should be 4: Tracks Two–Five)
  const baselineQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
  expect(baselineQueueSize).toBeGreaterThanOrEqual(4);

  // Hover Track Three row and open its track menu
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  const trackThreeRow = trackRows.filter({ hasText: 'Track Three' });
  await trackThreeRow.waitFor({ state: 'visible', timeout: 5_000 });

  await openTrackMenu(page, trackThreeRow);

  // Click "Play Next"
  const playNextItem = page.getByRole('menuitem', { name: /play next/i });
  await playNextItem.waitFor({ state: 'visible', timeout: 5_000 });
  await playNextItem.click();
  await page.waitForTimeout(500);

  // Queue must now have one additional entry (Track Three moved to front of queue
  // but it was already in the queue, so the total increases by one because
  // add_play_next inserts a new entry at the head without removing the original)
  const newQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
  expect(newQueueSize).toBeGreaterThan(baselineQueueSize);

  // Click next — Track Three should play next (it was inserted at queue head)
  await page.click('[data-testid="next-button"]');

  // Wait for the now-playing title to become "Track Three"
  await waitForTitle(page, 'Track Three');

  const titleAfterNext = await getNowPlayingTitle(page);
  expect(titleAfterNext).toBe('Track Three');

  // Playback state must still be Playing
  await page.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
});

// ================================================================
// Test 5: "Add to Queue" appends a track at the end of the queue
//
// With Track One playing (upcoming queue: [Two, Three, Four, Five]),
// clicking "Add to Queue" on Track Five appends it as a 5th entry:
// [Two, Three, Four, Five, Five].
// The queue size must increase by 1.
// ================================================================

test('"Add to Queue" on Track Five increases the queue size by one', async () => {
  // Start playback from Track One
  await startPlayback(page);

  // Re-navigate to album detail if it is no longer visible
  const detailPageVisible = await page.locator('[data-testid="album-detail-page"]').isVisible();
  if (!detailPageVisible) {
    await page.click('[data-testid="nav-albums"]', { force: true });
    await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
    await navigateToAlbumDetail(page);
    await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  }

  // Confirm Track One is playing
  const currentTitle = await getNowPlayingTitle(page);
  expect(currentTitle).toBe('Track One');

  // Capture the current queue size
  const beforeQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
  expect(beforeQueueSize).toBeGreaterThanOrEqual(4);

  // Hover Track Five row and open its track menu
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  const trackFiveRow = trackRows.filter({ hasText: 'Track Five' });
  await trackFiveRow.waitFor({ state: 'visible', timeout: 5_000 });

  await openTrackMenu(page, trackFiveRow);

  // Click "Add to Queue"
  const addToQueueItem = page.getByRole('menuitem', { name: /add to queue/i });
  await addToQueueItem.waitFor({ state: 'visible', timeout: 5_000 });
  await addToQueueItem.click();
  await page.waitForTimeout(500);

  // Queue must now have exactly one more entry than before
  const afterQueueSize = await page.evaluate(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
  expect(afterQueueSize).toBe(beforeQueueSize + 1);

  // The last entry in the queue must be Track Five
  const lastQueueEntry = await page.evaluate(async () => {
    const queue = await window.__TAURI_INTERNALS__.invoke('get_queue');
    return queue[queue.length - 1];
  });
  expect(lastQueueEntry).not.toBeNull();
  // The queue entry title field reflects the track title
  expect(lastQueueEntry.title).toBe('Track Five');
});

// ================================================================
// Test 6: Pressing Escape closes the open track menu
// ================================================================

test('pressing Escape while the track options menu is open closes it', async () => {
  const trackRows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  const firstRow = trackRows.first();
  await firstRow.waitFor({ state: 'visible' });

  // Open the menu
  await openTrackMenu(page, firstRow);

  // Verify the menu is visible before dismissing
  const playNextItem = page.getByRole('menuitem', { name: /play next/i });
  await expect(playNextItem).toBeVisible({ timeout: 5_000 });

  // Press Escape to dismiss
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // The menu must be gone — the menuitem must no longer be visible
  await expect(playNextItem).not.toBeVisible({ timeout: 3_000 });
});
