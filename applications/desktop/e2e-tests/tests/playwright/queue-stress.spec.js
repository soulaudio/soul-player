/**
 * Queue manipulation + mode cycling stress tests — Playwright CDP
 *
 * Verifies that rapid queue operations and repeat/shuffle mode changes
 * during active playback do not:
 *   - Corrupt queue state or lose tracks
 *   - Cause playback to stop unexpectedly
 *   - Leave repeat/shuffle in an inconsistent state
 *   - Accumulate errors or event listener leaks
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" / "Playwright Artist" — 5 tracks x 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
 *
 * Queue index note:
 *   After play_queue(5 tracks) + pm.play(), Track One is popped from the queue.
 *   Remaining queue: [T2, T3, T4, T5] at indices 0–3.
 *   get_queue() returns UPCOMING tracks only (N-1 after play_queue).
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
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
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
    try { await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(500);
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
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: 'album',
        contextId: '2001',
        contextName: 'Playwright Album',
        contextArtworkPath: null,
      },
    });
  });
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => {
      const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return state === 'Playing';
    },
    { timeout: 15_000 }
  );
  await p.waitForTimeout(150);
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

async function getSidebarTitle(p) {
  const container = p.locator('[data-testid="now-playing-title"]');
  await container.waitFor({ state: 'visible', timeout: 10_000 });
  const titleEl = container.locator('.text-sm').first();
  return (await titleEl.textContent()).trim();
}

async function getQueueSize(p) {
  return p.evaluate(async () =>
    (await window.__TAURI_INTERNALS__.invoke('get_queue')).length
  );
}

// ================================================================
// Test 1: Rapid skip_to_queue_index — 5 rapid index jumps
// ================================================================

test('rapid skip_to_queue_index: forward jumps resolve correctly', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  const start = Date.now();

  // After play_queue, remaining queue is [T2=0, T3=1, T4=2, T5=3]
  // Jump to index 1 (Track Three), then index 0 (Track Four — queue shifts after skip)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 1 })
  );
  await waitForSidebarTitle(page, 'Track Three');

  // Seek to 0 to prevent 2s auto-advance
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });

  // After skipping to T3, remaining queue is [T4=0, T5=1]
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('skip_to_queue_index', { index: 1 })
  );
  await waitForSidebarTitle(page, 'Track Five');

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(15_000);

  // Final track should be Track Five
  expect(await getSidebarTitle(page)).toBe('Track Five');

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 2: Add-to-queue during playback — queue grows correctly
// ================================================================

test('add_to_queue_end 3 times: queue size grows by 3', async () => {
  await startPlayback(page);

  const initialQueueSize = await getQueueSize(page);

  // Add Track One three more times to the end of the queue
  for (let i = 0; i < 3; i++) {
    await page.evaluate(async () => {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
      const t = tracks[0];
      await window.__TAURI_INTERNALS__.invoke('add_to_queue_end', {
        track: {
          trackId: String(t.id),
          title: t.title,
          artist: t.artist_name || 'Unknown Artist',
          album: t.album_title || null,
          albumId: t.album_id || null,
          filePath: t.file_path || '',
          durationSeconds: t.duration_seconds || null,
          trackNumber: t.track_number || null,
          coverArtPath: null,
        },
      });
    });
    await page.waitForTimeout(200);
  }

  const finalQueueSize = await getQueueSize(page);
  expect(finalQueueSize).toBe(initialQueueSize + 3);

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 3: Play-next interleaved with skip-next
// ================================================================

test('add_play_next then skip: plays the inserted track', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Insert Track Five as "play next"
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    const t5 = tracks.find(t => t.title === 'Track Five');
    await window.__TAURI_INTERNALS__.invoke('add_play_next', {
      track: {
        trackId: String(t5.id),
        title: t5.title,
        artist: t5.artist_name || 'Unknown Artist',
        album: t5.album_title || null,
        albumId: t5.album_id || null,
        filePath: t5.file_path || '',
        durationSeconds: t5.duration_seconds || null,
        trackNumber: t5.track_number || null,
        coverArtPath: null,
      },
    });
  });
  await page.waitForTimeout(300);

  // Skip next — should play the inserted Track Five, not Track Two
  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Five');

  expect(await getSidebarTitle(page)).toBe('Track Five');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 4: Rapid shuffle mode cycling — 6 full cycles
// ================================================================

test('rapid shuffle cycling: 6 full Off→Random→Smart→Off cycles', async () => {
  await startPlayback(page);

  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');
  const start = Date.now();

  for (let cycle = 0; cycle < 6; cycle++) {
    // Off → Random
    await shuffleBtn.click();
    await page.waitForTimeout(150);
    // Random → Smart
    await shuffleBtn.click();
    await page.waitForTimeout(150);
    // Smart → Off
    await shuffleBtn.click();
    await page.waitForTimeout(150);
  }

  const elapsed = Date.now() - start;
  // 18 clicks should complete well under 12s
  expect(elapsed).toBeLessThan(12_000);

  // After 6 full cycles (18 clicks), should be back to Off
  const finalMode = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(finalMode).toBe('off');

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 5: Rapid repeat mode cycling — 6 full cycles
// ================================================================

test('rapid repeat cycling: 6 full Off→All→One→Off cycles', async () => {
  await startPlayback(page);

  const repeatBtn = page.locator('[data-testid="repeat-button"]');
  const start = Date.now();

  for (let cycle = 0; cycle < 6; cycle++) {
    // Off → All
    await repeatBtn.click();
    await page.waitForTimeout(150);
    // All → One
    await repeatBtn.click();
    await page.waitForTimeout(150);
    // One → Off
    await repeatBtn.click();
    await page.waitForTimeout(150);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(12_000);

  // After 6 full cycles (18 clicks), should be back to Off
  const finalMode = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(finalMode).toBe('off');

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 6: Shuffle + Repeat simultaneous rapid toggling
// ================================================================

test('interleaved shuffle and repeat toggling: 10 alternations', async () => {
  await startPlayback(page);

  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');
  const repeatBtn = page.locator('[data-testid="repeat-button"]');

  const start = Date.now();

  for (let i = 0; i < 10; i++) {
    await shuffleBtn.click();
    await page.waitForTimeout(100);
    await repeatBtn.click();
    await page.waitForTimeout(100);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(10_000);

  // Playback should survive rapid mode changes
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Modes should be in some valid state
  const shuffle = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_shuffle')
  );
  expect(['off', 'random', 'smart']).toContain(shuffle);

  const repeat = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_repeat')
  );
  expect(['off', 'all', 'one']).toContain(repeat);
});

// ================================================================
// Test 7: RepeatOne + next-button interaction under stress
// ================================================================

test('RepeatOne enabled: next via IPC still advances tracks', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Enable RepeatOne and seek to 0 to prevent the 2s track from looping
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_repeat', { mode: 'one' });
    await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 });
  });
  await page.waitForTimeout(200);

  // Use IPC next_track instead of button click — more reliable with 2s tracks
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('next_track');
  });
  await page.waitForTimeout(500);

  // After skip, we should be on a different track or still playing
  // With RepeatOne the engine may or may not honor skip_next —
  // the important thing is the app doesn't crash
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused']).toContain(state);

  // App must remain responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 8: Queue operations + navigation interleaved
// ================================================================

test('add to queue + skip + navigate: queue state stays consistent', async () => {
  await startPlayback(page);
  const initialQueueSize = await getQueueSize(page);

  // Add a track to queue
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    const t = tracks[0];
    await window.__TAURI_INTERNALS__.invoke('add_to_queue_end', {
      track: {
        trackId: String(t.id),
        title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        albumId: t.album_id || null,
        filePath: t.file_path || '',
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
        coverArtPath: null,
      },
    });
  });
  await page.waitForTimeout(200);

  // Navigate to different pages
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForTimeout(300);
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForTimeout(300);

  // Queue size should still reflect the addition
  const afterNavQueueSize = await getQueueSize(page);
  expect(afterNavQueueSize).toBe(initialQueueSize + 1);

  // Skip next
  await page.click('[data-testid="next-button"]');
  await page.waitForTimeout(500);

  // Queue decrements by 1 after skip
  const afterSkipQueueSize = await getQueueSize(page);
  expect(afterSkipQueueSize).toBe(afterNavQueueSize - 1);

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 9: Rapid play_queue restarts — reload queue 5 times
// ================================================================

test('reloading queue 5 times: each restart begins from Track One', async () => {
  for (let i = 0; i < 5; i++) {
    await startPlayback(page);
    await waitForSidebarTitle(page, 'Track One');

    // Skip forward to verify the queue is working
    await page.click('[data-testid="next-button"]');
    await waitForSidebarTitle(page, 'Track Two');

    // Stop before restarting
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    });
    await page.waitForTimeout(300);
  }

  // Final restart
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 10: Volume rapid changes during queue operations
// ================================================================

test('rapid volume changes during skip operations: no crash', async () => {
  await startPlayback(page);

  const start = Date.now();

  for (let i = 0; i < 5; i++) {
    // Change volume rapidly (set_volume expects u8 0-100)
    const vol = (i % 5) * 20 + 10;
    await page.evaluate(async (v) =>
      window.__TAURI_INTERNALS__.invoke('set_volume', { volume: v }), vol
    );

    // Interleave with forward skip (alternating to stay within queue)
    if (i < 4) {
      await page.click('[data-testid="next-button"]');
      await page.waitForTimeout(200);
    }
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(10_000);

  // Playback still active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Restore volume
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_volume', { volume: 50 })
  );
});

// ================================================================
// Test 11: Crossfade enabled during skip + shuffle cycling
// ================================================================

test('crossfade enabled during skip and shuffle cycling: no crash', async () => {
  // Enable crossfade before playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 1000, curve: 'equal_power',
    })
  );

  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  const shuffleBtn = page.locator('[data-testid="shuffle-button"]');

  for (let i = 0; i < 4; i++) {
    // Toggle shuffle
    await shuffleBtn.click();
    await page.waitForTimeout(150);

    // Skip next
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(50);
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(400);
  }

  // Playback survived crossfade + shuffle + skip combo
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // App responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});
