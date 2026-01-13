/**
 * Seed test data for e2e testing
 * Creates 500 dummy tracks with actual audio files
 */

import { Database } from 'better-sqlite3';
import * as fs from 'fs';
import * as path from 'path';

const DB_PATH = path.join(__dirname, '../libraries/soul-storage/.tmp/dev.db');
const TEST_AUDIO_DIR = path.join(__dirname, '../test-audio');

async function createSilentMP3(filePath: string, durationSeconds: number = 3) {
  const dir = path.dirname(filePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  // Create a minimal valid MP3 file (silent)
  // This is a 1-second silent MP3 at 44.1kHz, 128kbps (minimal header)
  const mp3Header = Buffer.from([
    0xFF, 0xFB, 0x90, 0x00, // MP3 sync word + MPEG1 Layer3 128kbps
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  ]);

  // Repeat for approximate duration
  const frames = Math.ceil(durationSeconds * 38); // ~38 frames per second at 128kbps
  const data = Buffer.alloc(frames * 417); // 417 bytes per frame

  for (let i = 0; i < frames; i++) {
    mp3Header.copy(data, i * 417);
  }

  fs.writeFileSync(filePath, data);
}

async function seedDatabase() {
  console.log('[SeedTestData] Opening database:', DB_PATH);

  const Database = require('better-sqlite3');
  const db = new Database(DB_PATH);

  try {
    // Enable foreign keys
    db.pragma('foreign_keys = ON');

    // Insert test artist
    console.log('[SeedTestData] Inserting test artist...');
    db.prepare(`
      INSERT OR IGNORE INTO artists (id, name)
      VALUES (9999, 'Test Artist')
    `).run();

    // Insert test album
    console.log('[SeedTestData] Inserting test album...');
    db.prepare(`
      INSERT OR IGNORE INTO albums (id, title, artist_id, year)
      VALUES (9999, 'Test Album', 9999, 2024)
    `).run();

    // Insert test source
    console.log('[SeedTestData] Inserting test source...');
    db.prepare(`
      INSERT OR IGNORE INTO sources (id, name, source_type)
      VALUES (1, 'Local Library', 'local')
    `).run();

    // Insert 500 test tracks
    console.log('[SeedTestData] Inserting 500 test tracks...');

    const insertTrack = db.prepare(`
      INSERT OR IGNORE INTO tracks (
        id, title, artist_id, album_id, track_number, disc_number,
        duration_seconds, file_format, bit_rate, sample_rate, channels
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const insertAvailability = db.prepare(`
      INSERT OR IGNORE INTO track_availability (
        track_id, source_id, status, local_file_path
      ) VALUES (?, ?, ?, ?)
    `);

    const transaction = db.transaction((count: number) => {
      for (let i = 1; i <= count; i++) {
        const trackId = 10000 + i;
        const filePath = `test-audio/track_${i}.mp3`;

        insertTrack.run(
          trackId,
          `Test Track ${i}`,
          9999,
          9999,
          i,
          1,
          180.0,
          'mp3',
          320,
          44100,
          2
        );

        insertAvailability.run(
          trackId,
          1,
          'available',
          filePath
        );

        // Create actual audio file
        if (i <= 10 || i % 50 === 0) {
          console.log(`[SeedTestData] Creating audio file ${i}/500...`);
        }
        createSilentMP3(path.join(__dirname, '..', filePath), 3);
      }
    });

    transaction(500);

    // Verify
    const count = db.prepare('SELECT COUNT(*) as count FROM tracks WHERE id >= 10000 AND id < 10500').get();
    console.log(`[SeedTestData] ✓ Inserted ${count.count} test tracks`);

    const availCount = db.prepare('SELECT COUNT(*) as count FROM track_availability WHERE track_id >= 10000 AND track_id < 10500').get();
    console.log(`[SeedTestData] ✓ Inserted ${availCount.count} availability records`);

  } finally {
    db.close();
  }

  console.log('[SeedTestData] ✓ Database seeded successfully!');
}

async function cleanupTestData() {
  console.log('[SeedTestData] Cleaning up test data...');

  const Database = require('better-sqlite3');
  const db = new Database(DB_PATH);

  try {
    db.prepare('DELETE FROM track_availability WHERE track_id >= 10000 AND track_id < 10500').run();
    db.prepare('DELETE FROM tracks WHERE id >= 10000 AND id < 10500').run();
    db.prepare('DELETE FROM albums WHERE id = 9999').run();
    db.prepare('DELETE FROM artists WHERE id = 9999').run();

    console.log('[SeedTestData] ✓ Cleanup complete');
  } finally {
    db.close();
  }

  // Remove test audio files
  if (fs.existsSync(TEST_AUDIO_DIR)) {
    fs.rmSync(TEST_AUDIO_DIR, { recursive: true, force: true });
    console.log('[SeedTestData] ✓ Test audio files removed');
  }
}

// CLI
const command = process.argv[2];

if (command === 'seed') {
  seedDatabase().catch(console.error);
} else if (command === 'cleanup') {
  cleanupTestData().catch(console.error);
} else {
  console.log('Usage: ts-node scripts/seed-test-data.ts [seed|cleanup]');
  process.exit(1);
}
