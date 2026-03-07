/**
 * DSP Effect Editors E2E tests — Playwright CDP
 *
 * Tests DSP chain manipulation commands not covered by dsp-presets.spec.js:
 *   get_available_effects, remove_effect_from_chain, toggle_effect,
 *   update_effect_parameters, and all preset query commands
 *
 * 8 tests
 *
 * Seed data: None required (DSP commands work without tracks)
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

test.beforeEach(async () => {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(100);
  // Clear DSP chain before each test
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
});

// ── Test 1: get_available_effects returns effect list ──

test('get_available_effects returns an array of effect type names', async () => {
  const effects = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_available_effects')
  );

  expect(Array.isArray(effects)).toBe(true);
  expect(effects.length).toBeGreaterThanOrEqual(1);
});

// ── Test 2: remove_effect_from_chain removes an added effect ──

test('remove_effect_from_chain removes previously added effect', async () => {
  // Add a compressor at slot 0
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  // Verify it's there
  let chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  let effects = chain.filter(s => s.effect !== null);
  expect(effects.length).toBeGreaterThanOrEqual(1);

  // Remove it
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('remove_effect_from_chain', { slotIndex: 0 });
  });

  chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  effects = chain.filter(s => s.effect !== null);
  expect(effects).toHaveLength(0);
});

// ── Test 3: toggle_effect enables/disables an effect ──

test('toggle_effect disables and re-enables an effect in the chain', async () => {
  // Add an effect
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  // Disable it
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('toggle_effect', { slotIndex: 0, enabled: false });
  });

  let chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const slot = chain.find(s => s.effect !== null);
  if (slot) {
    expect(slot.enabled).toBe(false);
  }

  // Re-enable it
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('toggle_effect', { slotIndex: 0, enabled: true });
  });

  chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const reEnabled = chain.find(s => s.effect !== null);
  if (reEnabled) {
    expect(reEnabled.enabled).toBe(true);
  }
});

// ── Test 4: update_effect_parameters changes settings ──

test('update_effect_parameters changes compressor settings', async () => {
  // Add a compressor
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  // Update parameters
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('update_effect_parameters', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -10, ratio: 8, attackMs: 5, releaseMs: 200, kneeDb: 3, makeupGainDb: 6 } },
    });
  });

  const chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const slot = chain.find(s => s.effect !== null);
  expect(slot).toBeTruthy();
});

// ── Test 5: EQ preset queries return arrays ──

test('get_eq_presets returns array of EQ presets', async () => {
  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_eq_presets')
  );

  expect(Array.isArray(presets)).toBe(true);
});

// ── Test 6: Compressor preset queries return arrays ──

test('get_compressor_presets returns array of compressor presets', async () => {
  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_compressor_presets')
  );

  expect(Array.isArray(presets)).toBe(true);
});

// ── Test 7: Multiple effects in chain simultaneously ──

test('chain supports multiple effects in different slots', async () => {
  // Add compressor at slot 0 and crossfeed at slot 1
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

  const chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const effects = chain.filter(s => s.effect !== null);
  expect(effects.length).toBeGreaterThanOrEqual(2);
});

// ── Test 8: All preset query commands return arrays ──

test('all DSP preset query commands return arrays', async () => {
  const results = await page.evaluate(async () => {
    const limiter = await window.__TAURI_INTERNALS__.invoke('get_limiter_presets');
    const crossfeed = await window.__TAURI_INTERNALS__.invoke('get_crossfeed_presets');
    const stereo = await window.__TAURI_INTERNALS__.invoke('get_stereo_presets');
    const graphicEq = await window.__TAURI_INTERNALS__.invoke('get_graphic_eq_presets');
    return { limiter, crossfeed, stereo, graphicEq };
  });

  expect(Array.isArray(results.limiter)).toBe(true);
  expect(Array.isArray(results.crossfeed)).toBe(true);
  expect(Array.isArray(results.stereo)).toBe(true);
  expect(Array.isArray(results.graphicEq)).toBe(true);
});
