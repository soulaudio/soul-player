/**
 * Diagnostic: verify Ctrl+Space keydown event reaches JavaScript in the isolated world
 * Run this to understand why keyboard-and-volume tests 1 and 3 fail.
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
  if (!page) throw new Error('Main page not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  // Stop any playback started by DIAG tests so subsequent spec files
  // (e.g. genre-page) don't start with a contaminated playback state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ---- Helper: start playback same as keyboard-and-volume.spec.js ----
async function startPlayback(pg) {
  const card = pg.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });
  await card.hover();
  await card.locator('[data-testid="media-card-play-button"]').click();
  await pg.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await pg.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  await pg.evaluate(() => { document.activeElement?.blur(); document.body.focus(); });
  await pg.waitForTimeout(100);
}

test('DIAG-1: Ctrl+Space keydown event capture in isolated world', async () => {
  await startPlayback(page);

  // Install a keydown listener in the isolated world
  await page.evaluate(() => {
    window.__diagKeys = [];
    window.__diagHandler = (e) => {
      window.__diagKeys.push({
        key: e.key,
        code: e.code,
        ctrlKey: e.ctrlKey,
        repeat: e.repeat,
        defaultPrevented: e.defaultPrevented,
        activeElement: document.activeElement ? document.activeElement.tagName + '#' + (document.activeElement.id || '?') + '[' + document.activeElement.getAttribute('data-testid') + ']' : 'null',
      });
    };
    window.addEventListener('keydown', window.__diagHandler, true); // capturing phase
  });

  // Also capture what element currently has focus
  const focusInfo = await page.evaluate(() => {
    const el = document.activeElement;
    return {
      tag: el ? el.tagName : 'null',
      id: el ? el.id : null,
      testid: el ? el.getAttribute('data-testid') : null,
      role: el ? el.getAttribute('role') : null,
    };
  });
  console.log('Active element before Ctrl+Space:', JSON.stringify(focusInfo));

  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);

  const captured = await page.evaluate(() => {
    window.removeEventListener('keydown', window.__diagHandler, true);
    return window.__diagKeys;
  });
  console.log('Captured keydown events:', JSON.stringify(captured, null, 2));

  const backendState = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  console.log('Backend state after Ctrl+Space:', backendState);

  // The keydown event MUST reach the isolated world listener
  expect(captured.length).toBeGreaterThan(0);

  const spaceEvent = captured.find(k => (k.key === ' ' || k.code === 'Space') && k.ctrlKey);
  console.log('Space event found:', JSON.stringify(spaceEvent));
  expect(spaceEvent).toBeTruthy();
});

test('DIAG-2: Compare Ctrl+Space vs Ctrl+ArrowRight focus and events', async () => {
  await startPlayback(page);

  // Capture Ctrl+ArrowRight (known working)
  await page.evaluate(() => {
    window.__diagKeys2 = [];
    window.__diagHandler2 = (e) => {
      window.__diagKeys2.push({ key: e.key, code: e.code, ctrlKey: e.ctrlKey });
    };
    window.addEventListener('keydown', window.__diagHandler2, true);
  });

  await page.keyboard.press('Control+ArrowRight');
  await page.waitForTimeout(300);

  await page.evaluate(() => {
    window.removeEventListener('keydown', window.__diagHandler2, true);
  });
  const keysRight = await page.evaluate(() => window.__diagKeys2);
  console.log('Ctrl+ArrowRight events:', JSON.stringify(keysRight));

  // Wait for track to change, then restart for next test
  await page.waitForTimeout(2000);

  // Navigate away and restart for Ctrl+Space
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid^="media-card-album-"]', { timeout: 15_000 });
  await startPlayback(page);

  // Capture Ctrl+Space (failing)
  await page.evaluate(() => {
    window.__diagKeys3 = [];
    window.__diagHandler3 = (e) => {
      window.__diagKeys3.push({ key: e.key, code: e.code, ctrlKey: e.ctrlKey });
    };
    window.addEventListener('keydown', window.__diagHandler3, true);
  });

  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(500);

  await page.evaluate(() => {
    window.removeEventListener('keydown', window.__diagHandler3, true);
  });
  const keysSpace = await page.evaluate(() => window.__diagKeys3);
  console.log('Ctrl+Space events:', JSON.stringify(keysSpace));

  // Both should have captured the main key
  const hasArrowRight = keysRight.some(k => k.key === 'ArrowRight' && k.ctrlKey);
  const hasSpace = keysSpace.some(k => (k.key === ' ' || k.code === 'Space') && k.ctrlKey);

  console.log('hasArrowRight:', hasArrowRight, 'hasSpace:', hasSpace);
});

test('DIAG-3: Active element after startPlayback - where does focus go?', async () => {
  const card = page.locator('[data-testid="media-card-album-2001"]');
  await card.waitFor({ state: 'visible' });
  await card.hover();
  await card.locator('[data-testid="media-card-play-button"]').click();
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await page.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );

  // Check focus BEFORE blur
  const focusBefore = await page.evaluate(() => {
    const el = document.activeElement;
    return {
      tag: el ? el.tagName : 'null',
      id: el ? el.id : null,
      testid: el ? el.getAttribute('data-testid') : null,
    };
  });
  console.log('Active element BEFORE blur:', JSON.stringify(focusBefore));

  // Blur and wait 100ms (as per startPlayback)
  await page.evaluate(() => { document.activeElement?.blur(); document.body.focus(); });
  await page.waitForTimeout(100);

  // Check focus AFTER blur
  const focusAfter = await page.evaluate(() => {
    const el = document.activeElement;
    return {
      tag: el ? el.tagName : 'null',
      id: el ? el.id : null,
      testid: el ? el.getAttribute('data-testid') : null,
    };
  });
  console.log('Active element AFTER blur + 100ms:', JSON.stringify(focusAfter));

  // Wait another 500ms to see if focus moves
  await page.waitForTimeout(500);
  const focusLater = await page.evaluate(() => {
    const el = document.activeElement;
    return {
      tag: el ? el.tagName : 'null',
      id: el ? el.id : null,
      testid: el ? el.getAttribute('data-testid') : null,
    };
  });
  console.log('Active element after 600ms total:', JSON.stringify(focusLater));
});
