/**
 * Playwright config for production library screenshots.
 * Uses the real production database — no seeded test data.
 *
 * Run: npx playwright test --config playwright.prod.config.js
 */

import { defineConfig } from '@playwright/test';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  testDir: './tests/playwright',
  testMatch: '{prod-library-screenshots,dsd-audio-quality}.spec.js',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 240_000,
  reporter: [['list'], ['html', { open: 'never' }]],
  globalSetup: join(__dirname, 'playwright-prod-setup.js'),
  globalTeardown: join(__dirname, 'playwright-prod-teardown.js'),
});
