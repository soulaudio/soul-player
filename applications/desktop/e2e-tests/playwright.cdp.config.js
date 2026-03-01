/**
 * Playwright CDP config for Soul Player desktop E2E tests.
 *
 * Connects to the running Tauri app via Chrome DevTools Protocol (CDP)
 * through Edge WebView2's remote debugging port.
 *
 * Prerequisites:
 *   1. Build release binary: cargo build --release -p soul-player-desktop
 *   2. Install Playwright: npm run playwright:install
 *
 * Run: npm run test:playwright
 */

import { defineConfig } from '@playwright/test';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  testDir: './tests/playwright',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 60_000,
  reporter: [['list'], ['html', { open: 'never' }]],
  globalSetup: join(__dirname, 'playwright-global-setup.js'),
  globalTeardown: join(__dirname, 'playwright-global-teardown.js'),
});
