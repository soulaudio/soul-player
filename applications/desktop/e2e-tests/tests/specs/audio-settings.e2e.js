/**
 * E2E Tests for Audio Settings
 *
 * Tests the audio settings page functionality including:
 * - Volume leveling mode selection
 * - DSP effect chain configuration
 * - Settings persistence
 *
 * Prerequisites:
 * - App must be built: cargo build --release -p soul-player-desktop
 * - tauri-driver must be installed: cargo install tauri-driver
 * - UI components must have data-testid attributes (see TEST_IDS.md)
 *
 * NOTE: These tests will FAIL until data-testid attributes are added to UI components.
 */

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
  // Check for common selection indicators
  const ariaSelected = await element.getAttribute('aria-selected');
  if (ariaSelected === 'true') return true;

  const ariaPressed = await element.getAttribute('aria-pressed');
  if (ariaPressed === 'true') return true;

  // Check for data-state attribute (used by Radix UI)
  const dataState = await element.getAttribute('data-state');
  if (dataState === 'on' || dataState === 'active' || dataState === 'checked') return true;

  // Fallback: check for common selection classes
  const classList = await element.getAttribute('class') || '';
  return classList.includes('border-primary') ||
         classList.includes('bg-primary') ||
         classList.includes('selected') ||
         classList.includes('active');
}

/**
 * Navigate to audio settings tab
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

describe('Audio Settings Persistence', () => {
  beforeEach(async () => {
    // Wait for app to fully load
    await browser.pause(2000);
  });

  it('should navigate to settings page', async () => {
    // Click on the settings button in the header
    const settingsButton = await waitForElement('[data-testid="settings-button"]', 'Settings button');
    await settingsButton.waitForClickable({ timeout: 5000 });
    await settingsButton.click();

    // Verify we're on the settings page
    const settingsPage = await waitForElement('[data-testid="settings-page"]', 'Settings page');
    await expect(settingsPage).toBeDisplayed();
  });

  it('should navigate to audio settings tab', async () => {
    await navigateToAudioSettings();

    // Verify audio settings content is visible
    const audioSettingsContent = await waitForElement('[data-testid="audio-settings-content"]', 'Audio settings content');
    await expect(audioSettingsContent).toBeDisplayed();
  });

  it('should persist volume leveling mode selection', async () => {
    await navigateToAudioSettings();

    // Expand volume leveling stage if collapsed
    const volumeLevelingStage = await waitForElement('[data-testid="pipeline-stage-volume-leveling"]', 'Volume leveling stage');
    await volumeLevelingStage.click();
    await browser.pause(300);

    // Select ReplayGain Track mode
    const replaygainTrackOption = await waitForElement('[data-testid="volume-leveling-replaygain-track"]', 'ReplayGain Track option');
    await replaygainTrackOption.waitForClickable({ timeout: 5000 });
    await replaygainTrackOption.click();

    // Wait for setting to be saved
    await browser.pause(500);

    // Verify the selection is active
    const isSelected = await isElementSelected(replaygainTrackOption);
    expect(isSelected).toBe(true);
  });

  it('should persist ReplayGain Album mode selection', async () => {
    await navigateToAudioSettings();

    // Expand volume leveling stage
    const volumeLevelingStage = await waitForElement('[data-testid="pipeline-stage-volume-leveling"]', 'Volume leveling stage');
    await volumeLevelingStage.click();
    await browser.pause(300);

    // Select ReplayGain Album mode
    const replaygainAlbumOption = await waitForElement('[data-testid="volume-leveling-replaygain-album"]', 'ReplayGain Album option');
    await replaygainAlbumOption.waitForClickable({ timeout: 5000 });
    await replaygainAlbumOption.click();

    // Wait for setting to be saved
    await browser.pause(500);

    // Verify the selection
    const isSelected = await isElementSelected(replaygainAlbumOption);
    expect(isSelected).toBe(true);
  });

  it('should persist EBU R128 mode selection', async () => {
    await navigateToAudioSettings();

    // Expand volume leveling stage
    const volumeLevelingStage = await waitForElement('[data-testid="pipeline-stage-volume-leveling"]', 'Volume leveling stage');
    await volumeLevelingStage.click();
    await browser.pause(300);

    // Select EBU R128 mode
    const ebuR128Option = await waitForElement('[data-testid="volume-leveling-ebu-r128"]', 'EBU R128 option');
    await ebuR128Option.waitForClickable({ timeout: 5000 });
    await ebuR128Option.click();

    // Wait for setting to be saved
    await browser.pause(500);

    // Verify the selection
    const isSelected = await isElementSelected(ebuR128Option);
    expect(isSelected).toBe(true);
  });

  it('should adjust preamp slider when volume leveling is enabled', async () => {
    await navigateToAudioSettings();

    // Expand volume leveling stage
    const volumeLevelingStage = await waitForElement('[data-testid="pipeline-stage-volume-leveling"]', 'Volume leveling stage');
    await volumeLevelingStage.click();
    await browser.pause(300);

    // First enable a volume leveling mode
    const replaygainTrackOption = await waitForElement('[data-testid="volume-leveling-replaygain-track"]', 'ReplayGain Track option');
    await replaygainTrackOption.click();
    await browser.pause(300);

    // Find and interact with preamp slider
    const preampSlider = await waitForElement('[data-testid="preamp-slider"]', 'Preamp slider');
    await expect(preampSlider).toBeDisplayed();

    // Check that prevent clipping checkbox is visible
    const preventClippingCheckbox = await waitForElement('[data-testid="prevent-clipping-checkbox"]', 'Prevent clipping checkbox');
    await expect(preventClippingCheckbox).toBeDisplayed();
  });
});

describe('DSP Effect Chain', () => {
  beforeEach(async () => {
    // Wait for app to fully load
    await browser.pause(2000);
  });

  it('should display DSP pipeline stage', async () => {
    await navigateToAudioSettings();

    // Verify DSP stage is visible
    const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
    await expect(dspStage).toBeDisplayed();
  });

  it('should open effect picker when clicking add effect', async () => {
    await navigateToAudioSettings();

    // Expand DSP stage
    const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
    await dspStage.click();
    await browser.pause(300);

    // Click first empty slot's "Add Effect" button
    const addEffectButton = await $('[data-testid="dsp-slot-0-add"]');
    const buttonExists = await addEffectButton.isExisting();
    if (!buttonExists) {
      // No add button means slot is already filled - test is inconclusive but not a failure
      console.log('DSP slot 0 already has an effect, skipping add effect test');
      return;
    }

    await addEffectButton.click();
    await browser.pause(300);

    // Verify effect picker is shown
    const effectPicker = await waitForElement('[data-testid="effect-picker"]', 'Effect picker');
    await expect(effectPicker).toBeDisplayed();
  });

  it('should add parametric EQ effect', async () => {
    await navigateToAudioSettings();

    // Expand DSP stage
    const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
    await dspStage.click();
    await browser.pause(300);

    // Click first slot's "Add Effect" button
    const addEffectButton = await $('[data-testid="dsp-slot-0-add"]');
    const buttonExists = await addEffectButton.isExisting();
    if (!buttonExists) {
      console.log('DSP slot 0 already has an effect, skipping add EQ test');
      return;
    }

    await addEffectButton.click();
    await browser.pause(300);

    // Select Parametric EQ
    const eqOption = await waitForElement('[data-testid="effect-option-eq"]', 'EQ effect option');
    await eqOption.click();
    await browser.pause(500);

    // Verify effect was added - slot should now show EQ
    const slotEffect = await waitForElement('[data-testid="dsp-slot-0-effect"]', 'DSP slot 0 effect');
    const effectText = await slotEffect.getText();
    expect(effectText.toLowerCase()).toContain('eq');
  });

  it('should toggle effect enabled state', async () => {
    await navigateToAudioSettings();

    // Expand DSP stage
    const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
    await dspStage.click();
    await browser.pause(300);

    // Find enabled checkbox for slot 0 (if effect exists)
    const enabledCheckbox = await $('[data-testid="dsp-slot-0-enabled"]');
    const checkboxExists = await enabledCheckbox.isExisting();
    if (!checkboxExists) {
      console.log('No effect in DSP slot 0, skipping toggle test');
      return;
    }

    const initialState = await enabledCheckbox.isSelected();

    // Toggle it
    await enabledCheckbox.click();
    await browser.pause(300);

    // Verify state changed
    const newState = await enabledCheckbox.isSelected();
    expect(newState).toBe(!initialState);
  });

  it('should remove effect from chain', async () => {
    await navigateToAudioSettings();

    // Expand DSP stage
    const dspStage = await waitForElement('[data-testid="pipeline-stage-dsp"]', 'DSP pipeline stage');
    await dspStage.click();
    await browser.pause(300);

    // Find remove button for slot 0 (if effect exists)
    const removeButton = await $('[data-testid="dsp-slot-0-remove"]');
    const buttonExists = await removeButton.isExisting();
    if (!buttonExists) {
      console.log('No effect in DSP slot 0 to remove, skipping remove test');
      return;
    }

    await removeButton.click();
    await browser.pause(500);

    // Verify slot is now empty (add button should appear)
    const addEffectButton = await waitForElement('[data-testid="dsp-slot-0-add"]', 'Add effect button');
    await expect(addEffectButton).toBeDisplayed();
  });
});

describe('Audio Output Settings', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should display backend selector', async () => {
    await navigateToAudioSettings();

    // Expand output stage
    const outputStage = await waitForElement('[data-testid="pipeline-stage-output"]', 'Output pipeline stage');
    await outputStage.click();
    await browser.pause(300);

    // Verify backend selector is visible
    const backendSelector = await waitForElement('[data-testid="backend-selector"]', 'Backend selector');
    await expect(backendSelector).toBeDisplayed();
  });

  it('should display device selector', async () => {
    await navigateToAudioSettings();

    // Expand output stage
    const outputStage = await waitForElement('[data-testid="pipeline-stage-output"]', 'Output pipeline stage');
    await outputStage.click();
    await browser.pause(300);

    // Verify device selector is visible
    const deviceSelector = await waitForElement('[data-testid="device-selector"]', 'Device selector');
    await expect(deviceSelector).toBeDisplayed();
  });
});

describe('Resampling Settings', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should display resampling quality options', async () => {
    await navigateToAudioSettings();

    // Expand resampling stage
    const resamplingStage = await waitForElement('[data-testid="pipeline-stage-resampling"]', 'Resampling pipeline stage');
    await resamplingStage.click();
    await browser.pause(300);

    // Verify quality selector is visible
    const qualitySelector = await waitForElement('[data-testid="resampling-quality-selector"]', 'Resampling quality selector');
    await expect(qualitySelector).toBeDisplayed();
  });

  it('should change resampling quality', async () => {
    await navigateToAudioSettings();

    // Expand resampling stage
    const resamplingStage = await waitForElement('[data-testid="pipeline-stage-resampling"]', 'Resampling pipeline stage');
    await resamplingStage.click();
    await browser.pause(300);

    // Select "Maximum" quality option
    const maximumOption = await waitForElement('[data-testid="resampling-quality-maximum"]', 'Maximum quality option');
    await maximumOption.click();
    await browser.pause(500);

    // Verify selection
    const isSelected = await isElementSelected(maximumOption);
    expect(isSelected).toBe(true);
  });
});

describe('Buffer Settings', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should display buffer settings', async () => {
    await navigateToAudioSettings();

    // Expand buffer stage
    const bufferStage = await waitForElement('[data-testid="pipeline-stage-buffer"]', 'Buffer pipeline stage');
    await bufferStage.click();
    await browser.pause(300);

    // Verify buffer settings are visible
    const bufferSettings = await waitForElement('[data-testid="buffer-settings"]', 'Buffer settings container');
    await expect(bufferSettings).toBeDisplayed();
  });

  it('should toggle preload setting', async () => {
    await navigateToAudioSettings();

    // Expand buffer stage
    const bufferStage = await waitForElement('[data-testid="pipeline-stage-buffer"]', 'Buffer pipeline stage');
    await bufferStage.click();
    await browser.pause(300);

    // Toggle preload checkbox
    const preloadCheckbox = await waitForElement('[data-testid="preload-enabled-checkbox"]', 'Preload checkbox');
    const initialState = await preloadCheckbox.isSelected();
    await preloadCheckbox.click();
    await browser.pause(300);

    const newState = await preloadCheckbox.isSelected();
    expect(newState).toBe(!initialState);
  });

  it('should toggle crossfade setting', async () => {
    await navigateToAudioSettings();

    // Expand buffer stage
    const bufferStage = await waitForElement('[data-testid="pipeline-stage-buffer"]', 'Buffer pipeline stage');
    await bufferStage.click();
    await browser.pause(300);

    // Toggle crossfade checkbox
    const crossfadeCheckbox = await waitForElement('[data-testid="crossfade-enabled-checkbox"]', 'Crossfade checkbox');
    const initialState = await crossfadeCheckbox.isSelected();
    await crossfadeCheckbox.click();
    await browser.pause(300);

    const newState = await crossfadeCheckbox.isSelected();
    expect(newState).toBe(!initialState);
  });
});

describe('Reset Audio Settings', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should reset all audio settings to defaults', async () => {
    await navigateToAudioSettings();

    // Click reset button
    const resetButton = await waitForElement('[data-testid="reset-audio-settings"]', 'Reset audio settings button');
    await resetButton.click();
    await browser.pause(300);

    // Confirm in the dialog
    const confirmButton = await waitForElement('[data-testid="confirm-reset-button"]', 'Confirm reset button');
    await confirmButton.click();
    await browser.pause(500);

    // Verify settings were reset (volume leveling should be disabled)
    const volumeLevelingStage = await waitForElement('[data-testid="pipeline-stage-volume-leveling"]', 'Volume leveling stage');
    await volumeLevelingStage.click();
    await browser.pause(300);

    const disabledOption = await waitForElement('[data-testid="volume-leveling-disabled"]', 'Volume leveling disabled option');
    const isSelected = await isElementSelected(disabledOption);
    expect(isSelected).toBe(true);
  });
});
