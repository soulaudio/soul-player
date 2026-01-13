/**
 * E2E Test: Lazy Queue Loading
 *
 * Tests that the queue automatically loads more tracks as needed when:
 * 1. Playing through tracks (forward pagination)
 * 2. Jumping to last track in queue (jump loading)
 *
 * Prerequisites:
 * - Run: yarn seed-test-data seed
 * - This creates 500 test tracks in the database
 */

import { test, expect, _electron as electron, type ElectronApplication, type Page } from '@playwright/test';
import * as path from 'path';

let electronApp: ElectronApplication;
let page: Page;

const LIBRARY_PATH = path.join(__dirname, '../../../../');
const TOTAL_TEST_TRACKS = 500;
const INITIAL_BATCH_SIZE = 50;

test.beforeAll(async () => {
  // Launch Electron app
  electronApp = await electron.launch({
    args: [
      path.join(__dirname, '../../src-tauri/target/debug/soul-player-desktop'),
    ],
    env: {
      ...process.env,
      DATABASE_URL: 'sqlite:libraries/soul-storage/.tmp/dev.db',
    },
  });

  // Get the first window
  page = await electronApp.firstWindow();

  // Wait for app to be ready
  await page.waitForSelector('[data-testid="library-page"]', { timeout: 30000 });
});

test.afterAll(async () => {
  await electronApp.close();
});

test.describe('Lazy Queue Loading', () => {
  test('should load initial batch of 50 tracks when playing from large library', async () => {
    // Navigate to Tracks page
    await page.click('[data-testid="nav-tracks"]');
    await page.waitForSelector('[data-testid="track-list"]');

    // Verify we have 500+ tracks
    const trackCountText = await page.textContent('[data-testid="track-count"]');
    const totalTracks = parseInt(trackCountText?.match(/\d+/)?.[0] || '0');
    expect(totalTracks).toBeGreaterThanOrEqual(TOTAL_TEST_TRACKS);

    // Play first test track
    await page.click('[data-testid="track-row"]:has-text("Test Track 1")');

    // Wait for playback to start
    await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 5000 });

    // Open queue sidebar
    await page.click('[data-testid="queue-button"]');
    await page.waitForSelector('[data-testid="queue-sidebar"]');

    // Count queue items (should be 50 initially)
    const queueItems = await page.locator('[data-testid="queue-item"]').count();
    console.log(`[Test] Initial queue size: ${queueItems}`);
    expect(queueItems).toBe(INITIAL_BATCH_SIZE);
  });

  test('should load next batch when clicking last track in queue', async () => {
    // Ensure queue is open
    const queueVisible = await page.isVisible('[data-testid="queue-sidebar"]');
    if (!queueVisible) {
      await page.click('[data-testid="queue-button"]');
      await page.waitForSelector('[data-testid="queue-sidebar"]');
    }

    // Get initial queue size
    let queueSize = await page.locator('[data-testid="queue-item"]').count();
    console.log(`[Test] Queue size before clicking last track: ${queueSize}`);
    expect(queueSize).toBe(INITIAL_BATCH_SIZE);

    // Scroll to bottom of queue
    await page.locator('[data-testid="queue-sidebar"]').evaluate((el) => {
      el.scrollTop = el.scrollHeight;
    });

    // Click last track in queue (should trigger batch loading)
    await page.click('[data-testid="queue-item"]:last-child');

    // Wait for batch loading (give it 3 seconds)
    await page.waitForTimeout(3000);

    // Verify queue grew
    queueSize = await page.locator('[data-testid="queue-item"]').count();
    console.log(`[Test] Queue size after clicking last track: ${queueSize}`);

    // Queue should have loaded next batch (100 total)
    expect(queueSize).toBeGreaterThan(INITIAL_BATCH_SIZE);
    expect(queueSize).toBeLessThanOrEqual(INITIAL_BATCH_SIZE * 2);
  });

  test('should continue loading batches as user navigates through queue', async () => {
    // Skip to track 45 (near end of second batch)
    await page.evaluate(() => {
      (window as any).__testHelpers.skipToQueueIndex(94);
    });

    await page.waitForTimeout(2000);

    // Verify another batch was loaded
    const queueSize = await page.locator('[data-testid="queue-item"]').count();
    console.log(`[Test] Queue size after skipping to track 95: ${queueSize}`);

    // Should have 3 batches (150 tracks)
    expect(queueSize).toBeGreaterThanOrEqual(140);
  });

  test('should handle jumping far beyond loaded window', async () => {
    // Jump to track 250 (way beyond current window)
    await page.evaluate(() => {
      (window as any).__testHelpers.skipToQueueIndex(249);
    });

    await page.waitForTimeout(3000);

    // Verify track is playing
    const nowPlayingTitle = await page.textContent('[data-testid="now-playing-title"]');
    console.log(`[Test] Now playing: ${nowPlayingTitle}`);
    expect(nowPlayingTitle).toContain('Test Track 250');

    // Verify queue has tracks around position 250
    const queueSize = await page.locator('[data-testid="queue-item"]').count();
    console.log(`[Test] Queue size after jumping to track 250: ${queueSize}`);
    expect(queueSize).toBeGreaterThan(0);
  });

  test('should not crash when reaching end of library', async () => {
    // Jump to last track
    await page.evaluate(() => {
      (window as any).__testHelpers.skipToQueueIndex(499);
    });

    await page.waitForTimeout(3000);

    // Verify we're playing the last track
    const nowPlayingTitle = await page.textContent('[data-testid="now-playing-title"]');
    console.log(`[Test] Now playing (should be last track): ${nowPlayingTitle}`);
    expect(nowPlayingTitle).toContain('Test Track 500');

    // Try to skip next (should either stop or loop depending on repeat mode)
    await page.click('[data-testid="next-button"]');
    await page.waitForTimeout(1000);

    // App should not have crashed
    const isAppResponsive = await page.isVisible('[data-testid="library-page"]');
    expect(isAppResponsive).toBe(true);
  });
});

test.describe('Queue UI Updates', () => {
  test('should display queue items as they load', async () => {
    // Navigate to Tracks page
    await page.click('[data-testid="nav-tracks"]');
    await page.waitForSelector('[data-testid="track-list"]');

    // Play first track
    await page.click('[data-testid="track-row"]:has-text("Test Track 1")');
    await page.waitForSelector('[data-testid="now-playing-title"]');

    // Open queue
    await page.click('[data-testid="queue-button"]');

    // Verify initial queue
    let queueItems = await page.locator('[data-testid="queue-item"]').all();
    expect(queueItems.length).toBe(50);

    // Click track 48 (triggers batch load)
    await page.click('[data-testid="queue-item"]').nth(47);
    await page.waitForTimeout(2000);

    // Verify queue grew
    queueItems = await page.locator('[data-testid="queue-item"]').all();
    expect(queueItems.length).toBeGreaterThan(50);
  });
});
