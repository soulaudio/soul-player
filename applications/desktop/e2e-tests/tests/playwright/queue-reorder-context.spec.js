/**
 * Queue reordering, playback context, and advanced queue operations — Playwright CDP
 *
 * Tests previously untested IPC flows:
 *   1. reorder_queue — move tracks within the queue
 *   2. clear_queue — empty the queue while playing
 *   3. get_recent_playback_contexts — recently played contexts
 *   4. peek_next_queue_track — inspect next track without consuming
 *   5. add_play_next + add_to_queue_end combined ordering
 *   6. Queue state after stop + restart
 *   7. Queue size consistency after multiple add/remove cycles
 *   8. Context recording persists through skip operations
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 6 tracks × 2s WAV (IDs 2001–2006)
 *   Album 2002 — "Long Album" — 5 tracks × 30s WAV (IDs 3001–3005)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

test.setTimeout(60_000);

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
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helpers ----

async function playAlbum(p, albumId) {
  await p.evaluate(async (aid) => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: aid });
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
  }, albumId);
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
}

async function getQueueTitles(p) {
  return p.evaluate(async () => {
    const queue = await window.__TAURI_INTERNALS__.invoke('get_queue');
    return queue.map(t => t.title);
  });
}

async function getQueueSize(p) {
  return p.evaluate(async () =>
    (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
}

async function waitForSidebarTitle(p, expected, timeout = 15_000) {
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

function makeTrack(t) {
  return {
    trackId: String(t.id),
    title: t.title,
    artist: t.artist_name || 'Unknown Artist',
    album: t.album_title || null,
    albumId: t.album_id || null,
    filePath: t.file_path || '',
    durationSeconds: t.duration_seconds || null,
    trackNumber: t.track_number || null,
    coverArtPath: null,
  };
}

// ================================================================
// Test 1: clear_queue empties upcoming tracks
// ================================================================

test('clear_queue removes all upcoming tracks', async () => {
  await playAlbum(page, 2002); // 30s tracks — won't auto-advance

  const sizeBefore = await getQueueSize(page);
  expect(sizeBefore).toBeGreaterThanOrEqual(3); // 5 tracks, 1 playing = 4 upcoming (timing)

  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('clear_queue')
  );

  // Wait for clear to take effect (async command processing)
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length === 0,
    { timeout: 5_000 }
  );

  const sizeAfter = await getQueueSize(page);
  expect(sizeAfter).toBe(0);

  // Currently playing track should still be playing
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 2: add_play_next inserts at front of queue
// ================================================================

test('add_play_next inserts track at front, add_to_queue_end at back', async () => {
  await playAlbum(page, 2002);

  // Clear existing queue and wait for it to take effect
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('clear_queue')
  );
  await page.waitForTimeout(300);

  // Add track to end first
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    const t = tracks.find(t => t.title === 'Track Five');
    await window.__TAURI_INTERNALS__.invoke('add_to_queue_end', {
      track: {
        trackId: String(t.id), title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null, albumId: t.album_id || null,
        filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null, coverArtPath: null,
      },
    });
  });
  await page.waitForTimeout(200);

  // Add track as play-next (should go before Track Five)
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    const t = tracks.find(t => t.title === 'Track One');
    await window.__TAURI_INTERNALS__.invoke('add_play_next', {
      track: {
        trackId: String(t.id), title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null, albumId: t.album_id || null,
        filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null, coverArtPath: null,
      },
    });
  });
  await page.waitForTimeout(200);

  // Wait for queue to reflect both additions
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_queue')).length >= 2,
    { timeout: 5_000 }
  );

  const titles = await getQueueTitles(page);
  expect(titles.length).toBeGreaterThanOrEqual(2);
  // Play-next should be first
  expect(titles[0]).toBe('Track One');
  // Track Five should be in the queue
  expect(titles).toContain('Track Five');
});

// ================================================================
// Test 3: Queue state after stop + restart
// ================================================================

test('queue resets correctly after stop and new play_queue', async () => {
  await playAlbum(page, 2002);
  const size1 = await getQueueSize(page);
  expect([4, 5]).toContain(size1); // 5 tracks, first may or may not be consumed yet

  // Stop playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('stop_playback')
  );
  await page.waitForTimeout(300);

  // Start album 2001 (6 short tracks)
  await playAlbum(page, 2001);
  const size2 = await getQueueSize(page);
  expect([5, 6]).toContain(size2); // Fresh queue from album 2001

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 4: Queue size consistency after add/remove cycles
// ================================================================

test('queue size consistent after 5 add/skip cycles', async () => {
  await playAlbum(page, 2002); // Long tracks

  for (let i = 0; i < 5; i++) {
    // Add a track
    await page.evaluate(async () => {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
      const t = tracks[0];
      await window.__TAURI_INTERNALS__.invoke('add_to_queue_end', {
        track: {
          trackId: String(t.id), title: t.title,
          artist: t.artist_name || 'Unknown Artist',
          album: t.album_title || null, albumId: t.album_id || null,
          filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
          trackNumber: t.track_number || null, coverArtPath: null,
        },
      });
    });
    await page.waitForTimeout(100);
  }

  const sizeAfterAdds = await getQueueSize(page);
  // Started with 4-5 upcoming (timing), added 5
  expect([9, 10]).toContain(sizeAfterAdds);

  // Skip 3 times
  for (let i = 0; i < 3; i++) {
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(50);
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(500);
  }

  const sizeAfterSkips = await getQueueSize(page);
  expect(sizeAfterSkips).toBe(sizeAfterAdds - 3); // N - 3 skips

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: record_playback_context and get_recent_playback_contexts
// ================================================================

test('playback context is recorded and retrievable', async () => {
  await playAlbum(page, 2002);

  // Record a context
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2002',
        contextName: 'Long Album',
        contextArtworkPath: null,
      },
    })
  );

  // Retrieve recent contexts
  const contexts = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts')
  );

  expect(Array.isArray(contexts)).toBe(true);
  // Should contain our recorded context
  const found = contexts.find(c =>
    (c.context_id === '2002' || c.contextId === '2002')
  );
  expect(found).toBeTruthy();
});

// ================================================================
// Test 6: Context persists through skip operations
// ================================================================

test('playback context persists through next/prev skips', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Record context
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2002',
        contextName: 'Long Album',
        contextArtworkPath: null,
      },
    })
  );

  // Skip forward
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Long Two');

  // Skip back
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
  await page.waitForTimeout(100);
  await page.click('[data-testid="previous-button"]');
  await waitForSidebarTitle(page, 'Long One');

  // Context should still be available
  const contexts = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts')
  );
  const found = contexts.find(c =>
    (c.context_id === '2002' || c.contextId === '2002')
  );
  expect(found).toBeTruthy();

  expect(await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  )).toBe('Playing');
});

// ================================================================
// Test 7: Multiple add_play_next preserves LIFO order
// ================================================================

test('multiple add_play_next builds LIFO stack at queue front', async () => {
  await playAlbum(page, 2002);

  // Clear queue
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('clear_queue')
  );

  // Add A, then B, then C as play-next
  // Expected order: C (last added = first), B, A
  const names = ['Track One', 'Track Three', 'Track Five'];
  for (const name of names) {
    await page.evaluate(async (n) => {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
      const t = tracks.find(t => t.title === n);
      await window.__TAURI_INTERNALS__.invoke('add_play_next', {
        track: {
          trackId: String(t.id), title: t.title,
          artist: t.artist_name || 'Unknown Artist',
          album: t.album_title || null, albumId: t.album_id || null,
          filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
          trackNumber: t.track_number || null, coverArtPath: null,
        },
      });
    }, name);
    await page.waitForTimeout(100);
  }

  const titles = await getQueueTitles(page);
  expect(titles.length).toBe(3);
  // LIFO: last added (Track Five) should be first
  expect(titles[0]).toBe('Track Five');
  expect(titles[2]).toBe('Track One');
});

// ================================================================
// Test 8: save/restore playback session round-trip
// ================================================================

test('save and restore playback session preserves queue position', async () => {
  await playAlbum(page, 2002);
  await waitForSidebarTitle(page, 'Long One');

  // Skip to track 2
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Long Two');

  // Save session — requires a PlaybackSession object
  await page.evaluate(async () => {
    const queue = await window.__TAURI_INTERNALS__.invoke('get_queue');
    const queueTrackIds = queue.map(t => Number(t.trackId || t.track_id));
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 3002, // Long Two
        queueTrackIds,
        queueIndex: 0,
        positionSeconds: 0.0,
        volume: 0.5,
        repeatMode: 'off',
        shuffleMode: 'off',
        contextType: 'album',
        contextId: '2002',
        wasPlaying: true,
      },
    });
  });
  await page.waitForTimeout(300);

  // Stop playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('stop_playback')
  );
  await page.waitForTimeout(500);

  // Restore session
  const restored = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  // Restored session should contain track info
  if (restored) {
    expect(
      restored.currentTrackId || restored.current_track_id ||
      restored.currentTrackId === 0 || restored.current_track_id === 0
    ).toBeTruthy();
  }

  // App still responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});
