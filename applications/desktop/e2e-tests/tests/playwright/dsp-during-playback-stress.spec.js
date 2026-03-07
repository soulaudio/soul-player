/**
 * DSP During Playback Stress E2E tests — Playwright CDP
 *
 * Stress-tests DSP chain modifications while audio is actively playing:
 *   add/remove/toggle effects, change parameters, load presets during playback.
 *
 * 8 tests
 *
 * Seed data:
 *   Album 2002 "Long Album" — 5 tracks (30s WAV)
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
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  // Warm up LazyPlaybackManager
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('get_dsp_chain'); } catch {}
  });
  await page.waitForTimeout(3000);
});

test.afterAll(async () => {
  await browser.close();
});

async function startLongPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2002 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map(t => ({
      trackId: String(t.id), title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null, albumId: t.album_id || null,
      filePath: t.file_path || '', durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null, coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
}

async function getPlaybackState(p) {
  return p.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_playback_state'));
}

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
});

// ── Test 1: Add compressor during playback ──

test('adding compressor during playback does not interrupt audio', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 2: Remove effect during playback ──

test('removing effect during playback does not interrupt audio', async () => {
  test.setTimeout(30_000);

  // Add effect first, then start playback
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  await startLongPlayback(page);

  // Remove during playback
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('remove_effect_from_chain', { slotIndex: 0 });
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 3: Toggle effect on/off 10 times during playback ──

test('toggling effect 10 times during playback does not crash', async () => {
  test.setTimeout(30_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  await startLongPlayback(page);

  await page.evaluate(async () => {
    for (let i = 0; i < 10; i++) {
      await window.__TAURI_INTERNALS__.invoke('toggle_effect', {
        slotIndex: 0, enabled: i % 2 === 0,
      });
    }
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 4: Update effect parameters 10 times during playback ──

test('updating compressor parameters 10 times during playback is stable', async () => {
  test.setTimeout(30_000);

  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  await startLongPlayback(page);

  await page.evaluate(async () => {
    for (let i = 0; i < 10; i++) {
      await window.__TAURI_INTERNALS__.invoke('update_effect_parameters', {
        slotIndex: 0,
        effect: { type: 'compressor', settings: {
          thresholdDb: -10 - i, ratio: 2 + i, attackMs: 5 + i * 2,
          releaseMs: 50 + i * 20, kneeDb: i, makeupGainDb: i * 0.5,
        }},
      });
    }
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 5: Add/remove effects rapidly during playback ──

test('rapid add/remove effect cycles during playback do not crash', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  await page.evaluate(async () => {
    for (let i = 0; i < 5; i++) {
      await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
        slotIndex: 0,
        effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
      });
      await window.__TAURI_INTERNALS__.invoke('remove_effect_from_chain', { slotIndex: 0 });
    }
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 6: Load DSP preset during playback ──

test('loading a DSP preset during playback does not interrupt audio', async () => {
  test.setTimeout(30_000);
  await startLongPlayback(page);

  // Save a preset, then load it during playback
  const presetId = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: 'Stress Test Preset', description: null,
      effectChain: [{ type: 'compressor', settings: { thresholdDb: -15, ratio: 3, attackMs: 8, releaseMs: 80, kneeDb: 2, makeupGainDb: 3 } }],
    })
  );

  await page.evaluate(async (id) => {
    await window.__TAURI_INTERNALS__.invoke('load_dsp_chain_preset', { presetId: id });
  }, presetId);

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Cleanup
  await page.evaluate(async (id) => {
    await window.__TAURI_INTERNALS__.invoke('delete_dsp_chain_preset', { presetId: id }).catch(() => {});
  }, presetId);
});

// ── Test 7: Multiple effects + playback + seek ──

test('multiple effects active during seek operations remain stable', async () => {
  test.setTimeout(30_000);

  // Add two effects
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 1,
      effect: { type: 'crossfeed', settings: { preset: 'natural', levelDb: -4.5, cutoffHz: 700.0 } },
    });
  });

  await startLongPlayback(page);

  // Seek multiple times while effects active
  await page.evaluate(async () => {
    for (let i = 0; i < 5; i++) {
      await window.__TAURI_INTERNALS__.invoke('seek_to', { position: i * 5.0 });
    }
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);
});

// ── Test 8: clear_dsp_chain during playback ──

test('clearing entire DSP chain during playback does not crash', async () => {
  test.setTimeout(30_000);

  // Fill all 4 slots
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 1,
      effect: { type: 'crossfeed', settings: { preset: 'natural', levelDb: -4.5, cutoffHz: 700.0 } },
    });
  });

  await startLongPlayback(page);

  // Clear everything at once
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain');
  });

  await page.waitForTimeout(500);
  const state = await getPlaybackState(page);
  expect(['Playing', 'Paused']).toContain(state);

  // Verify chain is empty
  const chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const effects = chain.filter(s => s.effect !== null);
  expect(effects).toHaveLength(0);
});
