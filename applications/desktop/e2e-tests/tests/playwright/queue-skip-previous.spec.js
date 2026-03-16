/**
 * Queue skip-then-previous navigation — Playwright CDP tests
 *
 * BUG: When playing an album with a queue, clicking a track further in the
 * queue (e.g. Track Five) and then pressing "previous", the player jumps to
 * the track that was playing BEFORE the skip (Track One from history) instead
 * of going to the track before the skipped-to track in the queue (Track Four).
 * The skipped-over tracks (T2, T3, T4) also disappear from the visible queue
 * even though they were never played.
 *
 * Expected behaviour:
 *   Play T1 → click T5 in queue → previous → T4 plays, queue shows [T5]
 *   → previous again → T3 plays, queue shows [T4, T5]
 *
 * Actual behaviour:
 *   Play T1 → click T5 in queue → previous → T1 plays (from history!),
 *   queue shows only [T5] — T2, T3, T4 are gone from the visible queue.
 *
 * Root cause:
 *   skip_to_queue_index() advances source_index past all skipped tracks.
 *   previous() pops from history (which only has T1) instead of navigating
 *   backward through the source queue. go_back() only decrements by 1.
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 6 tracks x 2-second WAV files
 *   Track IDs 2001-2006, titles: Track One ... Track Five, Collab Track
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
  await page.waitForTimeout(300);
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
  // Seek to 0 to prevent auto-advance of 2s tracks
  await p.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  // Force queue sidebar refresh (play pops T1 without emitting QueueUpdated)
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('clear_add_to_queue');
  });
  await p.waitForTimeout(300);
}

async function getNowPlayingTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

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

async function getQueueTitles(p) {
  await p.waitForTimeout(300);
  const items = p.locator('[data-testid="queue-item"]');
  const count = await items.count();
  const titles = [];
  for (let i = 0; i < count; i++) {
    const text = await items.nth(i).textContent();
    // Extract just the track title (first line before artist)
    const match = text.match(/(Track \w+)/);
    if (match) titles.push(match[1]);
  }
  return titles;
}

async function getQueueSize(p) {
  const size = await p.evaluate(async () => {
    const q = await window.__TAURI_INTERNALS__.invoke('get_queue');
    return q.length;
  });
  return size;
}

// ================================================================
// Test 1: After skip_to_queue_index, previous should go to the
// track before the skipped-to track in the queue, not to history
// ================================================================

test('skip to Track Five then previous: should go to Track Four, not Track One', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Queue should be [T2, T3, T4, T5, Collab Track] — 5 tracks
  const initialQueueSize = await getQueueSize(page);
  expect(initialQueueSize).toBe(5);

  // Skip to Track Five (queue index 3)
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 3 });
  });
  await waitForTitle(page, 'Track Five');

  // Seek to 0 so previous() doesn't restart current track (>3s threshold)
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Press previous
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);

  // EXPECTED: Track Four (the track before Track Five in the queue)
  // ACTUAL BUG: Track One (popped from history — the track playing before the skip)
  const titleAfterPrev = await getNowPlayingTitle(page);
  expect(titleAfterPrev).toBe('Track Four');
});

// ================================================================
// Test 2: After skip, the skipped-over tracks should remain in queue
// ================================================================

test('skip to Track Five: skipped tracks T2-T4 should still be reachable via previous', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Skip to Track Five
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 3 });
  });
  await waitForTitle(page, 'Track Five');

  // Seek to 0 before each previous to avoid restart-current-track threshold
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Previous 1: T5 → T4
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  const t1 = await getNowPlayingTitle(page);
  expect(t1).toBe('Track Four');

  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Previous 2: T4 → T3
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  const t2 = await getNowPlayingTitle(page);
  expect(t2).toBe('Track Three');

  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Previous 3: T3 → T2
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  const t3 = await getNowPlayingTitle(page);
  expect(t3).toBe('Track Two');

  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Previous 4: T2 → T1
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  const t4 = await getNowPlayingTitle(page);
  expect(t4).toBe('Track One');
});

// ================================================================
// Test 3: After skip to Track Five, queue should still show T2-T4
// as upcoming tracks (they were skipped, not consumed)
// ================================================================

test('skip to Track Five: queue should still contain skipped tracks', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Before skip: queue = [T2, T3, T4, T5, Collab Track]
  const beforeSize = await getQueueSize(page);
  expect(beforeSize).toBe(5);

  // Skip to Track Five
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 3 });
  });
  await waitForTitle(page, 'Track Five');
  await page.waitForTimeout(300);

  // EXPECTED: Queue should be empty (T5 is now playing, it was the last track)
  // But if we press previous to go to T4, queue should show [T5]
  // The key assertion: the queue size via get_queue after skip should reflect
  // that skipped tracks are NOT consumed

  // T5 is now playing; Collab Track is still after T5 in the queue.
  // The bug is visible when pressing previous and losing the ability to go
  // through T4, T3, T2 in order.

  // Let's check the queue after skip (before pressing previous)
  const afterSkipSize = await getQueueSize(page);
  // T5 is playing, Collab Track remains after T5 — 1 track.
  expect(afterSkipSize).toBe(1);

  // Press previous — this is where the bug shows
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);

  const afterPrevTitle = await getNowPlayingTitle(page);
  // EXPECTED: Track Four, queue = [T5, Collab Track]
  expect(afterPrevTitle).toBe('Track Four');

  const afterPrevQueueSize = await getQueueSize(page);
  expect(afterPrevQueueSize).toBe(2); // [T5, Collab Track] remain
});

// ================================================================
// Test 4: Click queue item (not skip-to-end) then previous
//
// Play T1, click T3 in queue sidebar, then previous → should be T2
// ================================================================

test('click Track Three in queue then previous: should go to Track Two', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Skip to Track Three (queue index 1)
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 1 });
  });
  await waitForTitle(page, 'Track Three');

  // Seek to 0 to avoid restart threshold
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Press previous
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);

  // EXPECTED: Track Two (the track before T3 in queue order)
  // ACTUAL BUG: Track One (from history)
  const titleAfterPrev = await getNowPlayingTitle(page);
  expect(titleAfterPrev).toBe('Track Two');
});

// ================================================================
// Test 5: After next (not skip), previous works normally
//
// This is the baseline case that should already work:
// Play T1 → next → T2 → previous → T1
// ================================================================

test('baseline: next then previous returns to the original track', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Next → Track Two
  await page.click('[data-testid="next-button"]');
  await waitForTitle(page, 'Track Two');

  // Seek to 0
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);

  // Previous → Track One
  await page.click('[data-testid="previous-button"]');
  await waitForTitle(page, 'Track One');
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Queue should still have tracks after T1
  const queueSize = await getQueueSize(page);
  expect(queueSize).toBeGreaterThanOrEqual(3); // T2, T3, T4, T5 or fewer depending on timing
});

// ================================================================
// Test 6: Skip forward then previous multiple times preserves queue order
//
// Play T1 → skip to T4 → prev → T3 → prev → T2 → next → T3
// This verifies the queue remains navigable in both directions.
// ================================================================

test('skip to Track Four then navigate back and forward preserves queue', async () => {
  await startPlayback(page);
  expect(await getNowPlayingTitle(page)).toBe('Track One');

  // Skip to Track Four (queue index 2)
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 2 });
  });
  await waitForTitle(page, 'Track Four');

  // Prev → T3
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  expect(await getNowPlayingTitle(page)).toBe('Track Three');

  // Prev → T2
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(200);
  await page.click('[data-testid="previous-button"]');
  await page.waitForTimeout(500);
  expect(await getNowPlayingTitle(page)).toBe('Track Two');

  // Next → T3 (should go forward in queue, not to T4)
  await page.click('[data-testid="next-button"]');
  await waitForTitle(page, 'Track Three');
  expect(await getNowPlayingTitle(page)).toBe('Track Three');

  // Queue after T3 should contain [T4, T5, Collab Track]
  await page.waitForTimeout(300);
  const queueSize = await getQueueSize(page);
  expect(queueSize).toBe(3);
});
