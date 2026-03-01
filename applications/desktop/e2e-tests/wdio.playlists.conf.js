/**
 * WebdriverIO configuration for Playlists E2E Tests
 *
 * This config:
 * 1. Generates a tiny silent WAV file so the audio engine can actually load tracks
 * 2. Creates an isolated test database seeded with 1 artist, 1 album, 5 tracks, and 1 playlist
 * 3. Launches the app pointing at the test database
 * 4. Runs the playlists test specs
 * 5. Cleans up after tests
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.playlists.conf.js
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
 * Setup: create test database seeded with 1 album × 5 tracks and 1 pre-seeded playlist.
 */
function setupTestDatabase() {
  console.log('[Playlists E2E Setup] Creating isolated test environment...');

  const timestamp = Date.now();
  testDbDir = join(tmpdir(), `soul-player-playlists-e2e-${timestamp}`);
  testAudioDir = join(testDbDir, 'audio');
  mkdirSync(testAudioDir, { recursive: true });

  // Generate one shared WAV file (all test tracks will point to the same file)
  const wavPath = join(testAudioDir, 'test-track.wav');
  writeFileSync(wavPath, createSilentWavBuffer(2)); // 2 seconds of silence
  console.log(`[Playlists E2E Setup] Created WAV test file: ${wavPath}`);

  testDbPath = join(testDbDir, 'test.db');
  console.log(`[Playlists E2E Setup] Test database: ${testDbPath}`);

  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');
  const db = new Database(testDbPath);

  try {
    console.log('[Playlists E2E Setup] Running migrations...');
    const migrationFiles = readdirSync(migrationsDir)
      .filter(f => f.endsWith('.sql'))
      .sort();

    for (const file of migrationFiles) {
      const sql = readFileSync(join(migrationsDir, file), 'utf-8');
      db.exec(sql);
    }
    console.log('[Playlists E2E Setup] ✓ Migrations complete');

    // Create default user
    const now = Math.floor(Date.now() / 1000);
    db.prepare('INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)').run('1', 'Test User', now);

    // Insert local source
    db.prepare('INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)').run(1, 'Local', 'local');

    // ---- Artist and Album ----
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(2001, 'E2E Playlist Artist');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(2001, 'Playlist Test Album', 2001, 2022);

    // ---- 5 Tracks ----
    const trackTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
    trackTitles.forEach((title, i) => {
      const trackId = 2001 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(trackId, title, 2001, 2001, i + 1, 1, 2.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(trackId, 1, 'available', wavPath);
    });

    // ---- Pre-seeded empty playlist ----
    db.prepare(`
      INSERT INTO playlists (id, name, owner_id, is_public, is_favorite, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `).run('3001', 'Favorites', '1', 0, 0, now, now);

    const trackCount = db.prepare('SELECT COUNT(*) as count FROM tracks').get().count;
    const playlistCount = db.prepare('SELECT COUNT(*) as count FROM playlists').get().count;
    console.log(`[Playlists E2E Setup] ✓ Seeded ${trackCount} tracks and ${playlistCount} playlist(s)`);

  } finally {
    db.close();
  }

  console.log('[Playlists E2E Setup] ✓ Test environment ready');
  return testDbPath;
}

/**
 * Cleanup: remove temp test files.
 */
function cleanupTestDatabase() {
  if (testDbDir) {
    console.log('[Playlists E2E Cleanup] Removing test environment...');
    try {
      rmSync(testDbDir, { recursive: true, force: true });
      console.log('[Playlists E2E Cleanup] ✓ Removed');
    } catch (error) {
      console.error('[Playlists E2E Cleanup] Failed to remove:', error);
    }
  }
}

export const config = {
  ...baseConfig,

  // Run only the playlists spec
  specs: ['./tests/specs/playlists.e2e.js'],

  // Longer timeout for audio loading
  mochaOpts: {
    ...baseConfig.mochaOpts,
    timeout: 90000,
  },

  onPrepare: async function (config, capabilities) {
    testDbPath = setupTestDatabase();
    process.env.DATABASE_PATH = testDbPath;
    console.log('[Playlists E2E] DATABASE_PATH:', testDbPath);

    if (baseConfig.onPrepare) {
      await baseConfig.onPrepare(config, capabilities);
    }
  },

  before: async function (capabilities, specs) {
    console.log('[Playlists E2E] Starting tests with isolated database');
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
      console.log('[Playlists E2E] Test failed:', test.title);
      console.log('[Playlists E2E] Error:', error.message);
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const screenshotPath = `./screenshots/playlists-${timestamp}.png`;
      try {
        await browser.saveScreenshot(screenshotPath);
        console.log(`[Playlists E2E] Screenshot: ${screenshotPath}`);
      } catch (e) {
        console.error('[Playlists E2E] Screenshot failed:', e);
      }
    }
  },
};
