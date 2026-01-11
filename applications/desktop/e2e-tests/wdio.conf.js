/**
 * WebdriverIO configuration for Soul Player E2E tests with tauri-driver
 *
 * IMPORTANT: Before tests can run, you MUST add data-testid attributes to
 * the UI components. See TEST_IDS.md for the full list of required test IDs.
 *
 * Prerequisites:
 * 1. Build the app: cargo build --release (from applications/desktop/src-tauri)
 * 2. Install tauri-driver: cargo install tauri-driver
 * 3. Windows: Edge WebDriver (msedgedriver) must be in PATH
 * 4. Linux: WebKitWebDriver available (webkit2gtk)
 * 5. Add data-testid attributes to UI components (see TEST_IDS.md)
 *
 * Run tests: npm test (or yarn test)
 */

import { spawn, execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Determine the built application path based on platform
// Note: The binary name comes from Cargo.toml package name (soul-player-desktop)
function getAppPath() {
  // For Tauri 2.x, the target directory is at the workspace root, not in src-tauri
  const workspaceRoot = join(__dirname, '..', '..', '..');
  const tauriDir = join(__dirname, '..', 'src-tauri');

  // Check both possible locations (workspace root and src-tauri)
  const possibleTargetDirs = [
    join(workspaceRoot, 'target', 'release'),
    join(tauriDir, 'target', 'release'),
  ];

  for (const targetDir of possibleTargetDirs) {
    let appPath;
    if (process.platform === 'win32') {
      appPath = join(targetDir, 'soul-player-desktop.exe');
    } else if (process.platform === 'darwin') {
      // On macOS, check both the direct binary and the .app bundle
      appPath = join(targetDir, 'bundle', 'macos', 'Soul Player.app', 'Contents', 'MacOS', 'Soul Player');
      if (!existsSync(appPath)) {
        appPath = join(targetDir, 'soul-player-desktop');
      }
    } else {
      appPath = join(targetDir, 'soul-player-desktop');
    }

    if (existsSync(appPath)) {
      console.log(`Found app at: ${appPath}`);
      return appPath;
    }
  }

  // Fallback to workspace root (most common for Cargo workspaces)
  const fallbackDir = join(workspaceRoot, 'target', 'release');
  if (process.platform === 'win32') {
    return join(fallbackDir, 'soul-player-desktop.exe');
  } else if (process.platform === 'darwin') {
    return join(fallbackDir, 'bundle', 'macos', 'Soul Player.app', 'Contents', 'MacOS', 'Soul Player');
  } else {
    return join(fallbackDir, 'soul-player-desktop');
  }
}

// Store tauri-driver process reference
let tauriDriver;

export const config = {
  //
  // ====================
  // Runner Configuration
  // ====================
  //
  runner: 'local',

  //
  // ==================
  // Specify Test Files
  // ==================
  //
  specs: [
    './tests/specs/**/*.e2e.js'
  ],

  // Exclude test patterns
  exclude: [],

  //
  // ============
  // Capabilities
  // ============
  //
  maxInstances: 1, // Tauri apps need single instance

  capabilities: [{
    // Use 'wry' as the browser name for tauri-driver
    browserName: 'wry',
    'tauri:options': {
      application: getAppPath(),
    },
    // Note: Headless mode for Tauri/wry is limited.
    // On Linux/CI, you may need a virtual display (Xvfb) instead.
    // The --headless flag is kept for future compatibility but may not work on all platforms.
  }],

  //
  // ===================
  // Test Configurations
  // ===================
  //
  logLevel: 'info',
  bail: 0,
  baseUrl: '',
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  // Use tauri-driver as the WebDriver server
  port: 4444,
  hostname: 'localhost',
  path: '/',

  //
  // ==============
  // Framework
  // ==============
  //
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  //
  // ==============
  // Reporters
  // ==============
  //
  reporters: ['spec'],

  //
  // =====
  // Hooks
  // =====
  //

  /**
   * Start tauri-driver before test session
   */
  onPrepare: async function () {
    console.log('Starting tauri-driver...');

    // Verify tauri-driver is installed
    try {
      const driverPath = process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver';
      execSync(`${driverPath} --version`, { stdio: 'pipe' });
    } catch {
      throw new Error(
        'tauri-driver not found. Install it with: cargo install tauri-driver'
      );
    }

    // Verify the app exists
    const appPath = getAppPath();
    if (!existsSync(appPath)) {
      throw new Error(
        `Application not found at: ${appPath}\n` +
        'Build the app first with: cargo build --release -p soul-player-desktop'
      );
    }

    // Start tauri-driver
    tauriDriver = spawn(
      process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver',
      ['--port', '4444'],
      {
        stdio: ['ignore', 'pipe', 'pipe'],
        shell: process.platform === 'win32',
      }
    );

    tauriDriver.stdout.on('data', (data) => {
      console.log(`[tauri-driver] ${data}`);
    });

    tauriDriver.stderr.on('data', (data) => {
      console.error(`[tauri-driver] ${data}`);
    });

    // Handle tauri-driver exit
    tauriDriver.on('exit', (code) => {
      if (code !== 0 && code !== null) {
        console.error(`tauri-driver exited with code ${code}`);
      }
    });

    // Wait for tauri-driver to be ready (with readiness check)
    let ready = false;
    for (let i = 0; i < 10; i++) {
      await new Promise((resolve) => setTimeout(resolve, 500));
      try {
        // Try to connect to tauri-driver
        const response = await fetch('http://localhost:4444/status');
        if (response.ok) {
          ready = true;
          break;
        }
      } catch {
        // Not ready yet, keep waiting
      }
    }

    if (!ready) {
      console.warn('tauri-driver may not be ready, proceeding anyway...');
    }

    console.log('tauri-driver started');
  },

  /**
   * Stop tauri-driver after test session
   */
  onComplete: async function () {
    console.log('Stopping tauri-driver...');

    if (tauriDriver) {
      tauriDriver.kill();
      tauriDriver = null;
    }

    console.log('tauri-driver stopped');
  },

  /**
   * Before each test file
   */
  beforeSession: async function () {
    // Wait a bit for the app to initialize
    await new Promise((resolve) => setTimeout(resolve, 1000));
  },

  /**
   * After each test (useful for screenshots on failure)
   */
  afterTest: async function (test, context, { error }) {
    if (error) {
      // Take a screenshot on failure
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const screenshotPath = `./screenshots/${test.title}-${timestamp}.png`;
      try {
        await browser.saveScreenshot(screenshotPath);
        console.log(`Screenshot saved: ${screenshotPath}`);
      } catch (screenshotError) {
        console.error('Failed to save screenshot:', screenshotError);
      }
    }
  },
};
