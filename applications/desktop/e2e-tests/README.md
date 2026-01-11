# Soul Player E2E Tests

End-to-end tests for the Soul Player desktop application using WebdriverIO and tauri-driver.

## Current Status: PARTIALLY FUNCTIONAL

**Status Update**: DSP effect components have data-testid attributes implemented. See `TEST_IDS.md` for details on which components are ready.

**Before tests can run:**

1. **data-testid attributes must be added to UI components** - See `TEST_IDS.md` for the complete list of required test IDs. DSP effect components are implemented; navigation and other components still need IDs.

2. **The app must be built in release mode** - Tests run against the compiled binary.

3. **tauri-driver must be installed and working** - This is the WebDriver bridge for Tauri.

## Prerequisites

### Required Tools

1. **Rust and Cargo** - For building the app and tauri-driver
2. **tauri-driver** - WebDriver server for Tauri apps
   ```bash
   cargo install tauri-driver
   ```
3. **Node.js 18+** - For running WebdriverIO tests
4. **Edge WebDriver (Windows)** - msedgedriver must be in PATH
5. **WebKitWebDriver (Linux)** - Part of webkit2gtk package (usually at `/usr/lib/webkit2gtk-4.0/WebKitWebDriver`)

### Platform-Specific Requirements

**Windows:**
- Microsoft Edge (included with Windows 10/11)
- msedgedriver should be automatically available

**Linux:**
- webkit2gtk-4.0 development libraries
- WebKitWebDriver (usually `/usr/lib/webkit2gtk-4.0/WebKitWebDriver`)

**macOS:**
- WebKit is included with Safari
- May need to enable developer mode for WebDriver

## Setup

1. **Build the desktop app:**
   ```bash
   cd applications/desktop/src-tauri
   cargo build --release
   ```

2. **Install test dependencies:**
   ```bash
   cd applications/desktop/e2e-tests
   npm install
   # or
   yarn install
   ```

## Running Tests

### Run all tests:
```bash
npm test
# or
yarn test
```

### Run specific test file:
```bash
# Run only DSP effects tests
npm test -- --spec tests/specs/dsp-effects.e2e.js

# Run only navigation tests
npm test -- --spec tests/specs/navigation.e2e.js

# Run only audio settings tests
npm test -- --spec tests/specs/audio-settings.e2e.js
```

### Run tests in CI mode (headless):
```bash
npm run test:ci
# or
yarn test:ci
```

### Run tests with increased timeout:
```bash
# Useful for slow machines or debugging
npm test -- --mochaOpts.timeout 120000
```

## Test Structure

```
e2e-tests/
├── package.json              # Dependencies and scripts
├── wdio.conf.js              # WebdriverIO configuration
├── README.md                 # This file
├── TEST_IDS.md               # Required data-testid attributes
├── screenshots/              # Failure screenshots (auto-generated)
└── tests/
    └── specs/
        ├── audio-settings.e2e.js   # Audio settings tests
        ├── dsp-effects.e2e.js      # DSP effect workflows
        └── navigation.e2e.js       # Navigation tests
```

## Test Suites

### DSP Effects Tests (`dsp-effects.e2e.js`)

Comprehensive tests for the DSP effects chain configuration:

| Test Suite | Description |
|------------|-------------|
| **Add Effect Workflow** | Tests adding each effect type (Compressor, EQ, Limiter, Crossfeed, Stereo Enhancer, Graphic EQ) |
| **Edit Effect Workflow** | Tests opening editors, displaying controls, modifying parameters |
| **Remove Effect Workflow** | Tests removing effects and verifying empty slots |
| **Effect Chain Order** | Tests adding effects to multiple slots and maintaining order |
| **Presets** | Tests preset dropdown and applying presets |
| **Clear All** | Tests clearing all effects from the chain |
| **Enable/Disable Toggle** | Tests toggling effects on/off without removing |
| **Persistence** | Tests that effects persist after navigation and app reload |
| **Error Handling** | Tests graceful handling of edge cases |
| **Full Workflow Integration** | End-to-end workflow combining multiple operations |

### Audio Settings Tests (`audio-settings.e2e.js`)

Tests for audio settings including volume leveling, resampling, and buffer settings.

### Navigation Tests (`navigation.e2e.js`)

Tests for app navigation, settings tabs, and keyboard shortcuts.

## Writing Tests

Tests use WebdriverIO's API with Mocha as the test framework.

### Example Test

```javascript
describe('My Feature', () => {
  it('should do something', async () => {
    // Find element by data-testid
    const button = await $('[data-testid="my-button"]');

    // Wait for it to be clickable
    await button.waitForClickable({ timeout: 5000 });

    // Interact with it
    await button.click();

    // Assert on result
    const result = await $('[data-testid="result"]');
    await expect(result).toBeDisplayed();
  });
});
```

### Helper Functions

The DSP tests include reusable helper functions:

```javascript
// Navigate to DSP configuration
await navigateToDspConfig();

// Add an effect to a slot
await addEffect(0, 'Compressor');

// Remove an effect from a slot
await removeEffect(0);

// Open the effect editor
await openEffectEditor(0);

// Check if a slot has an effect
const hasEffect = await slotHasEffect(0);

// Clear all effects
await clearAllEffects();
```

### Best Practices

1. **Use data-testid selectors** - More stable than CSS classes or text content
2. **Wait for elements** - Use `waitForClickable`, `waitForDisplayed`, etc.
3. **Add pauses after navigation** - Give the app time to render
4. **Handle conditional UI** - Check if elements exist before interacting
5. **Clean up state** - Use `clearAllEffects()` in `beforeEach` when needed
6. **Use descriptive test names** - Make it clear what the test verifies

## Test IDs

See `TEST_IDS.md` for the complete list of required `data-testid` attributes that need to be added to UI components.

### DSP Effect Test IDs (IMPLEMENTED)

| Test ID | Component |
|---------|-----------|
| `dsp-config` | Main DSP config container |
| `effect-slot-{index}` | Effect slot container (0-3) |
| `add-effect-btn-{index}` | Add effect button for each slot |
| `edit-effect-btn-{index}` | Edit button for each slot |
| `remove-effect-btn-{index}` | Remove button for each slot |
| `effect-picker-{index}` | Effect type picker dropdown |
| `clear-all-btn` | Clear all effects button |
| `compressor-editor` | Compressor editor container |
| `compressor-threshold` | Threshold slider |
| `compressor-ratio` | Ratio slider |
| `compressor-preset-select` | Preset dropdown |
| `limiter-editor` | Limiter editor container |
| `parametric-eq-editor` | Parametric EQ editor |
| `graphic-eq-editor` | Graphic EQ editor |
| `crossfeed-editor` | Crossfeed editor |
| `stereo-editor` | Stereo enhancer editor |

## Troubleshooting

### tauri-driver not found
Make sure it's in your PATH:
```bash
cargo install tauri-driver
```

### App not starting
Ensure the app is built:
```bash
cd applications/desktop/src-tauri
cargo build --release
```

### Tests timing out
- Increase `waitforTimeout` in `wdio.conf.js`
- Add `await browser.pause(milliseconds)` after navigation
- Run with `--mochaOpts.timeout 120000` for slow machines

### Element not found errors
- Check that the data-testid attribute is added to the component
- Verify the element is rendered (not hidden or conditionally removed)
- Try adding a `browser.pause()` before interacting

### Screenshots on failure
Screenshots are automatically saved to `./screenshots/` when tests fail.

### DSP tests failing intermittently
- The DSP chain may have residual effects from previous test runs
- Ensure `clearAllEffects()` is called in `beforeEach` hooks
- Increase wait times with `browser.pause()`

## CI Integration

For CI environments:

1. Build the app in release mode
2. Install tauri-driver
3. On Linux: Set up a virtual display (Xvfb) - headless mode is not fully supported
4. Run `npm run test:ci`

Example GitHub Actions step (Linux):
```yaml
- name: Run E2E Tests
  run: |
    # Install dependencies
    cargo install tauri-driver

    # Build the app (from workspace root)
    cargo build --release -p soul-player-desktop

    # Set up virtual display (Linux only)
    export DISPLAY=:99
    Xvfb :99 -screen 0 1920x1080x24 &
    sleep 2

    # Run tests
    cd applications/desktop/e2e-tests
    npm ci
    npm test
```

Example GitHub Actions step (Windows):
```yaml
- name: Run E2E Tests
  run: |
    cargo install tauri-driver
    cargo build --release -p soul-player-desktop
    cd applications/desktop/e2e-tests
    npm ci
    npm test
```

**Note**: Full E2E test suite requires all UI components to have data-testid attributes. DSP effects tests are ready; navigation tests require additional implementation.

## Limitations

- **Single instance**: Tauri apps can only run one instance at a time
- **Platform differences**: Some WebDriver features may vary between platforms
- **Headless mode**: May not be fully supported on all platforms
- **Native dialogs**: File dialogs and system dialogs cannot be automated
- **Backend state**: Tests may be affected by backend state from previous runs

## Related Documentation

- [WebdriverIO Docs](https://webdriver.io/docs/gettingstarted)
- [Tauri Testing Guide](https://tauri.app/v1/guides/testing/webdriver/)
- [Mocha Framework](https://mochajs.org/)
- [Soul Player TEST_IDS.md](./TEST_IDS.md)
