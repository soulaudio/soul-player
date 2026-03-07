/**
 * Playback Context History E2E tests — Playwright CDP
 *
 * Tests the "Jump Back Into" feature that records and retrieves playback
 * context history. When an album/artist/genre/playlist is played, its
 * context is recorded via record_playback_context IPC. The Home page
 * uses get_recent_playback_contexts to show the "Jump Back Into" section.
 *
 * IPC commands tested:
 *   record_playback_context({ contextType, contextId, contextName, contextArtworkPath })
 *   get_recent_playback_contexts(limit) → Vec<FrontendPlaybackContext>
 *   get_current_playback_context() → Option<FrontendPlaybackContext>
 *   clear_playback_context_history() → u64 (rows cleared)
 *
 * 8 tests:
 *   1. record_playback_context stores album context
 *   2. get_recent_playback_contexts returns recorded contexts in recency order
 *   3. get_current_playback_context returns the most recent context
 *   4. clear_playback_context_history removes all contexts
 *   5. Recording multiple contexts from different types works
 *   6. Duplicate context updates timestamp instead of creating new entry
 *   7. Home page "Jump Back Into" section shows recorded context
 *   8. Context persists across page navigation
 *
 * Seed data:
 *   Album 2001 — "Playwright Album"
 *   Artist 2001 — "Playwright Artist"
 *   Genre 4001 — "Playwright Genre"
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
  }).catch(() => {});
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Clear context history before each test
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('clear_playback_context_history'); } catch {}
  }).catch(() => {});

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

// Helper: record a context
async function recordContext(p, contextType, contextId, contextName) {
  await p.evaluate(async (args) => {
    await window.__TAURI_INTERNALS__.invoke('record_playback_context', {
      input: {
        contextType: args.contextType,
        contextId: args.contextId,
        contextName: args.contextName,
        contextArtworkPath: null,
      },
    });
  }, { contextType, contextId, contextName });
}

// Helper: get recent contexts
async function getRecentContexts(p, limit = 10) {
  return p.evaluate(async (lim) =>
    window.__TAURI_INTERNALS__.invoke('get_recent_playback_contexts', { limit: lim }),
    limit
  );
}

// ── Test 1: record_playback_context stores album context ──

test('record_playback_context stores an album context', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');

  const contexts = await getRecentContexts(page);
  expect(contexts.length).toBeGreaterThanOrEqual(1);

  const albumCtx = contexts.find(c => c.contextId === '2001' && c.contextType === 'album');
  expect(albumCtx).toBeTruthy();
  expect(albumCtx.contextName).toBe('Playwright Album');
});

// ── Test 2: get_recent_playback_contexts returns in recency order ──

test('get_recent_playback_contexts returns contexts in most-recent-first order', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');
  await page.waitForTimeout(1500); // Need >1s gap so timestamps differ
  await recordContext(page, 'artist', '2001', 'Playwright Artist');
  await page.waitForTimeout(1500);
  await recordContext(page, 'genre', '4001', 'Playwright Genre');

  const contexts = await getRecentContexts(page);
  expect(contexts.length).toBeGreaterThanOrEqual(3);

  // Most recent should be genre (recorded last)
  expect(contexts[0].contextType).toBe('genre');
  expect(contexts[1].contextType).toBe('artist');
  expect(contexts[2].contextType).toBe('album');
});

// ── Test 3: get_current_playback_context returns the most recent ──

test('get_current_playback_context returns the most recently recorded context', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');
  await page.waitForTimeout(1500); // Need >1s gap for different timestamps
  await recordContext(page, 'artist', '2001', 'Playwright Artist');

  const current = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_current_playback_context')
  );

  expect(current).toBeTruthy();
  expect(current.contextType).toBe('artist');
  expect(current.contextName).toBe('Playwright Artist');
});

// ── Test 4: clear_playback_context_history removes all contexts ──

test('clear_playback_context_history clears all recorded contexts', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');
  await recordContext(page, 'artist', '2001', 'Playwright Artist');

  let contexts = await getRecentContexts(page);
  expect(contexts.length).toBeGreaterThanOrEqual(2);

  const cleared = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('clear_playback_context_history')
  );
  expect(cleared).toBeGreaterThanOrEqual(2);

  contexts = await getRecentContexts(page);
  expect(contexts).toHaveLength(0);
});

// ── Test 5: Multiple context types coexist ──

test('recording album, artist, genre, and playlist contexts all coexist', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');
  await recordContext(page, 'artist', '2001', 'Playwright Artist');
  await recordContext(page, 'genre', '4001', 'Playwright Genre');
  await recordContext(page, 'playlist', '3001', 'Favorites');

  const contexts = await getRecentContexts(page);
  const types = contexts.map(c => c.contextType);

  expect(types).toContain('album');
  expect(types).toContain('artist');
  expect(types).toContain('genre');
  expect(types).toContain('playlist');
});

// ── Test 6: Re-recording same context updates timestamp ──

test('re-recording same context updates its timestamp (moves to top)', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');
  await page.waitForTimeout(100);
  await recordContext(page, 'artist', '2001', 'Playwright Artist');
  await page.waitForTimeout(100);

  // Re-record album — should move it to top
  await recordContext(page, 'album', '2001', 'Playwright Album');

  const contexts = await getRecentContexts(page);
  // Album should now be most recent (index 0)
  expect(contexts[0].contextType).toBe('album');
  expect(contexts[0].contextId).toBe('2001');

  // Should not create a duplicate — still exactly 2 distinct entries
  const uniqueKeys = new Set(contexts.map(c => `${c.contextType}:${c.contextId}`));
  expect(uniqueKeys.size).toBe(2);
});

// ── Test 7: Home page shows recorded context in Jump Back Into ──

test('Home page "Jump Back Into" section shows album after recording context', async () => {
  // Record a playback context for album 2001
  await recordContext(page, 'album', '2001', 'Playwright Album');

  // Navigate to Home
  await page.click('[data-testid="nav-home"]', { force: true });
  await page.waitForSelector('[data-testid="home-page"]', { timeout: 15_000 });

  // Wait for sections to render
  await page.waitForSelector('[data-testid^="home-section-"]', { timeout: 15_000 });

  // The album card should appear somewhere on the home page
  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await expect(albumCard).toBeVisible({ timeout: 10_000 });
});

// ── Test 8: Context persists across page navigation ──

test('playback context persists across page navigation', async () => {
  await recordContext(page, 'album', '2001', 'Playwright Album');

  // Navigate away
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-artist-"]', { timeout: 15_000 });
  await page.waitForTimeout(500);

  // Navigate back
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  // Context should still be there
  const contexts = await getRecentContexts(page);
  const albumCtx = contexts.find(c => c.contextId === '2001' && c.contextType === 'album');
  expect(albumCtx).toBeTruthy();
});
