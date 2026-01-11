/**
 * E2E Tests for Navigation
 *
 * Tests basic navigation functionality:
 * - Home page
 * - Library page
 * - Settings page
 * - Search
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

describe('App Navigation', () => {
  beforeEach(async () => {
    // Wait for app to fully load
    await browser.pause(2000);
  });

  it('should load the home page by default', async () => {
    const homePage = await waitForElement('[data-testid="home-page"]', 'Home page container');
    await expect(homePage).toBeDisplayed();
  });

  it('should navigate to library page', async () => {
    const libraryTab = await waitForElement('[data-testid="nav-library"]', 'Library navigation tab');
    await libraryTab.waitForClickable({ timeout: 5000 });
    await libraryTab.click();
    await browser.pause(500);

    const libraryPage = await waitForElement('[data-testid="library-page"]', 'Library page container');
    await expect(libraryPage).toBeDisplayed();
  });

  it('should navigate to settings page', async () => {
    const settingsButton = await waitForElement('[data-testid="settings-button"]', 'Settings button');
    await settingsButton.waitForClickable({ timeout: 5000 });
    await settingsButton.click();
    await browser.pause(500);

    const settingsPage = await waitForElement('[data-testid="settings-page"]', 'Settings page container');
    await expect(settingsPage).toBeDisplayed();
  });

  it('should navigate to search page', async () => {
    const searchButton = await waitForElement('[data-testid="search-button"]', 'Search button');
    await searchButton.waitForClickable({ timeout: 5000 });
    await searchButton.click();
    await browser.pause(500);

    const searchPage = await waitForElement('[data-testid="search-page"]', 'Search page container');
    await expect(searchPage).toBeDisplayed();
  });

  it('should navigate back to home from settings', async () => {
    // First go to settings
    const settingsButton = await waitForElement('[data-testid="settings-button"]', 'Settings button');
    await settingsButton.waitForClickable({ timeout: 5000 });
    await settingsButton.click();
    await browser.pause(500);

    // Then go home
    const homeButton = await waitForElement('[data-testid="home-button"]', 'Home button');
    await homeButton.waitForClickable({ timeout: 5000 });
    await homeButton.click();
    await browser.pause(500);

    const homePage = await waitForElement('[data-testid="home-page"]', 'Home page container');
    await expect(homePage).toBeDisplayed();
  });

  it('should show queue sidebar when queue button is clicked', async () => {
    const queueButton = await waitForElement('[data-testid="queue-button"]', 'Queue toggle button');
    await queueButton.waitForClickable({ timeout: 5000 });
    await queueButton.click();
    await browser.pause(500);

    const queueSidebar = await waitForElement('[data-testid="queue-sidebar"]', 'Queue sidebar');
    await expect(queueSidebar).toBeDisplayed();
  });

  it('should hide queue sidebar when close is clicked', async () => {
    // First open queue
    const queueButton = await waitForElement('[data-testid="queue-button"]', 'Queue toggle button');
    await queueButton.waitForClickable({ timeout: 5000 });
    await queueButton.click();
    await browser.pause(500);

    // Then close it
    const closeQueueButton = await waitForElement('[data-testid="queue-close"]', 'Queue close button');
    await closeQueueButton.waitForClickable({ timeout: 5000 });
    await closeQueueButton.click();
    await browser.pause(500);

    // Queue sidebar should either not exist or not be displayed
    const queueSidebar = await $('[data-testid="queue-sidebar"]');
    const isDisplayed = await queueSidebar.isDisplayed().catch(() => false);
    expect(isDisplayed).toBe(false);
  });
});

describe('Settings Navigation', () => {
  beforeEach(async () => {
    await browser.pause(2000);

    // Navigate to settings first
    const settingsButton = await waitForElement('[data-testid="settings-button"]', 'Settings button');
    await settingsButton.waitForClickable({ timeout: 5000 });
    await settingsButton.click();
    await browser.pause(500);
  });

  it('should navigate to general settings tab', async () => {
    const generalTab = await waitForElement('[data-testid="settings-tab-general"]', 'General settings tab');
    await generalTab.waitForClickable({ timeout: 5000 });
    await generalTab.click();
    await browser.pause(300);

    const generalContent = await waitForElement('[data-testid="general-settings-content"]', 'General settings content');
    await expect(generalContent).toBeDisplayed();
  });

  it('should navigate to library settings tab', async () => {
    const libraryTab = await waitForElement('[data-testid="settings-tab-library"]', 'Library settings tab');
    await libraryTab.waitForClickable({ timeout: 5000 });
    await libraryTab.click();
    await browser.pause(300);

    const libraryContent = await waitForElement('[data-testid="library-settings-content"]', 'Library settings content');
    await expect(libraryContent).toBeDisplayed();
  });

  it('should navigate to sources settings tab', async () => {
    const sourcesTab = await waitForElement('[data-testid="settings-tab-sources"]', 'Sources settings tab');
    await sourcesTab.waitForClickable({ timeout: 5000 });
    await sourcesTab.click();
    await browser.pause(300);

    const sourcesContent = await waitForElement('[data-testid="sources-settings-content"]', 'Sources settings content');
    await expect(sourcesContent).toBeDisplayed();
  });

  it('should navigate to audio settings tab', async () => {
    const audioTab = await waitForElement('[data-testid="settings-tab-audio"]', 'Audio settings tab');
    await audioTab.waitForClickable({ timeout: 5000 });
    await audioTab.click();
    await browser.pause(300);

    const audioContent = await waitForElement('[data-testid="audio-settings-content"]', 'Audio settings content');
    await expect(audioContent).toBeDisplayed();
  });

  it('should navigate to shortcuts settings tab', async () => {
    const shortcutsTab = await waitForElement('[data-testid="settings-tab-shortcuts"]', 'Shortcuts settings tab');
    await shortcutsTab.waitForClickable({ timeout: 5000 });
    await shortcutsTab.click();
    await browser.pause(300);

    const shortcutsContent = await waitForElement('[data-testid="shortcuts-settings-content"]', 'Shortcuts settings content');
    await expect(shortcutsContent).toBeDisplayed();
  });

  it('should navigate to about settings tab', async () => {
    const aboutTab = await waitForElement('[data-testid="settings-tab-about"]', 'About settings tab');
    await aboutTab.waitForClickable({ timeout: 5000 });
    await aboutTab.click();
    await browser.pause(300);

    const aboutContent = await waitForElement('[data-testid="about-settings-content"]', 'About settings content');
    await expect(aboutContent).toBeDisplayed();
  });
});

describe('Keyboard Shortcuts', () => {
  beforeEach(async () => {
    await browser.pause(2000);
  });

  it('should open search with Ctrl+K', async () => {
    // Press Ctrl+K (note: on macOS this should be Command+K, but WebDriver uses Control)
    await browser.keys(['Control', 'k']);
    await browser.pause(500);

    const searchPage = await waitForElement('[data-testid="search-page"]', 'Search page');
    await expect(searchPage).toBeDisplayed();
  });

  it('should navigate to home with Ctrl+H', async () => {
    // First go somewhere else
    const libraryTab = await waitForElement('[data-testid="nav-library"]', 'Library tab');
    await libraryTab.waitForClickable({ timeout: 5000 });
    await libraryTab.click();
    await browser.pause(500);

    // Press Ctrl+H
    await browser.keys(['Control', 'h']);
    await browser.pause(500);

    const homePage = await waitForElement('[data-testid="home-page"]', 'Home page');
    await expect(homePage).toBeDisplayed();
  });

  it('should navigate to library with Ctrl+L', async () => {
    // Press Ctrl+L
    await browser.keys(['Control', 'l']);
    await browser.pause(500);

    const libraryPage = await waitForElement('[data-testid="library-page"]', 'Library page');
    await expect(libraryPage).toBeDisplayed();
  });
});
