/**
 * DSP Chain Presets E2E tests — Playwright CDP
 *
 * Tests save/load/delete of DSP chain presets via IPC.
 *
 * IPC commands tested:
 *   get_dsp_chain_presets() → Vec<DspPreset>
 *   save_dsp_chain_preset(name, description, effect_chain) → i64 (preset ID)
 *   load_dsp_chain_preset(preset_id) → ()
 *   delete_dsp_chain_preset(preset_id) → ()
 *   get_dsp_chain() → Vec<EffectSlot>
 *   add_effect_to_chain(slot_index, effect_type) → ()
 *   clear_dsp_chain() → ()
 *
 * 7 tests:
 *   1. get_dsp_chain_presets returns an array (may include built-ins)
 *   2. save_dsp_chain_preset creates a new preset
 *   3. Saved preset appears in get_dsp_chain_presets list
 *   4. load_dsp_chain_preset applies the preset to the DSP chain
 *   5. delete_dsp_chain_preset removes user preset
 *   6. Saving preset with same name overwrites (upsert)
 *   7. Cannot delete built-in presets (returns error)
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

  // Warm up the LazyPlaybackManager so DSP commands work
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

  // Clean up: delete any user presets named "Test Preset" or "Test Preset 2"
  await page.evaluate(async () => {
    try {
      const presets = await window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets');
      for (const p of presets) {
        if (!p.isBuiltin && (p.name === 'Test Preset' || p.name === 'Test Preset 2')) {
          await window.__TAURI_INTERNALS__.invoke('delete_dsp_chain_preset', { presetId: p.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});

  // Clear the DSP chain
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
});

test.afterEach(async () => {
  // Clean up test presets
  await page.evaluate(async () => {
    try {
      const presets = await window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets');
      for (const p of presets) {
        if (!p.isBuiltin && (p.name === 'Test Preset' || p.name === 'Test Preset 2')) {
          await window.__TAURI_INTERNALS__.invoke('delete_dsp_chain_preset', { presetId: p.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});

  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain'); } catch {}
  }).catch(() => {});
});

// ── Test 1: get_dsp_chain_presets returns an array ──

test('get_dsp_chain_presets returns an array', async () => {
  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );
  expect(Array.isArray(presets)).toBe(true);
});

// ── Test 2: save_dsp_chain_preset creates a new preset ──

test('save_dsp_chain_preset creates a new preset and returns its ID', async () => {
  // Add a compressor to the chain first
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  // Get the current chain
  const chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );

  // Build the effect_chain array from chain slots that have effects
  const effectChain = chain.filter(s => s.effect !== null).map(s => s.effect);

  const presetId = await page.evaluate(async (args) =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: args.name,
      description: args.description,
      effectChain: args.effectChain,
    }),
    { name: 'Test Preset', description: 'A test preset', effectChain }
  );

  expect(typeof presetId).toBe('number');
  expect(presetId).toBeGreaterThan(0);
});

// ── Test 3: Saved preset appears in list ──

test('saved preset appears in get_dsp_chain_presets list', async () => {
  // Save a preset with empty chain
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: 'Test Preset',
      description: 'Test description',
      effectChain: [],
    });
  });

  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );

  const testPreset = presets.find(p => p.name === 'Test Preset');
  expect(testPreset).toBeTruthy();
  expect(testPreset.isBuiltin).toBe(false);
});

// ── Test 4: load_dsp_chain_preset applies preset to DSP chain ──

test('load_dsp_chain_preset applies the preset to the active DSP chain', async () => {
  // Add compressor, save as preset
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_effect_to_chain', {
      slotIndex: 0,
      effect: { type: 'compressor', settings: { thresholdDb: -20, ratio: 4, attackMs: 10, releaseMs: 100, kneeDb: 0, makeupGainDb: 0 } },
    });
  });

  const chain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const effectChain = chain.filter(s => s.effect !== null).map(s => s.effect);

  const presetId = await page.evaluate(async (args) =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: args.name, description: null, effectChain: args.effectChain,
    }),
    { name: 'Test Preset', effectChain }
  );

  // Clear the chain
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('clear_dsp_chain');
  });

  // Verify chain is empty
  const emptyChain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const emptyEffects = emptyChain.filter(s => s.effect !== null);
  expect(emptyEffects).toHaveLength(0);

  // Load the preset
  await page.evaluate(async (id) =>
    window.__TAURI_INTERNALS__.invoke('load_dsp_chain_preset', { presetId: id }),
    presetId
  );

  // Chain should now have the compressor
  const loadedChain = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain')
  );
  const loadedEffects = loadedChain.filter(s => s.effect !== null);
  expect(loadedEffects.length).toBeGreaterThanOrEqual(1);
});

// ── Test 5: delete_dsp_chain_preset removes user preset ──

test('delete_dsp_chain_preset removes a user-created preset', async () => {
  const presetId = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: 'Test Preset', description: null, effectChain: [],
    })
  );

  // Verify it exists
  let presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );
  expect(presets.some(p => p.name === 'Test Preset')).toBe(true);

  // Delete it
  await page.evaluate(async (id) =>
    window.__TAURI_INTERNALS__.invoke('delete_dsp_chain_preset', { presetId: id }),
    presetId
  );

  // Verify it's gone
  presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );
  expect(presets.some(p => p.name === 'Test Preset')).toBe(false);
});

// ── Test 6: Saving with same name upserts ──

test('saving preset with same name overwrites existing preset', async () => {
  // Save first version
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: 'Test Preset', description: 'Version 1', effectChain: [],
    })
  );

  // Save second version with same name
  await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('save_dsp_chain_preset', {
      name: 'Test Preset', description: 'Version 2', effectChain: [],
    })
  );

  // Should still be only one preset with that name
  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );
  const testPresets = presets.filter(p => p.name === 'Test Preset');
  expect(testPresets).toHaveLength(1);
});

// ── Test 7: Built-in presets cannot be deleted ──

test('delete_dsp_chain_preset rejects deletion of built-in presets', async () => {
  const presets = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_dsp_chain_presets')
  );

  const builtIn = presets.find(p => p.isBuiltin);
  if (!builtIn) {
    // No built-in presets — skip test
    test.skip();
    return;
  }

  // Attempt to delete built-in preset should throw
  const error = await page.evaluate(async (id) => {
    try {
      await window.__TAURI_INTERNALS__.invoke('delete_dsp_chain_preset', { presetId: id });
      return null;
    } catch (e) {
      return String(e);
    }
  }, builtIn.id);

  expect(error).toBeTruthy();
});
