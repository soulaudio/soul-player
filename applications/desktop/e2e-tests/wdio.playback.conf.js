/**
 * WebdriverIO configuration for Playback E2E Tests
 *
 * This config:
 * 1. Generates tiny silent WAV files so the audio engine can actually load tracks
 * 2. Creates an isolated test database seeded with 2 albums × 5 tracks each
 * 3. Launches the app pointing at the test database
 * 4. Runs the playback test specs
 * 5. Cleans up after tests
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.playback.conf.js
 */

import { config as baseConfig } from './wdio.conf.js';
import { mkdirSync, rmSync, readdirSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { fileURLToPath } from 'url';
import Database from 'better-sqlite3';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Global test state
let testDbPath = null;
let testDbDir = null;
let testAudioDir = null;

/**
 * Create a minimal silent WAV file (1 second, 44100 Hz, 16-bit, mono).
 * Returns a Buffer containing the full WAV file.
 */
function createSilentWavBuffer(durationSeconds = 1) {
  const sampleRate = 44100;
  const channels = 1;
  const bitsPerSample = 16;
  const numSamples = Math.floor(sampleRate * durationSeconds * channels);
  const dataSize = numSamples * (bitsPerSample / 8);
  const fileSize = 36 + dataSize; // RIFF header total without the initial "RIFF" + size fields

  // Total buffer: 4 (RIFF) + 4 (fileSize) + 4 (WAVE) + 24 (fmt chunk) + 8 (data header) + dataSize
  const buffer = Buffer.alloc(44 + dataSize, 0);
  let offset = 0;

  // RIFF chunk descriptor
  buffer.write('RIFF', offset); offset += 4;
  buffer.writeUInt32LE(fileSize, offset); offset += 4;
  buffer.write('WAVE', offset); offset += 4;

  // fmt sub-chunk
  buffer.write('fmt ', offset); offset += 4;
  buffer.writeUInt32LE(16, offset); offset += 4;           // Subchunk1Size (16 for PCM)
  buffer.writeUInt16LE(1, offset); offset += 2;            // AudioFormat = PCM
  buffer.writeUInt16LE(channels, offset); offset += 2;     // NumChannels
  buffer.writeUInt32LE(sampleRate, offset); offset += 4;   // SampleRate
  buffer.writeUInt32LE(sampleRate * channels * (bitsPerSample / 8), offset); offset += 4; // ByteRate
  buffer.writeUInt16LE(channels * (bitsPerSample / 8), offset); offset += 2; // BlockAlign
  buffer.writeUInt16LE(bitsPerSample, offset); offset += 2; // BitsPerSample

  // data sub-chunk
  buffer.write('data', offset); offset += 4;
  buffer.writeUInt32LE(dataSize, offset); offset += 4;
  // Remaining bytes are already 0 (silence)

  return buffer;
}

/**
 * Setup: create test database seeded with 2 albums × 5 tracks, plus real WAV files.
 */
function setupTestDatabase() {
  console.log('[Playback E2E Setup] Creating isolated test environment...');

  const timestamp = Date.now();
  testDbDir = join(tmpdir(), `soul-player-playback-e2e-${timestamp}`);
  testAudioDir = join(testDbDir, 'audio');
  mkdirSync(testAudioDir, { recursive: true });

  // Generate one shared WAV file (all test tracks will point to the same file)
  const wavPath = join(testAudioDir, 'test-track.wav');
  writeFileSync(wavPath, createSilentWavBuffer(2)); // 2 seconds of silence
  console.log(`[Playback E2E Setup] Created WAV test file: ${wavPath}`);

  testDbPath = join(testDbDir, 'test.db');
  console.log(`[Playback E2E Setup] Test database: ${testDbPath}`);

  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');
  const db = new Database(testDbPath);

  try {
    console.log('[Playback E2E Setup] Running migrations...');
    const migrationFiles = readdirSync(migrationsDir)
      .filter(f => f.endsWith('.sql'))
      .sort();

    for (const file of migrationFiles) {
      const sql = readFileSync(join(migrationsDir, file), 'utf-8');
      db.exec(sql);
    }
    console.log('[Playback E2E Setup] ✓ Migrations complete');

    // Create default user
    const now = Math.floor(Date.now() / 1000);
    db.prepare('INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)').run('1', 'Test User', now);

    // Insert local source
    db.prepare('INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)').run(1, 'Local', 'local');

    // ---- Album 1: "Rock Classics" ----
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(1001, 'E2E Rock Band');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(1001, 'Rock Classics', 1001, 2020);

    const trackTitles1 = ['Highway Blues', 'Thunder Road', 'Midnight Run', 'Electric Storm', 'Final Countdown'];
    trackTitles1.forEach((title, i) => {
      const trackId = 1001 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(trackId, title, 1001, 1001, i + 1, 1, 2.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(trackId, 1, 'available', wavPath);
    });

    // ---- Album 2: "Jazz Night" ----
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(1002, 'E2E Jazz Quartet');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(1002, 'Jazz Night', 1002, 2021);

    const trackTitles2 = ['Blue Moon Serenade', 'Autumn Leaves', 'Smooth Operator', 'Night Groove', 'Dawn Chorus'];
    trackTitles2.forEach((title, i) => {
      const trackId = 1011 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(trackId, title, 1002, 1002, i + 1, 1, 2.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(trackId, 1, 'available', wavPath);
    });

    const trackCount = db.prepare('SELECT COUNT(*) as count FROM tracks').get().count;
    console.log(`[Playback E2E Setup] ✓ Seeded ${trackCount} tracks across 2 albums`);

  } finally {
    db.close();
  }

  console.log('[Playback E2E Setup] ✓ Test environment ready');
  return testDbPath;
}

/**
 * Cleanup: remove temp test files.
 */
function cleanupTestDatabase() {
  if (testDbDir) {
    console.log('[Playback E2E Cleanup] Removing test environment...');
    try {
      rmSync(testDbDir, { recursive: true, force: true });
      console.log('[Playback E2E Cleanup] ✓ Removed');
    } catch (error) {
      console.error('[Playback E2E Cleanup] Failed to remove:', error);
    }
  }
}

export const config = {
  ...baseConfig,

  // Run only the playback spec
  specs: ['./tests/specs/playback.e2e.js'],

  // Longer timeout for audio loading
  mochaOpts: {
    ...baseConfig.mochaOpts,
    timeout: 90000,
  },

  onPrepare: async function (config, capabilities) {
    testDbPath = setupTestDatabase();
    process.env.DATABASE_PATH = testDbPath;
    console.log('[Playback E2E] DATABASE_PATH:', testDbPath);

    if (baseConfig.onPrepare) {
      await baseConfig.onPrepare(config, capabilities);
    }
  },

  before: async function (capabilities, specs) {
    console.log('[Playback E2E] Starting tests with isolated database');
    await browser.pause(4000); // Extra time for app to initialize
  },

  onComplete: async function (exitCode, config, capabilities, results) {
    cleanupTestDatabase();
    if (baseConfig.onComplete) {
      await baseConfig.onComplete(exitCode, config, capabilities, results);
    }
  },

  afterTest: async function (test, context, { error }) {
    if (error) {
      console.log('[Playback E2E] Test failed:', test.title);
      console.log('[Playback E2E] Error:', error.message);
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const screenshotPath = `./screenshots/playback-${timestamp}.png`;
      try {
        await browser.saveScreenshot(screenshotPath);
        console.log(`[Playback E2E] Screenshot: ${screenshotPath}`);
      } catch (e) {
        console.error('[Playback E2E] Screenshot failed:', e);
      }
    }
  },
};
