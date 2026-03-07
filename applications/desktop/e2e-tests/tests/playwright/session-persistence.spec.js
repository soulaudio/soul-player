/**
 * Session Persistence E2E tests — Playwright CDP
 *
 * Tests playback session save/restore lifecycle:
 *   save_playback_session, restore_playback_session,
 *   clear_playback_session
 *
 * 6 tests
 *
 * Seed data:
 *   Album 2001 "Playwright Album" — 5 tracks (2001-2005)
 *   Album 2002 "Long Album" — 5 tracks (3001-3005)
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
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('clear_playback_session'); } catch {}
  }).catch(() => {});
});

// ── Test 1: save_playback_session accepts a session object ──

test('save_playback_session stores session data without error', async () => {
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
        session: {
          currentTrackId: 2001,
          queueTrackIds: [2001, 2002, 2003, 2004, 2005],
          queueIndex: 0,
          positionSeconds: 1.5,
          volume: 75.0,
          repeatMode: 'off',
          shuffleMode: 'off',
          contextType: 'album',
          contextId: '2001',
          wasPlaying: true,
        },
      });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  expect(error).toBeNull();
});

// ── Test 2: restore_playback_session returns saved data ──

test('restore_playback_session returns previously saved session', async () => {
  // Save a session
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 3001,
        queueTrackIds: [3001, 3002, 3003],
        queueIndex: 1,
        positionSeconds: 15.0,
        volume: 50.0,
        repeatMode: 'all',
        shuffleMode: 'off',
        wasPlaying: false,
      },
    });
  });

  // Restore it
  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session).toBeTruthy();
  expect(session.currentTrackId).toBe(3001);
  expect(session.queueIndex).toBe(1);
  expect(session.volume).toBe(50.0);
  expect(session.repeatMode).toBe('all');
});

// ── Test 3: clear_playback_session removes saved data ──

test('clear_playback_session removes the saved session', async () => {
  // Save then clear
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
      session: {
        currentTrackId: 2001,
        queueTrackIds: [2001],
        queueIndex: 0,
        positionSeconds: 0,
        volume: 80.0,
        repeatMode: 'off',
        shuffleMode: 'off',
        wasPlaying: false,
      },
    });
    await window.__TAURI_INTERNALS__.invoke('clear_playback_session');
  });

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session).toBeNull();
});

// ── Test 4: Session survives multiple save/restore cycles ──

test('multiple save/restore cycles maintain data integrity', async () => {
  for (let i = 0; i < 5; i++) {
    await page.evaluate(async (idx) => {
      await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
        session: {
          currentTrackId: 2001 + idx,
          queueTrackIds: [2001, 2002, 2003, 2004, 2005],
          queueIndex: idx,
          positionSeconds: idx * 1.0,
          volume: 60.0,
          repeatMode: 'off',
          shuffleMode: 'off',
          wasPlaying: true,
        },
      });
    }, i);
  }

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  // Last save should win
  expect(session).toBeTruthy();
  expect(session.currentTrackId).toBe(2005);
  expect(session.queueIndex).toBe(4);
});

// ── Test 5: restore_playback_session returns null when no session saved ──

test('restore_playback_session returns null when no session exists', async () => {
  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );

  expect(session).toBeNull();
});

// ── Test 6: Save session with all optional fields ──

test('save_playback_session handles optional context fields as null', async () => {
  // currentTrackId must be non-null for restore to find a session
  const error = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('save_playback_session', {
        session: {
          currentTrackId: 2001,
          queueTrackIds: [2001],
          queueIndex: 0,
          positionSeconds: 0.0,
          volume: 100.0,
          repeatMode: 'one',
          shuffleMode: 'on',
          contextType: null,
          contextId: null,
          wasPlaying: false,
        },
      });
      return null;
    } catch (e) {
      return String(e);
    }
  });

  expect(error).toBeNull();

  const session = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('restore_playback_session')
  );
  expect(session).toBeTruthy();
  expect(session.shuffleMode).toBe('on');
  expect(session.repeatMode).toBe('one');
  expect(session.contextType).toBeNull();
  expect(session.contextId).toBeNull();
});
