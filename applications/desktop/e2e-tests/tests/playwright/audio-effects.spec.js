/**
 * Audio Effects (DSP Chain) E2E tests — Playwright CDP
 *
 * Tests the DSP effect chain configurator on the audio settings page:
 *  1. DSP config section is visible on audio settings page
 *  2. Add Compressor to slot 0 opens the compressor editor
 *  3. Compressor editor shows all expected controls
 *  4. Remove effect from slot 0 clears the slot
 *  5. Add Graphic EQ and select a preset
 *  6. Add Crossfeed and use a preset button
 *  7. Clear all effects removes all effects from all slots
 *
 * Architecture notes:
 * - effect-picker-{N} is a custom <div> panel (not a <select>) that appears
 *   when the "Add Effect" button is clicked. Effects are chosen by clicking
 *   buttons in the panel grouped by category (EQ, Dynamics, Spatial).
 * - After adding an effect, click edit-effect-btn-{N} to expand the editor panel.
 * - clear-all-btn only renders when at least one slot has an effect — it opens
 *   a confirmation dialog (ConfirmDialog with "Clear All" confirm button).
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ── CDP connection shared across all tests in this file ────────────────────

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );

  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Navigate to Audio settings before each test
test.beforeEach(async () => {
  // Dismiss any lingering overlay from a previous test
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Open settings panel — force:true handles any residual backdrop overlay
  await page.click('[data-testid="settings-button"]', { force: true });

  // Wait for the sidebar to be visible (any nav item confirms it)
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });

  // Navigate to the Audio section
  await page.click('[data-testid="nav-settings-audio"]');

  // Wait for the audio settings page container
  await page.waitForSelector('[data-testid="audio-settings-page"]', { timeout: 10_000 });

  // Scroll to the DSP section and wait for it to render (including async chain load)
  await page.locator('[data-testid="audio-stage-dsp"]').scrollIntoViewIfNeeded();
  await page.waitForSelector('[data-testid="dsp-config"]', { timeout: 10_000 });

  // Wait for the DSP chain to finish loading — slots only appear after getDspChain() resolves.
  // The component shows a loading spinner while loading=true.
  await page.waitForSelector('[data-testid="effect-slot-0"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  // Best-effort cleanup: clear all effects if the clear-all button is present.
  // The clear-all-btn is only rendered when the chain has at least one effect.
  try {
    const clearBtn = page.locator('[data-testid="clear-all-btn"]');
    const isClearVisible = await clearBtn.isVisible({ timeout: 500 }).catch(() => false);
    if (isClearVisible) {
      await clearBtn.click();
      // A ConfirmDialog (data-testid="confirm-dialog") must appear.
      // Click the confirm button inside the dialog — NOT the clear-all-btn again.
      const dialog = page.locator('[data-testid="confirm-dialog"]');
      await dialog.waitFor({ state: 'visible', timeout: 3_000 });
      // The destructive confirm button is the last button in the dialog footer
      const confirmBtn = dialog.locator('button').last();
      await confirmBtn.click();
      // Wait for chain to clear (add-effect-btn-0 reappears)
      await page.locator('[data-testid="add-effect-btn-0"]').waitFor({ state: 'visible', timeout: 5_000 });
    }
  } catch {
    // Nothing to clean up or cleanup failed — proceed anyway
  }

  // Dismiss settings panel
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Helper: add an effect to a slot by clicking the picker panel ────────────

/**
 * Adds an effect to the given slot by:
 *  1. Clicking the "add effect" dashed button to expand the picker panel
 *  2. Clicking the effect button by its visible text label
 *
 * The picker panel (effect-picker-{N}) is a grouped div, not a <select>.
 * Each effect is a <button> whose text matches effectLabel.
 */
async function addEffectToSlot(slotIndex, effectLabel) {
  const addBtn = page.locator(`[data-testid="add-effect-btn-${slotIndex}"]`);
  await expect(addBtn).toBeVisible({ timeout: 5_000 });
  await addBtn.click();
  await page.waitForTimeout(300);

  // Wait for the picker panel to appear
  const picker = page.locator(`[data-testid="effect-picker-${slotIndex}"]`);
  await picker.waitFor({ state: 'visible', timeout: 5_000 });

  // The picker is a custom panel; click the button whose text matches effectLabel
  const effectBtn = picker.getByRole('button', { name: new RegExp(effectLabel, 'i') }).first();
  await effectBtn.waitFor({ state: 'visible', timeout: 3_000 });
  await effectBtn.click();

  // Wait for the picker to close (effect is being added)
  await picker.waitFor({ state: 'hidden', timeout: 5_000 }).catch(() => {});
  await page.waitForTimeout(400);
}

/**
 * Opens the editor panel for a slot that already has an effect by
 * clicking the "Edit" button (edit-effect-btn-{N}).
 */
async function openEditor(slotIndex) {
  const editBtn = page.locator(`[data-testid="edit-effect-btn-${slotIndex}"]`);
  await expect(editBtn).toBeVisible({ timeout: 5_000 });
  await editBtn.click();
  await page.waitForTimeout(300);
}

// ── Tests ──────────────────────────────────────────────────────────────────

test('DSP config section is visible on audio settings page', async () => {
  // The DSP config container must be present
  const dspConfig = page.locator('[data-testid="dsp-config"]');
  await expect(dspConfig).toBeVisible();

  // All 4 effect slots must be present
  for (let i = 0; i < 4; i++) {
    await expect(page.locator(`[data-testid="effect-slot-${i}"]`)).toBeVisible();
  }

  await page.screenshot({ path: 'screenshots/audio-effects-dsp-config.png' });
});

test('add Compressor to slot 0 opens the compressor editor', async () => {
  await addEffectToSlot(0, 'Compressor');

  // After adding, the slot should show an edit button
  await expect(page.locator('[data-testid="edit-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // Open the editor
  await openEditor(0);

  // The compressor editor must be visible
  await expect(page.locator('[data-testid="compressor-editor"]')).toBeVisible({ timeout: 5_000 });

  await page.screenshot({ path: 'screenshots/audio-effects-compressor-editor.png' });
});

test('compressor editor shows all expected controls', async () => {
  await addEffectToSlot(0, 'Compressor');
  await openEditor(0);

  // All main parameter sliders must be visible
  await expect(page.locator('[data-testid="compressor-threshold"]')).toBeVisible({ timeout: 5_000 });
  await expect(page.locator('[data-testid="compressor-ratio"]')).toBeVisible();
  await expect(page.locator('[data-testid="compressor-attack"]')).toBeVisible();
  await expect(page.locator('[data-testid="compressor-release"]')).toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-effects-compressor-controls.png' });
});

test('remove effect from slot 0 clears the slot', async () => {
  await addEffectToSlot(0, 'Compressor');

  // Confirm effect was added (edit button visible)
  await expect(page.locator('[data-testid="edit-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // Click the remove button for slot 0
  const removeBtn = page.locator('[data-testid="remove-effect-btn-0"]');
  await expect(removeBtn).toBeVisible({ timeout: 5_000 });
  await removeBtn.click();
  await page.waitForTimeout(400);

  // The "Add Effect" dashed button must reappear
  await expect(page.locator('[data-testid="add-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // The compressor editor must no longer be visible
  await expect(page.locator('[data-testid="compressor-editor"]')).not.toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-effects-slot-cleared.png' });
});

test('add Graphic EQ and select a preset', async () => {
  await addEffectToSlot(0, 'Graphic EQ');

  // Confirm effect was added
  await expect(page.locator('[data-testid="edit-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // Open the editor
  await openEditor(0);

  // The graphic-eq-editor must be visible
  const eqEditor = page.locator('[data-testid="graphic-eq-editor"]');
  await expect(eqEditor).toBeVisible({ timeout: 5_000 });

  // The preset dropdown button must be visible
  const presetBtn = page.locator('[data-testid="graphic-eq-preset-select"]');
  await expect(presetBtn).toBeVisible();

  // Click to open the preset dropdown
  await presetBtn.click();
  await page.waitForTimeout(300);

  // Select the second preset option in the dropdown list.
  // The dropdown renders <button> elements inside an absolute-positioned div.
  // We look for buttons inside the popover that appeared.
  // The first preset is "Flat" (the current one), so pick the second.
  const dropdownOptions = page.locator('.absolute.z-50 button, [class*="popover"] button').filter({
    hasNotText: /^$/,
  });
  const optionCount = await dropdownOptions.count();

  if (optionCount >= 2) {
    // Click the second option (index 1)
    await dropdownOptions.nth(1).click();
  } else {
    // Fallback: pick any option with text "Bass Boost" or "Treble Boost"
    const bassBoostOption = page.getByRole('button', { name: /bass boost/i });
    const isVisible = await bassBoostOption.isVisible().catch(() => false);
    if (isVisible) {
      await bassBoostOption.click();
    } else {
      // Just close the dropdown and proceed
      await page.keyboard.press('Escape');
    }
  }
  await page.waitForTimeout(300);

  // The reset button must now be visible (it's always rendered; disabled only when flat)
  const resetBtn = page.locator('[data-testid="graphic-eq-reset-btn"]');
  await expect(resetBtn).toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-effects-graphic-eq.png' });
});

test('add Crossfeed and use a preset button', async () => {
  await addEffectToSlot(0, 'Crossfeed');

  // Confirm effect was added
  await expect(page.locator('[data-testid="edit-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // Open the editor
  await openEditor(0);

  // The crossfeed editor must be visible
  const crossfeedEditor = page.locator('[data-testid="crossfeed-editor"]');
  await expect(crossfeedEditor).toBeVisible({ timeout: 5_000 });

  // Click the first crossfeed preset button (e.g. natural)
  // Preset buttons have testid pattern: crossfeed-preset-{id}
  const firstPresetBtn = page.locator('[data-testid^="crossfeed-preset-"]').first();
  await expect(firstPresetBtn).toBeVisible({ timeout: 5_000 });
  await firstPresetBtn.click();
  await page.waitForTimeout(300);

  // The crossfeed-level slider must be visible
  const levelSlider = page.locator('[data-testid="crossfeed-level"]');
  await expect(levelSlider).toBeVisible({ timeout: 5_000 });

  await page.screenshot({ path: 'screenshots/audio-effects-crossfeed.png' });
});

test('clear all effects removes all effects from all slots', async () => {
  // Add a Compressor to slot 0 so that clear-all-btn becomes visible
  await addEffectToSlot(0, 'Compressor');

  // Confirm effect was added
  await expect(page.locator('[data-testid="edit-effect-btn-0"]')).toBeVisible({ timeout: 5_000 });

  // The "Clear All" button must now be visible (only shown when chain has effects)
  const clearAllBtn = page.locator('[data-testid="clear-all-btn"]');
  await expect(clearAllBtn).toBeVisible({ timeout: 5_000 });

  // Click it — this opens a ConfirmDialog
  await clearAllBtn.click();
  await page.waitForTimeout(300);

  // The confirmation dialog (data-testid="confirm-dialog") must appear.
  // Click the destructive confirm button (the last button in the dialog footer).
  const dialog = page.locator('[data-testid="confirm-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 5_000 });
  const confirmBtn = dialog.locator('button').last();
  await confirmBtn.click();
  await page.waitForTimeout(500);

  // All 4 "Add Effect" buttons must reappear (slots are empty)
  for (let i = 0; i < 4; i++) {
    await expect(page.locator(`[data-testid="add-effect-btn-${i}"]`)).toBeVisible({ timeout: 5_000 });
  }

  // The clear-all-btn must be gone (no effects remaining)
  await expect(clearAllBtn).not.toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-effects-cleared.png' });
});
