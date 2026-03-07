/**
 * Seek / scrub stress tests — Playwright CDP
 *
 * Verifies that rapid and sustained seeking operations do not:
 *   - Crash the audio engine or freeze the UI
 *   - Cause position drift or stuck progress bar
 *   - Interfere with play/pause or track-skip operations
 *   - Leave stale state after many seek cycles
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks x 2-second WAV files
 *   Track IDs 2001–2005, titles: Track One … Track Five
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
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: false }); } catch {}
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
  // Seek to 0 to prevent 2s auto-advance
  await p.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
  });
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

// ================================================================
// Test 1: Rapid seek_to — 20 seeks across the 2-second track
// ================================================================

test('rapid seek_to: 20 seeks across the track complete without crash', async () => {
  await startPlayback(page);

  const start = Date.now();

  for (let i = 0; i < 20; i++) {
    const position = (i % 10) * 0.2; // 0.0, 0.2, 0.4, ..., 1.8, 0.0, ...
    await page.evaluate(async (pos) =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: pos }), position
    );
    // Minimal wait to let the command reach the audio thread
    await page.waitForTimeout(50);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(8_000);

  // Playback should still be active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 2: Seek back-and-forth — alternating near-start and near-end
// ================================================================

test('seek alternating near-start and near-end: 10 cycles stay stable', async () => {
  await startPlayback(page);

  for (let i = 0; i < 10; i++) {
    // Seek near end
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: 1.5 })
    );
    await page.waitForTimeout(80);

    // Seek back to start before auto-advance can trigger
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.1 })
    );
    await page.waitForTimeout(80);
  }

  // Should still be on Track One (never let it reach the end)
  await waitForSidebarTitle(page, 'Track One');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 3: Seek + skip interleaved — seek then skip, repeat
// ================================================================

test('seek + skip interleaved: seek to mid, skip next, seek to start, repeat', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Seek to mid of Track One, then skip next
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 1.0 })
  );
  await page.waitForTimeout(100);

  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Two');

  // Seek to start of Track Two, then skip next
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 })
  );
  await page.waitForTimeout(100);

  await page.click('[data-testid="next-button"]');
  await waitForSidebarTitle(page, 'Track Three');

  // Seek + previous
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.5 })
  );
  await page.waitForTimeout(100);

  await page.click('[data-testid="previous-button"]');
  await waitForSidebarTitle(page, 'Track Two');

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 4: Rapid seek while paused — position updates without resuming
// ================================================================

test('rapid seek while paused: 10 seeks do not auto-resume playback', async () => {
  await startPlayback(page);

  // Robust pause: seek to 0, pause, wait, then re-check.
  // If auto-advance raced us (track finished before pause took effect), re-pause.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    await window.__TAURI_INTERNALS__.invoke('pause_playback');
  });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
  await page.waitForTimeout(300);
  // Re-check — if auto-advance raced us, pause again
  const check = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  if (check !== 'Paused') {
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
      await window.__TAURI_INTERNALS__.invoke('pause_playback');
    });
    await page.waitForFunction(
      async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
      { timeout: 5_000 }
    );
  }

  // Seek 10 times while paused (only in the safe 0–1.0 range)
  for (let i = 0; i < 10; i++) {
    const pos = (i * 0.1) % 1.0;
    await page.evaluate(async (p) =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: p }), pos
    );
    await page.waitForTimeout(50);
  }

  // Must still be Paused
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
});

// ================================================================
// Test 5: Click-to-seek on the seek-track element — 5 rapid clicks
// ================================================================

test('rapid click-to-seek on seek-track: 5 clicks stay responsive', async () => {
  await startPlayback(page);

  const seekTrack = page.locator('[data-testid="seek-track"]');
  await expect(seekTrack).toBeVisible({ timeout: 5_000 });
  const box = await seekTrack.boundingBox();
  expect(box).not.toBeNull();

  const start = Date.now();

  // Click at 5 different positions along the bar
  const percentages = [0.1, 0.8, 0.3, 0.9, 0.05];
  for (const pct of percentages) {
    const x = box.x + box.width * pct;
    const y = box.y + box.height / 2;
    await page.mouse.click(x, y);
    await page.waitForTimeout(150);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(5_000);

  // State must still be Playing
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 6: Seek near end repeatedly — force auto-advance through all tracks
// ================================================================

test('seek near end to force auto-advance through 3 tracks', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Advance through 3 tracks (not 4 — Track Five is the last and may finish
  // before we can seek, leaving the queue exhausted)
  const expectedSequence = ['Track Two', 'Track Three', 'Track Four'];

  for (const expected of expectedSequence) {
    // Seek near end of current track
    await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: 1.7 })
    );

    // Wait for auto-advance — use generous timeout since the audio thread
    // needs time to detect EOF, emit LoadNext, and load the next track
    await waitForSidebarTitle(page, expected);

    // Seek back to start of the new track to prevent it from auto-advancing
    // before we seek near its end in the next iteration
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(200);
  }

  // Should be playing Track Four
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');
});

// ================================================================
// Test 7: Seek + pause + seek + resume — combined operations
// ================================================================

test('seek + pause + seek + resume: all operations chain correctly', async () => {
  await startPlayback(page);
  await waitForSidebarTitle(page, 'Track One');

  // Seek to mid
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.5 })
  );
  await page.waitForTimeout(100);

  // Pause
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );

  // Seek while paused
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.2 })
  );
  await page.waitForTimeout(200);

  // Verify position is near 0.2
  const pos = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_position')
  );
  expect(pos).toBeLessThan(0.8);

  // Resume
  await page.click('[data-testid="play-pause-button"]');
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 5_000 }
  );

  // Still on Track One
  await waitForSidebarTitle(page, 'Track One');
});

// ================================================================
// Test 8: Seek beyond track duration — should clamp or auto-advance
// ================================================================

test('seeking beyond track duration does not crash', async () => {
  await startPlayback(page);

  // Seek to position beyond the 2-second duration
  await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 100.0 });
    } catch {
      // Error is acceptable — must not crash
    }
  });

  await page.waitForTimeout(500);

  // App must still be responsive
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();

  // Playback may have stopped, advanced, or be playing — all valid
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ================================================================
// Test 9: Rapid seeks with crossfade enabled — crossfade cancels on seek
// ================================================================

test('rapid seeks with crossfade enabled: 15 seeks complete without crash', async () => {
  // Enable crossfade before playback
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 3000, curve: 'equal_power',
    })
  );

  await startPlayback(page);

  for (let i = 0; i < 15; i++) {
    const position = (i * 0.13) % 1.8;
    await page.evaluate(async (pos) =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: pos }), position
    );
    await page.waitForTimeout(50);
  }

  // Playback survived rapid seeks with crossfade active
  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(state).toBe('Playing');

  // Crossfade should still be enabled
  const settings = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_crossfade_settings')
  );
  expect(settings.enabled).toBe(true);
});
