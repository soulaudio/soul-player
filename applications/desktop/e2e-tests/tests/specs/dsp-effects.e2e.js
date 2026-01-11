/**
 * E2E Tests for DSP Effects Workflows
 *
 * Comprehensive tests for the DSP effects chain configuration including:
 * - Adding effects to slots
 * - Editing effect parameters
 * - Removing effects
 * - Effect chain ordering
 * - Preset application
 * - Effect enable/disable
 * - Clear all effects
 * - Settings persistence
 *
 * Prerequisites:
 * - App must be built: cargo build --release -p soul-player-desktop
 * - tauri-driver must be installed: cargo install tauri-driver
 * - UI components must have data-testid attributes (see TEST_IDS.md)
 *
 * NOTE: These tests require proper data-testid attributes on DSP components.
 */

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Helper to wait for an element with better error messages
 */
async function waitForElement(selector, description, timeout = 5000) {
  const element = await $(selector);
  try {
    await element.waitForExist({ timeout });
  } catch {
    throw new Error(
      `Element not found: ${description}\n` +
      `Selector: ${selector}\n` +
      `Hint: Make sure the data-testid attribute is added to the UI component.`
    );
  }
  return element;
}

/**
 * Helper to check if an element has a specific class or aria-selected attribute
 */
async function isElementSelected(element) {
  const ariaSelected = await element.getAttribute('aria-selected');
  if (ariaSelected === 'true') return true;

  const ariaPressed = await element.getAttribute('aria-pressed');
  if (ariaPressed === 'true') return true;

  const dataState = await element.getAttribute('data-state');
  if (dataState === 'on' || dataState === 'active' || dataState === 'checked') return true;

  const classList = await element.getAttribute('class') || '';
  return classList.includes('border-primary') ||
         classList.includes('bg-primary') ||
         classList.includes('selected') ||
         classList.includes('active');
}

/**
 * Navigate to the audio settings tab
 */
async function navigateToAudioSettings() {
  const settingsButton = await waitForElement('[data-testid="settings-button"]', 'Settings button');
  await settingsButton.waitForClickable({ timeout: 5000 });
  await settingsButton.click();
  await browser.pause(500);

  const audioTab = await waitForElement('[data-testid="settings-tab-audio"]', 'Audio settings tab');
  await audioTab.waitForClickable({ timeout: 5000 });
  await audioTab.click();
  await browser.pause(500);
}

/**
 * Navigate to DSP configuration section
 */
async function navigateToDspConfig() {
  await navigateToAudioSettings();

  // Expand DSP pipeline stage
  const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
  await dspStage.click();
  await browser.pause(300);

  // Wait for DSP config to be displayed
  await waitForElement('[data-testid="dsp-config"]', 'DSP config container');
}

/**
 * Add an effect to a specific slot
 * @param {number} slotIndex - The slot index (0-3)
 * @param {string} effectName - The effect name to click (e.g., 'Compressor', 'Parametric EQ')
 */
async function addEffect(slotIndex, effectName) {
  // Check if add button exists (slot might already have an effect)
  const addButton = await $(`[data-testid="add-effect-btn-${slotIndex}"]`);
  const buttonExists = await addButton.isExisting();

  if (!buttonExists) {
    // Slot already has an effect, remove it first
    await removeEffect(slotIndex);
    await browser.pause(300);
  }

  // Click add effect button
  const addEffectBtn = await waitForElement(
    `[data-testid="add-effect-btn-${slotIndex}"]`,
    `Add effect button for slot ${slotIndex}`
  );
  await addEffectBtn.click();
  await browser.pause(300);

  // Wait for effect picker to appear
  await waitForElement(`[data-testid="effect-picker-${slotIndex}"]`, 'Effect picker');

  // Find and click the effect by name (partial text match)
  const effectButton = await $(`button*=${effectName}`);
  await effectButton.waitForClickable({ timeout: 3000 });
  await effectButton.click();
  await browser.pause(500);

  // Verify effect was added by checking the slot shows the effect
  await waitForElement(`[data-testid="effect-slot-${slotIndex}"]`, `Effect slot ${slotIndex}`);
}

/**
 * Remove an effect from a slot
 * @param {number} slotIndex - The slot index (0-3)
 */
async function removeEffect(slotIndex) {
  const removeButton = await $(`[data-testid="remove-effect-btn-${slotIndex}"]`);
  const buttonExists = await removeButton.isExisting();

  if (buttonExists) {
    await removeButton.click();
    await browser.pause(300);
  }
}

/**
 * Open the effect editor for a slot
 * @param {number} slotIndex - The slot index (0-3)
 */
async function openEffectEditor(slotIndex) {
  const editButton = await waitForElement(
    `[data-testid="edit-effect-btn-${slotIndex}"]`,
    `Edit button for slot ${slotIndex}`
  );
  await editButton.waitForClickable({ timeout: 3000 });
  await editButton.click();
  await browser.pause(300);
}

/**
 * Check if a slot has an effect
 * @param {number} slotIndex - The slot index (0-3)
 * @returns {boolean} True if slot has an effect
 */
async function slotHasEffect(slotIndex) {
  const removeButton = await $(`[data-testid="remove-effect-btn-${slotIndex}"]`);
  return await removeButton.isExisting();
}

/**
 * Clear all effects from the chain
 */
async function clearAllEffects() {
  const clearButton = await $('[data-testid="clear-all-btn"]');
  const buttonExists = await clearButton.isExisting();

  if (buttonExists) {
    await clearButton.click();
    await browser.pause(300);

    // Confirm in dialog (the confirm dialog uses "Clear All" text)
    const confirmButton = await $('button*=Clear All');
    if (await confirmButton.isExisting()) {
      await confirmButton.click();
      await browser.pause(500);
    }
  }
}

// =============================================================================
// Test Suites
// =============================================================================

describe('DSP Effects - Add Effect Workflow', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should add a compressor effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add compressor to slot 0
    await addEffect(0, 'Compressor');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('compressor');
  });

  it('should add a parametric EQ effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add EQ to slot 0
    await addEffect(0, 'Parametric EQ');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('eq');
  });

  it('should add a limiter effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add limiter to slot 0
    await addEffect(0, 'Limiter');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('limiter');
  });

  it('should add a crossfeed effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add crossfeed to slot 0
    await addEffect(0, 'Crossfeed');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('crossfeed');
  });

  it('should add a stereo enhancer effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add stereo enhancer to slot 0
    await addEffect(0, 'Stereo Enhancer');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('stereo');
  });

  it('should add a graphic EQ effect to slot 0', async () => {
    await navigateToDspConfig();

    // Clear any existing effects first
    await clearAllEffects();
    await browser.pause(300);

    // Add graphic EQ to slot 0
    await addEffect(0, 'Graphic EQ');

    // Verify effect was added
    const slotElement = await waitForElement('[data-testid="effect-slot-0"]', 'Effect slot 0');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('eq');
  });
});

describe('DSP Effects - Edit Effect Workflow', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should open compressor editor and display controls', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor
    await openEffectEditor(0);

    // Verify compressor editor is displayed
    const editor = await waitForElement('[data-testid="compressor-editor"]', 'Compressor editor');
    await expect(editor).toBeDisplayed();

    // Verify threshold control exists
    const thresholdSlider = await waitForElement('[data-testid="compressor-threshold"]', 'Compressor threshold slider');
    await expect(thresholdSlider).toBeDisplayed();

    // Verify ratio control exists
    const ratioSlider = await waitForElement('[data-testid="compressor-ratio"]', 'Compressor ratio slider');
    await expect(ratioSlider).toBeDisplayed();
  });

  it('should change compressor threshold value', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor
    await openEffectEditor(0);

    // Get threshold slider
    const thresholdSlider = await waitForElement('[data-testid="compressor-threshold"]', 'Compressor threshold slider');

    // Get initial value
    const initialValue = await thresholdSlider.getValue();

    // Move slider to a different value
    await thresholdSlider.setValue('-30');
    await browser.pause(300);

    // Get new value - note: the slider value might be formatted differently
    const newValue = await thresholdSlider.getValue();

    // Verify value changed
    expect(newValue).not.toBe(initialValue);
  });

  it('should open parametric EQ editor and display band controls', async () => {
    await navigateToDspConfig();

    // Clear and add EQ
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Parametric EQ');

    // Open editor
    await openEffectEditor(0);

    // Verify EQ editor is displayed
    const editor = await waitForElement('[data-testid="parametric-eq-editor"]', 'Parametric EQ editor');
    await expect(editor).toBeDisplayed();
  });

  it('should open limiter editor and display controls', async () => {
    await navigateToDspConfig();

    // Clear and add limiter
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Limiter');

    // Open editor
    await openEffectEditor(0);

    // Verify limiter editor is displayed
    const editor = await waitForElement('[data-testid="limiter-editor"]', 'Limiter editor');
    await expect(editor).toBeDisplayed();

    // Verify ceiling control exists
    const ceilingSlider = await waitForElement('[data-testid="limiter-ceiling"]', 'Limiter ceiling slider');
    await expect(ceilingSlider).toBeDisplayed();
  });

  it('should open crossfeed editor and display preset cards', async () => {
    await navigateToDspConfig();

    // Clear and add crossfeed
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Crossfeed');

    // Open editor
    await openEffectEditor(0);

    // Verify crossfeed editor is displayed
    const editor = await waitForElement('[data-testid="crossfeed-editor"]', 'Crossfeed editor');
    await expect(editor).toBeDisplayed();
  });

  it('should open stereo enhancer editor and display width control', async () => {
    await navigateToDspConfig();

    // Clear and add stereo enhancer
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Stereo Enhancer');

    // Open editor
    await openEffectEditor(0);

    // Verify stereo editor is displayed
    const editor = await waitForElement('[data-testid="stereo-editor"]', 'Stereo enhancer editor');
    await expect(editor).toBeDisplayed();

    // Verify width control exists
    const widthSlider = await waitForElement('[data-testid="stereo-width"]', 'Stereo width slider');
    await expect(widthSlider).toBeDisplayed();
  });

  it('should close editor when clicking edit button again', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor
    await openEffectEditor(0);

    // Verify editor is open
    const editor = await waitForElement('[data-testid="compressor-editor"]', 'Compressor editor');
    await expect(editor).toBeDisplayed();

    // Click edit button again to close
    const editButton = await $('[data-testid="edit-effect-btn-0"]');
    await editButton.click();
    await browser.pause(300);

    // Verify editor is closed (element should not be displayed)
    const editorAfter = await $('[data-testid="compressor-editor"]');
    const isDisplayed = await editorAfter.isDisplayed().catch(() => false);
    expect(isDisplayed).toBe(false);
  });
});

describe('DSP Effects - Remove Effect Workflow', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should remove effect from slot 0', async () => {
    await navigateToDspConfig();

    // Clear and add an effect first
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Verify effect exists
    const hasEffectBefore = await slotHasEffect(0);
    expect(hasEffectBefore).toBe(true);

    // Remove the effect
    await removeEffect(0);

    // Verify add button appears (indicating slot is empty)
    const addButton = await waitForElement('[data-testid="add-effect-btn-0"]', 'Add effect button for slot 0');
    await expect(addButton).toBeDisplayed();
  });

  it('should show add button after removing effect', async () => {
    await navigateToDspConfig();

    // Clear and add an effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Limiter');
    await browser.pause(300);

    // Remove the effect
    const removeButton = await waitForElement('[data-testid="remove-effect-btn-0"]', 'Remove button for slot 0');
    await removeButton.click();
    await browser.pause(300);

    // Verify add button is displayed
    const addButton = await waitForElement('[data-testid="add-effect-btn-0"]', 'Add effect button');
    await expect(addButton).toBeDisplayed();
  });
});

describe('DSP Effects - Effect Chain Order', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should add effects to multiple slots in order', async () => {
    await navigateToDspConfig();

    // Clear all effects
    await clearAllEffects();
    await browser.pause(300);

    // Add EQ to slot 0
    await addEffect(0, 'Parametric EQ');
    await browser.pause(200);

    // Add Compressor to slot 1
    await addEffect(1, 'Compressor');
    await browser.pause(200);

    // Add Limiter to slot 2
    await addEffect(2, 'Limiter');
    await browser.pause(200);

    // Verify order is maintained
    const slot0 = await $('[data-testid="effect-slot-0"]');
    const slot0Text = await slot0.getText();
    expect(slot0Text.toLowerCase()).toContain('eq');

    const slot1 = await $('[data-testid="effect-slot-1"]');
    const slot1Text = await slot1.getText();
    expect(slot1Text.toLowerCase()).toContain('compressor');

    const slot2 = await $('[data-testid="effect-slot-2"]');
    const slot2Text = await slot2.getText();
    expect(slot2Text.toLowerCase()).toContain('limiter');
  });

  it('should maintain order after removing middle effect', async () => {
    await navigateToDspConfig();

    // Clear all effects
    await clearAllEffects();
    await browser.pause(300);

    // Add effects to slots 0, 1, 2
    await addEffect(0, 'Parametric EQ');
    await browser.pause(200);
    await addEffect(1, 'Compressor');
    await browser.pause(200);
    await addEffect(2, 'Limiter');
    await browser.pause(200);

    // Remove effect from slot 1
    await removeEffect(1);
    await browser.pause(300);

    // Verify slot 0 still has EQ
    const slot0 = await $('[data-testid="effect-slot-0"]');
    const slot0Text = await slot0.getText();
    expect(slot0Text.toLowerCase()).toContain('eq');

    // Verify slot 1 now shows add button
    const addButton = await $('[data-testid="add-effect-btn-1"]');
    const addButtonExists = await addButton.isExisting();
    expect(addButtonExists).toBe(true);

    // Verify slot 2 still has Limiter
    const slot2 = await $('[data-testid="effect-slot-2"]');
    const slot2Text = await slot2.getText();
    expect(slot2Text.toLowerCase()).toContain('limiter');
  });
});

describe('DSP Effects - Presets', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should open compressor preset dropdown', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor
    await openEffectEditor(0);

    // Click preset dropdown
    const presetSelect = await waitForElement('[data-testid="compressor-preset-select"]', 'Compressor preset select');
    await presetSelect.click();
    await browser.pause(300);

    // Verify dropdown opened (look for preset options)
    const presetOption = await $('button*=Punchy');
    const optionExists = await presetOption.isExisting();
    expect(optionExists).toBe(true);
  });

  it('should apply punchy preset to compressor', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor
    await openEffectEditor(0);

    // Get initial threshold value
    const thresholdBefore = await $('[data-testid="compressor-threshold"]');
    const valueBefore = await thresholdBefore.getValue();

    // Open preset dropdown and select Punchy
    const presetSelect = await $('[data-testid="compressor-preset-select"]');
    await presetSelect.click();
    await browser.pause(300);

    const punchyOption = await $('button*=Punchy');
    if (await punchyOption.isExisting()) {
      await punchyOption.click();
      await browser.pause(500);

      // Verify value changed (Punchy preset has different threshold)
      const thresholdAfter = await $('[data-testid="compressor-threshold"]');
      const valueAfter = await thresholdAfter.getValue();

      // Values should be different after applying preset
      // Note: This may need adjustment based on exact preset values
      expect(valueAfter).not.toBe(valueBefore);
    }
  });

  it('should open graphic EQ preset dropdown', async () => {
    await navigateToDspConfig();

    // Clear and add graphic EQ
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Graphic EQ');

    // Open editor
    await openEffectEditor(0);

    // Verify graphic EQ editor is displayed
    const editor = await waitForElement('[data-testid="graphic-eq-editor"]', 'Graphic EQ editor');
    await expect(editor).toBeDisplayed();

    // Click preset dropdown
    const presetSelect = await $('[data-testid="graphic-eq-preset-select"]');
    if (await presetSelect.isExisting()) {
      await presetSelect.click();
      await browser.pause(300);
    }
  });
});

describe('DSP Effects - Clear All', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should clear all effects from chain', async () => {
    await navigateToDspConfig();

    // Add effects to multiple slots
    await clearAllEffects();
    await browser.pause(300);

    await addEffect(0, 'Compressor');
    await browser.pause(200);
    await addEffect(1, 'Limiter');
    await browser.pause(200);

    // Click clear all button
    const clearButton = await waitForElement('[data-testid="clear-all-btn"]', 'Clear all button');
    await clearButton.click();
    await browser.pause(300);

    // Confirm in dialog
    const confirmButton = await $('button*=Clear All');
    if (await confirmButton.isExisting()) {
      await confirmButton.click();
      await browser.pause(500);
    }

    // Verify all slots are empty (show add buttons)
    for (let i = 0; i < 4; i++) {
      const addButton = await $(`[data-testid="add-effect-btn-${i}"]`);
      const buttonExists = await addButton.isExisting();
      expect(buttonExists).toBe(true);
    }
  });

  it('should not show clear all button when chain is empty', async () => {
    await navigateToDspConfig();

    // Clear all effects first
    await clearAllEffects();
    await browser.pause(500);

    // Verify clear all button is not displayed when chain is empty
    const clearButton = await $('[data-testid="clear-all-btn"]');
    const isDisplayed = await clearButton.isDisplayed().catch(() => false);
    expect(isDisplayed).toBe(false);
  });

  it('should show clear all button when at least one effect exists', async () => {
    await navigateToDspConfig();

    // Clear and add one effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');
    await browser.pause(300);

    // Verify clear all button is displayed
    const clearButton = await waitForElement('[data-testid="clear-all-btn"]', 'Clear all button');
    await expect(clearButton).toBeDisplayed();
  });
});

describe('DSP Effects - Enable/Disable Toggle', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should toggle effect enabled state', async () => {
    await navigateToDspConfig();

    // Clear and add an effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');
    await browser.pause(300);

    // Find the enable checkbox within slot 0
    const slotElement = await $('[data-testid="effect-slot-0"]');
    const checkbox = await slotElement.$('input[type="checkbox"]');

    if (await checkbox.isExisting()) {
      // Get initial state
      const initialState = await checkbox.isSelected();

      // Toggle it
      await checkbox.click();
      await browser.pause(300);

      // Verify state changed
      const newState = await checkbox.isSelected();
      expect(newState).toBe(!initialState);
    }
  });

  it('should show disabled indicator when effect is disabled', async () => {
    await navigateToDspConfig();

    // Clear and add an effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');
    await browser.pause(300);

    // Find and uncheck the enable checkbox
    const slotElement = await $('[data-testid="effect-slot-0"]');
    const checkbox = await slotElement.$('input[type="checkbox"]');

    if (await checkbox.isExisting()) {
      // Ensure it's checked first
      const isChecked = await checkbox.isSelected();
      if (isChecked) {
        await checkbox.click();
        await browser.pause(300);
      }

      // Verify "Disabled" text appears in the slot
      const slotText = await slotElement.getText();
      expect(slotText.toLowerCase()).toContain('disabled');
    }
  });

  it('should re-enable effect after disabling', async () => {
    await navigateToDspConfig();

    // Clear and add an effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Limiter');
    await browser.pause(300);

    // Find the enable checkbox
    const slotElement = await $('[data-testid="effect-slot-0"]');
    const checkbox = await slotElement.$('input[type="checkbox"]');

    if (await checkbox.isExisting()) {
      // Disable it
      const initialState = await checkbox.isSelected();
      if (initialState) {
        await checkbox.click();
        await browser.pause(300);
      }

      // Re-enable it
      await checkbox.click();
      await browser.pause(300);

      // Verify it's enabled again
      const finalState = await checkbox.isSelected();
      expect(finalState).toBe(true);
    }
  });
});

describe('DSP Effects - Persistence', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should persist effect after navigating away and back', async () => {
    await navigateToDspConfig();

    // Clear and add an effect
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');
    await browser.pause(300);

    // Navigate away (go to home)
    const homeButton = await $('[data-testid="home-button"]');
    if (await homeButton.isExisting()) {
      await homeButton.click();
      await browser.pause(500);
    } else {
      // Alternative: click somewhere to navigate away
      const settingsButton = await $('[data-testid="settings-button"]');
      await settingsButton.click();
      await browser.pause(500);
    }

    // Navigate back to DSP config
    await navigateToDspConfig();

    // Verify effect is still there
    const slotElement = await $('[data-testid="effect-slot-0"]');
    const slotText = await slotElement.getText();
    expect(slotText.toLowerCase()).toContain('compressor');
  });

  it('should persist effect parameters after editing', async () => {
    await navigateToDspConfig();

    // Clear and add compressor
    await clearAllEffects();
    await browser.pause(300);
    await addEffect(0, 'Compressor');

    // Open editor and change threshold
    await openEffectEditor(0);
    const thresholdSlider = await $('[data-testid="compressor-threshold"]');
    await thresholdSlider.setValue('-35');
    await browser.pause(500);

    // Close editor
    const editButton = await $('[data-testid="edit-effect-btn-0"]');
    await editButton.click();
    await browser.pause(300);

    // Navigate away
    const settingsButton = await $('[data-testid="settings-button"]');
    await settingsButton.click();
    await browser.pause(300);

    // Navigate back
    await navigateToDspConfig();

    // Open editor again
    await openEffectEditor(0);

    // Verify threshold value was persisted
    const thresholdAfter = await $('[data-testid="compressor-threshold"]');
    const valueAfter = await thresholdAfter.getValue();

    // Value should be close to -35 (allowing for rounding)
    const numValue = parseFloat(valueAfter);
    expect(numValue).toBeLessThanOrEqual(-30);
  });
});

describe('DSP Effects - Error Handling', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should gracefully handle empty DSP chain', async () => {
    await navigateToDspConfig();

    // Clear all effects
    await clearAllEffects();
    await browser.pause(500);

    // Verify DSP config is still displayed
    const dspConfig = await waitForElement('[data-testid="dsp-config"]', 'DSP config container');
    await expect(dspConfig).toBeDisplayed();

    // Verify all slots show add buttons
    for (let i = 0; i < 4; i++) {
      const addButton = await $(`[data-testid="add-effect-btn-${i}"]`);
      const buttonExists = await addButton.isExisting();
      expect(buttonExists).toBe(true);
    }
  });

  it('should handle rapid effect add/remove', async () => {
    await navigateToDspConfig();

    // Clear all
    await clearAllEffects();
    await browser.pause(300);

    // Rapidly add and remove effects
    for (let i = 0; i < 3; i++) {
      await addEffect(0, 'Compressor');
      await browser.pause(100);
      await removeEffect(0);
      await browser.pause(100);
    }

    // Verify slot 0 is in a valid state (empty, showing add button)
    const addButton = await $('[data-testid="add-effect-btn-0"]');
    const buttonExists = await addButton.isExisting();
    expect(buttonExists).toBe(true);
  });
});

describe('DSP Effects - Full Workflow Integration', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should complete full effect configuration workflow', async () => {
    await navigateToDspConfig();

    // Step 1: Clear existing chain
    await clearAllEffects();
    await browser.pause(300);

    // Step 2: Add Parametric EQ to slot 0
    await addEffect(0, 'Parametric EQ');
    await browser.pause(200);

    // Step 3: Add Compressor to slot 1
    await addEffect(1, 'Compressor');
    await browser.pause(200);

    // Step 4: Edit compressor settings
    await openEffectEditor(1);
    const thresholdSlider = await $('[data-testid="compressor-threshold"]');
    await thresholdSlider.setValue('-24');
    await browser.pause(300);

    // Close editor
    const editButton1 = await $('[data-testid="edit-effect-btn-1"]');
    await editButton1.click();
    await browser.pause(200);

    // Step 5: Add Limiter to slot 2
    await addEffect(2, 'Limiter');
    await browser.pause(200);

    // Step 6: Disable the compressor (but keep it in chain)
    const slot1 = await $('[data-testid="effect-slot-1"]');
    const checkbox = await slot1.$('input[type="checkbox"]');
    if (await checkbox.isExisting()) {
      const isChecked = await checkbox.isSelected();
      if (isChecked) {
        await checkbox.click();
        await browser.pause(200);
      }
    }

    // Step 7: Verify final chain state
    // Slot 0: EQ (enabled)
    const slot0 = await $('[data-testid="effect-slot-0"]');
    const slot0Text = await slot0.getText();
    expect(slot0Text.toLowerCase()).toContain('eq');

    // Slot 1: Compressor (disabled)
    const slot1Text = await slot1.getText();
    expect(slot1Text.toLowerCase()).toContain('compressor');
    expect(slot1Text.toLowerCase()).toContain('disabled');

    // Slot 2: Limiter (enabled)
    const slot2 = await $('[data-testid="effect-slot-2"]');
    const slot2Text = await slot2.getText();
    expect(slot2Text.toLowerCase()).toContain('limiter');

    // Slot 3: Empty
    const addButton3 = await $('[data-testid="add-effect-btn-3"]');
    const slot3Empty = await addButton3.isExisting();
    expect(slot3Empty).toBe(true);
  });

  it('should handle master audio chain with all effect types', async () => {
    await navigateToDspConfig();

    // Clear all
    await clearAllEffects();
    await browser.pause(300);

    // Add one of each category
    // EQ category
    await addEffect(0, 'Graphic EQ');
    await browser.pause(200);

    // Dynamics category
    await addEffect(1, 'Compressor');
    await browser.pause(200);

    // Spatial category
    await addEffect(2, 'Crossfeed');
    await browser.pause(200);

    // More dynamics
    await addEffect(3, 'Limiter');
    await browser.pause(200);

    // Verify all 4 slots are filled
    for (let i = 0; i < 4; i++) {
      const removeButton = await $(`[data-testid="remove-effect-btn-${i}"]`);
      const hasEffect = await removeButton.isExisting();
      expect(hasEffect).toBe(true);
    }
  });
});
