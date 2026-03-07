/**
 * Crossfade stress tests — Playwright CDP
 *
 * Exercises crossfade settings under stress conditions:
 *   1. Rapid enable/disable toggling during playback
 *   2. Duration cycling through all valid values during playback
 *   3. Curve cycling through all 5 curve types during playback
 *   4. Bulk settings changes (set_crossfade_settings) during playback
 *   5. Crossfade + seek interleaved operations
 *   6. Crossfade + skip (next/prev) interleaved operations
 *   7. Crossfade + pause/resume during active fade
 *   8. Concurrent crossfade setting changes (parallel IPC)
 *   9. Crossfade enabled through full queue auto-advance chain
 *  10. Crossfade settings persistence across play_queue restarts
 *  11. All curves × skip forward: each curve survives track transitions
 *  12. Extreme durations (0ms gapless and 10000ms max) during playback
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks × 2s WAV
 *   Album 2002 — "Long Album" — 5 tracks × 30s WAV (IDs 3001–3005)
 *   Album 2003 — "Marathon Album" — 10 tracks × 15s WAV (IDs 4001–4010)
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

test.setTimeout(120_000);

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

  // Warm up LazyPlaybackManager
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('get_crossfade_settings'); } catch {}
  });
  await page.waitForTimeout(3000);
});

test.afterAll(async () => {
  // Reset crossfade to defaults
  await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
        enabled: false, durationMs: 3000, curve: 'equal_power',
      });
    } catch {}
  }).catch(() => {});
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try {
      await window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
        enabled: false, durationMs: 3000, curve: 'equal_power',
      });
    } catch {}
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

const ALL_CURVES = ['linear', 'square_root', 's_curve', 'equal_power'];

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

async function playLongAlbum(p) {
  return playAlbum(p, 2002);
}

async function getPlaybackState(p) {
  return p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
}

async function getCrossfadeSettings(p) {
  return p.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_crossfade_settings')
  );
}

async function waitForSidebarTitle(p, expected, timeout = 30_000) {
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
  return p.evaluate(() => {
    const container = document.querySelector('[data-testid="now-playing-title"]');
    if (!container) return null;
    const titleEl = container.querySelector('.text-sm');
    return titleEl ? titleEl.textContent.trim() : null;
  });
}

// ================================================================
// Test 1: Rapid enable/disable toggling during playback
// ================================================================

test('rapid crossfade toggle: 20 enable/disable cycles during playback', async () => {
  await playLongAlbum(page);
  await waitForSidebarTitle(page, 'Long One');

  const start = Date.now();

  for (let i = 0; i < 20; i++) {
    await page.evaluate(async (en) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: en }),
      i % 2 === 0
    );
    await page.waitForTimeout(50);
  }

  const elapsed = Date.now() - start;
  expect(elapsed).toBeLessThan(10_000);

  // Verify final state matches last write (i=19 → enabled: false)
  const settings = await getCrossfadeSettings(page);
  expect(settings.enabled).toBe(false);

  // Playback survived
  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');
});

// ================================================================
// Test 2: Duration cycling through valid values during playback
// ================================================================

test('crossfade duration cycling: 10 different durations during playback', async () => {
  await playLongAlbum(page);

  // Enable crossfade first
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: true })
  );

  const durations = [0, 500, 1000, 2000, 3000, 5000, 7000, 10000, 1500, 250];

  for (const ms of durations) {
    await page.evaluate(async (d) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_duration', { durationMs: d }), ms
    );
    await page.waitForTimeout(100);

    const actual = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_crossfade_duration')
    );
    expect(actual).toBe(ms);
  }

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 3: Curve cycling through all 5 types during playback
// ================================================================

test('crossfade curve cycling: all 4 curves set and verified during playback', async () => {
  await playLongAlbum(page);

  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: true })
  );

  for (const curve of ALL_CURVES) {
    await page.evaluate(async (c) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_curve', { curve: c }), curve
    );
    await page.waitForTimeout(100);

    const actual = await page.evaluate(async () =>
      window.__TAURI_INTERNALS__.invoke('get_crossfade_curve')
    );
    expect(actual).toBe(curve);
  }

  // Rapid cycle 3 more times
  for (let round = 0; round < 3; round++) {
    for (const curve of ALL_CURVES) {
      await page.evaluate(async (c) =>
        window.__TAURI_INTERNALS__.invoke('set_crossfade_curve', { curve: c }), curve
      );
      await page.waitForTimeout(30);
    }
  }

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 4: Bulk settings changes during playback
// ================================================================

test('bulk crossfade settings: 10 set_crossfade_settings calls during playback', async () => {
  await playLongAlbum(page);

  const configs = [
    { enabled: true, durationMs: 1000, curve: 'linear' },
    { enabled: true, durationMs: 5000, curve: 'equal_power' },
    { enabled: false, durationMs: 3000, curve: 's_curve' },
    { enabled: true, durationMs: 0, curve: 'linear' },
    { enabled: true, durationMs: 10000, curve: 'exponential' },
    { enabled: true, durationMs: 2000, curve: 'square_root' },
    { enabled: false, durationMs: 500, curve: 'equal_power' },
    { enabled: true, durationMs: 7000, curve: 's_curve' },
    { enabled: true, durationMs: 3000, curve: 'exponential' },
    { enabled: true, durationMs: 1500, curve: 'equal_power' },
  ];

  for (const cfg of configs) {
    await page.evaluate(async (c) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', c), cfg
    );
    await page.waitForTimeout(100);
  }

  // Verify last config stuck
  const settings = await getCrossfadeSettings(page);
  expect(settings.enabled).toBe(true);
  expect(settings.duration_ms || settings.durationMs).toBe(1500);

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 5: Crossfade + seek interleaved
// ================================================================

test('crossfade settings + seek interleaved: 10 cycles without crash', async () => {
  await playLongAlbum(page);
  await waitForSidebarTitle(page, 'Long One');

  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: true })
  );

  for (let i = 0; i < 10; i++) {
    // Change crossfade setting
    const curve = ALL_CURVES[i % ALL_CURVES.length];
    await page.evaluate(async (c) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_curve', { curve: c }), curve
    );

    // Seek to a position
    const seekPos = (i * 2.5) % 25;
    await page.evaluate(async (pos) =>
      window.__TAURI_INTERNALS__.invoke('seek_to', { position: pos }), seekPos
    );
    await page.waitForTimeout(150);
  }

  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');
});

// ================================================================
// Test 6: Crossfade + skip (next/prev) interleaved
// ================================================================

test('crossfade enabled + rapid next/prev: 8 skip operations survive', async () => {
  await playAlbum(page, 2003); // Marathon Album — 10 × 15s
  await waitForSidebarTitle(page, 'Marathon 01');

  // Enable crossfade with 2s duration
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 2000, curve: 'equal_power',
    })
  );

  // Skip forward 4 times
  for (let i = 0; i < 4; i++) {
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(100);
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(500);
  }

  // Should be on Marathon 05
  await waitForSidebarTitle(page, 'Marathon 05');

  // Skip back 2 times
  for (let i = 0; i < 2; i++) {
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('seek_to', { position: 0.0 }); } catch {}
    });
    await page.waitForTimeout(100);
    await page.click('[data-testid="previous-button"]');
    await page.waitForTimeout(500);
  }

  expect(await getPlaybackState(page)).toBe('Playing');
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
});

// ================================================================
// Test 7: Crossfade + pause/resume during active fade window
// ================================================================

test('pause and resume during crossfade window: no hang or crash', async () => {
  await playLongAlbum(page);
  await waitForSidebarTitle(page, 'Long One');

  // Enable crossfade with 5s duration
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 5000, curve: 'equal_power',
    })
  );

  // Seek near end to trigger crossfade window
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 26.0 })
  );
  await page.waitForTimeout(500);

  // Pause during the crossfade window
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('pause_playback')
  );
  await page.waitForFunction(
    async () => {
      const s = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return s === 'Paused';
    },
    { timeout: 5_000 }
  );

  // Wait while paused
  await page.waitForTimeout(2_000);

  // Resume
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('resume_playback')
  );
  await page.waitForFunction(
    async () => {
      const s = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return s === 'Playing';
    },
    { timeout: 5_000 }
  );

  // Wait for auto-advance to complete
  await page.waitForTimeout(6_000);

  // App must be responsive regardless of what track we're on
  await expect(page.locator('[data-testid="nav-albums"]')).toBeVisible();
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ================================================================
// Test 8: Concurrent crossfade setting changes (parallel IPC)
// ================================================================

test('concurrent crossfade IPC: 5 parallel setting changes resolve cleanly', async () => {
  await playLongAlbum(page);

  // Fire 5 setting changes in parallel
  const results = await page.evaluate(async () => {
    const promises = [
      window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
        enabled: true, durationMs: 1000, curve: 'linear',
      }),
      window.__TAURI_INTERNALS__.invoke('set_crossfade_duration', { durationMs: 2000 }),
      window.__TAURI_INTERNALS__.invoke('set_crossfade_curve', { curve: 's_curve' }),
      window.__TAURI_INTERNALS__.invoke('set_crossfade_enabled', { enabled: true }),
      window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
        enabled: true, durationMs: 4000, curve: 'exponential',
      }),
    ];
    const settled = await Promise.allSettled(promises);
    return settled.map(r => r.status);
  });

  // All should resolve (no rejects)
  for (const status of results) {
    expect(status).toBe('fulfilled');
  }

  // Settings should be in some valid final state
  const settings = await getCrossfadeSettings(page);
  expect(settings.enabled).toBe(true);
  expect(typeof (settings.duration_ms || settings.durationMs)).toBe('number');

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 9: Crossfade enabled through full queue auto-advance chain
// ================================================================

test('crossfade during auto-advance: 3 tracks advance with crossfade enabled', async () => {
  await playAlbum(page, 2003); // Marathon Album — 15s tracks
  await waitForSidebarTitle(page, 'Marathon 01');

  // Enable crossfade with 2s duration
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 2000, curve: 'equal_power',
    })
  );

  // Seek near end of track 1
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );
  await waitForSidebarTitle(page, 'Marathon 02', 30_000);

  // Seek near end of track 2
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );
  await waitForSidebarTitle(page, 'Marathon 03', 30_000);

  // Seek near end of track 3
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('seek_to', { position: 13.0 })
  );
  await waitForSidebarTitle(page, 'Marathon 04', 30_000);

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 10: Crossfade settings persistence across play_queue restarts
// ================================================================

test('crossfade settings persist across play_queue restarts', async () => {
  // Set specific crossfade config
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 4500, curve: 'exponential',
    })
  );

  // Start playback
  await playLongAlbum(page);
  await page.waitForTimeout(1_000);

  // Verify settings
  let settings = await getCrossfadeSettings(page);
  expect(settings.enabled).toBe(true);
  expect(settings.duration_ms || settings.durationMs).toBe(4500);

  // Stop and restart
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('stop_playback')
  );
  await page.waitForTimeout(500);

  await playLongAlbum(page);
  await page.waitForTimeout(500);

  // Settings should still be the same
  settings = await getCrossfadeSettings(page);
  expect(settings.enabled).toBe(true);
  expect(settings.duration_ms || settings.durationMs).toBe(4500);

  expect(await getPlaybackState(page)).toBe('Playing');
});

// ================================================================
// Test 11: All curves × skip forward
// ================================================================

test('each curve type survives a track skip transition', async () => {
  for (const curve of ALL_CURVES) {
    // Set this curve
    await page.evaluate(async (c) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
        enabled: true, durationMs: 1000, curve: c,
      }), curve
    );

    // Play marathon album
    await playAlbum(page, 2003);
    await waitForSidebarTitle(page, 'Marathon 01');

    // Skip next
    await page.click('[data-testid="next-button"]');
    await waitForSidebarTitle(page, 'Marathon 02');

    // Verify playback survived
    expect(await getPlaybackState(page)).toBe('Playing');

    // Stop before next curve
    await page.evaluate(async () => {
      try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    });
    await page.waitForTimeout(300);
  }
});

// ================================================================
// Test 12: Extreme durations — 0ms gapless and 10000ms max
// ================================================================

test('extreme crossfade durations: 0ms and 10000ms during playback', async () => {
  await playLongAlbum(page);
  await waitForSidebarTitle(page, 'Long One');

  // Test 0ms (gapless)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 0, curve: 'linear',
    })
  );
  let settings = await getCrossfadeSettings(page);
  expect(settings.duration_ms || settings.durationMs).toBe(0);
  expect(await getPlaybackState(page)).toBe('Playing');

  await page.waitForTimeout(1_000);

  // Test 10000ms (maximum)
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('set_crossfade_settings', {
      enabled: true, durationMs: 10000, curve: 'equal_power',
    })
  );
  settings = await getCrossfadeSettings(page);
  expect(settings.duration_ms || settings.durationMs).toBe(10000);
  expect(await getPlaybackState(page)).toBe('Playing');

  await page.waitForTimeout(1_000);

  // Switch rapidly between extremes
  for (let i = 0; i < 5; i++) {
    const ms = i % 2 === 0 ? 0 : 10000;
    await page.evaluate(async (d) =>
      window.__TAURI_INTERNALS__.invoke('set_crossfade_duration', { durationMs: d }), ms
    );
    await page.waitForTimeout(200);
  }

  expect(await getPlaybackState(page)).toBe('Playing');
  expect(await getSidebarTitle(page)).toBe('Long One');
});
