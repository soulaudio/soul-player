/**
 * WebdriverIO configuration for Lazy Queue E2E Test
 *
 * This config:
 * 1. Creates an isolated test database
 * 2. Seeds it with 500 test tracks using better-sqlite3
 * 3. Launches the app with DATABASE_PATH pointing to test DB
 * 4. Runs automated UI tests
 * 5. Cleans up test database
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.lazy-queue.conf.js
 */

import { config as baseConfig } from './wdio.conf.js';
import { mkdirSync, rmSync, readdirSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { fileURLToPath } from 'url';
import Database from 'better-sqlite3';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Global test database state
let testDbPath = null;
let testDbDir = null;

/**
 * Create isolated test database and seed with 500 tracks
 */
function setupTestDatabase() {
  console.log('[E2E Setup] Creating isolated test database...');

  // Create temp directory for test database
  const timestamp = Date.now();
  testDbDir = join(tmpdir(), `soul-player-e2e-${timestamp}`);
  mkdirSync(testDbDir, { recursive: true });

  testDbPath = join(testDbDir, 'test.db');
  console.log(`[E2E Setup] Test database: ${testDbPath}`);

  // Get migrations directory
  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');

  // Create database and run migrations
  const db = new Database(testDbPath);

  try {
    console.log('[E2E Setup] Running migrations...');

    // Read and execute migration files
    const migrationFiles = readdirSync(migrationsDir)
      .filter(f => f.endsWith('.sql'))
      .sort();

    for (const file of migrationFiles) {
      const sql = readFileSync(join(migrationsDir, file), 'utf-8');
      console.log(`[E2E Setup] Running migration: ${file}`);
      db.exec(sql);
    }

    console.log('[E2E Setup] ✓ Migrations complete');

    // Create default user
    console.log('[E2E Setup] Creating default user...');
    const now = Math.floor(Date.now() / 1000);
    db.prepare('INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)')
      .run('1', 'Test User', now);

    // Seed test data
    console.log('[E2E Setup] Seeding 500 test tracks...');

    // Insert test artist
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)')
      .run(9999, 'E2E Test Artist');

    // Insert test album
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)')
      .run(9999, 'E2E Test Album', 9999, 2024);

    // Insert local source
    db.prepare('INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)')
      .run(1, 'Local', 'local');

    // Prepare insert statements
    const insertTrack = db.prepare(`
      INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const insertAvailability = db.prepare(`
      INSERT INTO track_sources (track_id, source_id, status, local_file_path)
      VALUES (?, ?, ?, ?)
    `);

    // Insert 500 tracks in a transaction
    const insertMany = db.transaction((count) => {
      for (let i = 1; i <= count; i++) {
        const trackId = 10000 + i;
        const title = `E2E Test Track ${i}`;
        const filePath = `test/track_${i}.mp3`;

        insertTrack.run(trackId, title, 9999, 9999, i, 1, 180.0, 'mp3');
        insertAvailability.run(trackId, 1, 'available', filePath);

        if (i % 100 === 0) {
          console.log(`[E2E Setup] Seeded ${i}/500 tracks...`);
        }
      }
    });

    insertMany(500);

    // Verify
    const count = db.prepare('SELECT COUNT(*) as count FROM tracks WHERE id >= 10000').get().count;
    console.log(`[E2E Setup] ✓ Seeded ${count} test tracks`);

    if (count !== 500) {
      throw new Error(`Expected 500 tracks, got ${count}`);
    }

  } finally {
    db.close();
  }

  console.log('[E2E Setup] ✓ Test database ready');
  return testDbPath;
}

/**
 * Clean up test database
 */
function cleanupTestDatabase() {
  if (testDbDir) {
    console.log('[E2E Cleanup] Removing test database...');
    try {
      rmSync(testDbDir, { recursive: true, force: true });
      console.log('[E2E Cleanup] ✓ Test database removed');
    } catch (error) {
      console.error('[E2E Cleanup] Failed to remove test database:', error);
    }
  }
}

// Extend base config
export const config = {
  ...baseConfig,

  // Override specs to run only lazy queue test
  specs: [
    './tests/specs/lazy-queue.e2e.js'
  ],

  /**
   * Gets executed before test session starts
   */
  onPrepare: async function (config, capabilities) {
    // Setup test database FIRST
    testDbPath = setupTestDatabase();

    // Set DATABASE_PATH as global environment variable
    // The app launched by tauri-driver will inherit this
    process.env.DATABASE_PATH = testDbPath;

    console.log('[E2E] Test database path configured:', testDbPath);
    console.log('[E2E] Set DATABASE_PATH environment variable');

    // Run base onPrepare (starts tauri-driver)
    if (baseConfig.onPrepare) {
      await baseConfig.onPrepare(config, capabilities);
    }

    console.log('[E2E] Test environment ready');
  },

  /**
   * Gets executed before test execution begins (modifies capabilities)
   */
  before: async function (capabilities, specs, browser) {
    console.log('[E2E] Starting tests with isolated database');
    console.log(`[E2E] Test database: ${testDbPath}`);

    // Give app extra time to initialize with test database
    await browser.pause(3000);
  },

  /**
   * Gets executed after all tests are done
   */
  onComplete: async function (exitCode, config, capabilities, results) {
    // Clean up test database
    cleanupTestDatabase();

    // Run base onComplete (stops tauri-driver)
    if (baseConfig.onComplete) {
      await baseConfig.onComplete(exitCode, config, capabilities, results);
    }
  },

  /**
   * Gets executed before test execution begins
   */
  before: async function (capabilities, specs) {
    console.log('[E2E] Starting tests with isolated database');
    console.log(`[E2E] Test database: ${testDbPath}`);

    // Give app extra time to initialize with test database
    await browser.pause(3000);
  },

  /**
   * Gets executed after each test
   */
  afterTest: async function (test, context, { error, result, duration, passed, retries }) {
    if (error) {
      console.log('[E2E] Test failed:', test.title);
      console.log('[E2E] Error:', error.message);

      // Take screenshot
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const screenshotPath = `./screenshots/lazy-queue-${timestamp}.png`;
      try {
        await browser.saveScreenshot(screenshotPath);
        console.log(`[E2E] Screenshot saved: ${screenshotPath}`);
      } catch (screenshotError) {
        console.error('[E2E] Failed to save screenshot:', screenshotError);
      }
    }
  },
};
