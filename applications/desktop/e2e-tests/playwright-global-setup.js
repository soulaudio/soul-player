/**
 * Playwright global setup — seeds a test database, launches the Tauri binary
 * with CDP enabled, and waits until the debugger endpoint is ready.
 */

import { spawn } from 'child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { fileURLToPath } from 'url';
import Database from 'better-sqlite3';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const CDP_PORT = 9222;
export const CDP_URL = `http://localhost:${CDP_PORT}`;

// ---- App binary path (mirrors wdio.conf.js logic) ----

function getAppPath() {
  const workspaceRoot = join(__dirname, '..', '..', '..');
  const candidates = [
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop.exe'),
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop'),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  return candidates[0]; // will fail with a clear error below
}

// ---- Silent WAV factory (mirrors wdio.playback.conf.js) ----

function createSilentWavBuffer(durationSeconds = 2) {
  const sampleRate = 44100;
  const channels = 1;
  const bitsPerSample = 16;
  const numSamples = Math.floor(sampleRate * durationSeconds * channels);
  const dataSize = numSamples * (bitsPerSample / 8);
  const buf = Buffer.alloc(44 + dataSize, 0);
  let o = 0;
  buf.write('RIFF', o); o += 4;
  buf.writeUInt32LE(36 + dataSize, o); o += 4;
  buf.write('WAVE', o); o += 4;
  buf.write('fmt ', o); o += 4;
  buf.writeUInt32LE(16, o); o += 4;
  buf.writeUInt16LE(1, o); o += 2;
  buf.writeUInt16LE(channels, o); o += 2;
  buf.writeUInt32LE(sampleRate, o); o += 4;
  buf.writeUInt32LE(sampleRate * channels * (bitsPerSample / 8), o); o += 4;
  buf.writeUInt16LE(channels * (bitsPerSample / 8), o); o += 2;
  buf.writeUInt16LE(bitsPerSample, o); o += 2;
  buf.write('data', o); o += 4;
  buf.writeUInt32LE(dataSize, o);
  return buf;
}

// ---- Database seed ----

function seedDatabase() {
  const timestamp = Date.now();
  const dir = join(tmpdir(), `soul-player-playwright-${timestamp}`);
  const audioDir = join(dir, 'audio');
  mkdirSync(audioDir, { recursive: true });

  const wavPath = join(audioDir, 'test-track.wav');
  writeFileSync(wavPath, createSilentWavBuffer(2));

  const dbPath = join(dir, 'test.db');
  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');
  const db = new Database(dbPath);

  try {
    const files = readdirSync(migrationsDir).filter(f => f.endsWith('.sql')).sort();
    for (const file of files) {
      db.exec(readFileSync(join(migrationsDir, file), 'utf-8'));
    }

    const now = Math.floor(Date.now() / 1000);
    db.prepare('INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)').run('1', 'Test User', now);
    db.prepare('INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)').run(1, 'Local', 'local');

    // Artist + album (id 2001 to match playlists.e2e.js seeds)
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(2001, 'Playwright Artist');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(2001, 'Playwright Album', 2001, 2024);

    const trackTitles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
    trackTitles.forEach((title, i) => {
      const tid = 2001 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(tid, title, 2001, 2001, i + 1, 1, 2.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(tid, 1, 'available', wavPath);
    });

    // Seed one playlist
    db.prepare(`
      INSERT INTO playlists (id, name, owner_id, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?)
    `).run('3001', 'Favorites', '1', now, now);

    console.log('[Playwright Setup] ✓ Database seeded');
  } finally {
    db.close();
  }

  return { dbPath, dir };
}

// ---- Global setup entry point ----

export default async function globalSetup() {
  const appPath = getAppPath();
  if (!existsSync(appPath)) {
    throw new Error(
      `[Playwright Setup] App binary not found: ${appPath}\n` +
      'Build it first: cargo build --release -p soul-player-desktop'
    );
  }

  const { dbPath, dir } = seedDatabase();
  process.env.DATABASE_PATH = dbPath;
  process.env.PLAYWRIGHT_TEST_DIR = dir;

  console.log(`[Playwright Setup] Launching: ${appPath}`);
  console.log(`[Playwright Setup] DATABASE_PATH: ${dbPath}`);

  const app = spawn(appPath, [], {
    env: {
      ...process.env,
      // Enable Edge WebView2 remote debugging so Playwright can connect via CDP
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT}`,
    },
    stdio: 'ignore',
    detached: false,
  });

  app.on('error', (err) => console.error('[Playwright Setup] App launch error:', err));

  process.env.PLAYWRIGHT_APP_PID = String(app.pid);
  console.log(`[Playwright Setup] App PID: ${app.pid}`);

  // Wait for CDP endpoint to become available (up to 30 s)
  console.log(`[Playwright Setup] Waiting for CDP at ${CDP_URL} ...`);
  const deadline = Date.now() + 30_000;
  let ready = false;
  while (Date.now() < deadline) {
    await new Promise(r => setTimeout(r, 500));
    try {
      const res = await fetch(`${CDP_URL}/json/version`);
      if (res.ok) { ready = true; break; }
    } catch { /* not ready yet */ }
  }

  if (!ready) throw new Error('[Playwright Setup] CDP endpoint did not become ready within 30 s');

  // Extra settle time for the app UI to finish loading
  await new Promise(r => setTimeout(r, 3000));
  console.log('[Playwright Setup] ✓ App ready');
}
