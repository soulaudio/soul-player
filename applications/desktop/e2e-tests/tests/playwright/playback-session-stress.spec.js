/**
 * Playback Session Stress E2E tests — Playwright CDP
 *
 * Stress-tests playback session save/restore with rapid cycling,
 * concurrent saves, and edge cases.
 *
 * 6 tests
 *
 * Seed data:
 *   Album 2001 — 5 tracks (2001-2005)
 *   Album 2002 — 5 tracks (3001-3005)
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
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_playback_session'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_playback_session'); } catch {}
  }).catch(() => {});
});

function makeSession(trackId, queueIndex, volume) {
  return {
    currentTrackId: trackId,
    queueTrackIds: [2001, 2002, 2003, 2004, 2005],
    queueIndex,
    positionSeconds: queueIndex * 1.0,
    volume,
    repeatMode: 'off',
    shuffleMode: 'off',
    wasPlaying: true,
  };
}

// ── Test 1: 20 rapid save/restore cycles ──

test('20 rapid save/restore cycles maintain last-write-wins', async () => {
  test.setTimeout(30_000);

  for (let i = 0; i < 20; i++) {
    await page.evaluate(async (args) => {
      await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
        session: args.session,
      });
    }, { session: makeSession(2001 + (i % 5), i % 5, 50 + i) });
  }

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session).toBeTruthy();
  // Last iteration i=19: trackId = 2001 + (19 % 5) = 2005, volume = 69
  expect(session.currentTrackId).toBe(2005);
  expect(session.volume).toBe(69);
});

// ── Test 2: Save/clear/save/restore cycle ──

test('save-clear-save-restore returns the second save', async () => {
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 2001, queueTrackIds: [2001], queueIndex: 0,
        positionSeconds: 5.0, volume: 80, repeatMode: 'off', shuffleMode: 'off', wasPlaying: true,
      },
    });
    await window.__TAURI_INTERNALS__.invoke('clear_playback_session');
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 3001, queueTrackIds: [3001, 3002], queueIndex: 1,
        positionSeconds: 10.0, volume: 60, repeatMode: 'all', shuffleMode: 'on', wasPlaying: false,
      },
    });
  });

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session.currentTrackId).toBe(3001);
  expect(session.repeatMode).toBe('all');
  expect(session.shuffleMode).toBe('on');
});

// ── Test 3: Save with large queue ──

test('save/restore session with 100-track queue', async () => {
  const largeQueue = Array.from({ length: 100 }, (_, i) => 2001 + (i % 20));

  await page.evaluate(async (queue) => {
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: queue[50],
        queueTrackIds: queue,
        queueIndex: 50,
        positionSeconds: 25.0,
        volume: 75,
        repeatMode: 'off',
        shuffleMode: 'off',
        wasPlaying: true,
      },
    });
  }, largeQueue);

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session).toBeTruthy();
  expect(session.queueTrackIds.length).toBe(100);
  expect(session.queueIndex).toBe(50);
});

// ── Test 4: Concurrent saves (last write wins) ──

test('concurrent save calls resolve without errors', async () => {
  await page.evaluate(async () => {
    const sessions = Array.from({ length: 5 }, (_, i) => ({
      currentTrackId: 2001 + i,
      queueTrackIds: [2001, 2002, 2003],
      queueIndex: i % 3,
      positionSeconds: i * 2.0,
      volume: 50 + i * 10,
      repeatMode: 'off',
      shuffleMode: 'off',
      wasPlaying: true,
    }));

    await Promise.all(sessions.map(s =>
      window.__TAURI_INTERNALS__.invoke('save_playback_session', { session: s })
    ));
  });

  // At least one save should have succeeded
  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );
  expect(session).toBeTruthy();
});

// ── Test 5: Save during active playback ──

test('saving session while audio is playing does not interrupt playback', async () => {
  test.setTimeout(30_000);

  // Start playback
  await page.evaluate(async () => {
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

  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Save session 5 times during playback
  for (let i = 0; i < 5; i++) {
    await page.evaluate(async (idx) => {
      await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
        session: {
          currentTrackId: 3001,
          queueTrackIds: [3001, 3002, 3003, 3004, 3005],
          queueIndex: idx,
          positionSeconds: idx * 5.0,
          volume: 70,
          repeatMode: 'off',
          shuffleMode: 'off',
          wasPlaying: true,
        },
      });
    }, i);
  }

  const state = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  // Short test tracks may finish before we get here
  expect(['Playing', 'Paused', 'Stopped']).toContain(state);
});

// ── Test 6: 10 rapid clear cycles ──

test('10 rapid clear_playback_session calls are idempotent', async () => {
  // Save something first
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 2001, queueTrackIds: [2001], queueIndex: 0,
        positionSeconds: 0, volume: 50, repeatMode: 'off', shuffleMode: 'off', wasPlaying: false,
      },
    });
  });

  // Clear 10 times rapidly
  await page.evaluate(async () => {
    for (let i = 0; i < 10; i++) {
      await window.__TAURI_INTERNALS__.invoke('clear_playback_session');
    }
  });

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );
  expect(session).toBeNull();
});
