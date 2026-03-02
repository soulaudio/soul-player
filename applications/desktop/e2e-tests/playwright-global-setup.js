/**
 * Playwright global setup — seeds a test database, launches the Tauri binary
 * with CDP enabled, and waits until the debugger endpoint is ready.
 */

import { spawn, execSync } from 'child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { fileURLToPath } from 'url';
import { createHash } from 'crypto';
import Database from 'better-sqlite3';
import { chromium } from '@playwright/test';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const CDP_PORT = 9222;
export const CDP_URL = `http://localhost:${CDP_PORT}`;

// ---- App binary path (mirrors wdio.conf.js logic) ----

function getAppPath() {
  const workspaceRoot = join(__dirname, '..', '..', '..');
  const candidates = [
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop.exe'),
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop'),
    join(workspaceRoot, 'target', 'debug', 'soul-player-desktop.exe'),
    join(workspaceRoot, 'target', 'debug', 'soul-player-desktop'),
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
  writeFileSync(wavPath, createSilentWavBuffer(10));

  // Create a separate folder with 3 WAV files that are NOT pre-loaded into the DB.
  // These are used by import-and-scan.spec.js to exercise the real import pipeline.
  // The folder is exposed via process.env.PLAYWRIGHT_IMPORT_DIR.
  const importDir = join(dir, 'test-import-files');
  mkdirSync(importDir, { recursive: true });
  writeFileSync(join(importDir, 'import-track-01.wav'), createSilentWavBuffer(2));
  writeFileSync(join(importDir, 'import-track-02.wav'), createSilentWavBuffer(2));
  writeFileSync(join(importDir, 'import-track-03.wav'), createSilentWavBuffer(2));

  const dbPath = join(dir, 'test.db');
  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');
  const db = new Database(dbPath);

  try {
    const files = readdirSync(migrationsDir).filter(f => f.endsWith('.sql')).sort();
    for (const file of files) {
      db.exec(readFileSync(join(migrationsDir, file), 'utf-8'));
    }

    // Populate _sqlx_migrations so SQLx doesn't re-run migrations on startup.
    // SQLx checksum = SHA-384 of the raw migration file bytes (stored as BLOB).
    db.exec(`
      CREATE TABLE IF NOT EXISTS _sqlx_migrations (
        version INTEGER PRIMARY KEY NOT NULL,
        description TEXT NOT NULL,
        installed_on TEXT NOT NULL DEFAULT (datetime('now')),
        success INTEGER NOT NULL DEFAULT 1,
        checksum BLOB NOT NULL,
        execution_time INTEGER NOT NULL DEFAULT 0
      )
    `);
    const insertMigration = db.prepare(
      'INSERT OR IGNORE INTO _sqlx_migrations (version, description, checksum, execution_time) VALUES (?, ?, ?, ?)'
    );
    for (const file of files) {
      // filename format: 20250105000001_create_users.sql
      const match = file.match(/^(\d+)_(.+)\.sql$/);
      if (!match) continue;
      const version = parseInt(match[1], 10);
      const description = match[2].replace(/_/g, ' ');
      const content = readFileSync(join(migrationsDir, file));
      const checksum = Buffer.from(createHash('sha384').update(content).digest());
      insertMigration.run(version, description, checksum, 0);
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

    // Seed a genre and link all 5 tracks to it.
    // SQLite allows explicit INTEGER PRIMARY KEY inserts even with AUTOINCREMENT,
    // as long as the value hasn't been used before.
    db.prepare(`
      INSERT OR IGNORE INTO genres (id, name, canonical_name)
      VALUES (?, ?, ?)
    `).run(4001, 'Playwright Genre', 'playwright genre');

    // track_genres uses INTEGER track_id (after migration 20250106000009)
    const trackIds = [2001, 2002, 2003, 2004, 2005];
    const insertTrackGenre = db.prepare(
      'INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?, ?)'
    );
    for (const tid of trackIds) {
      insertTrackGenre.run(tid, 4001);
    }

    // Seed a library_sources record so the app skips the onboarding screen.
    // The desktop app uses device_id = 'desktop-local' (hardcoded in library_settings.rs).
    db.prepare(`
      INSERT OR IGNORE INTO library_sources (user_id, device_id, name, source_type, path, enabled, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run('1', 'desktop-local', 'Test Music', 'watched', audioDir, 1, now, now);

    console.log('[Playwright Setup] ✓ Database seeded');
  } finally {
    db.close();
  }

  return { dbPath, dir, importDir };
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

  const { dbPath, dir, importDir } = seedDatabase();
  process.env.DATABASE_PATH = dbPath;
  process.env.PLAYWRIGHT_TEST_DIR = dir;
  process.env.PLAYWRIGHT_IMPORT_DIR = importDir;

  // Kill any running instance first — tauri-plugin-single-instance causes second
  // instances to immediately exit, so we must ensure a clean slate.
  try {
    execSync('powershell -Command "Stop-Process -Name soul-player-desktop -Force -ErrorAction SilentlyContinue"', { stdio: 'ignore' });
    await new Promise(r => setTimeout(r, 1000)); // wait for process to fully exit
    console.log('[Playwright Setup] Killed any existing soul-player instance');
  } catch { /* nothing was running */ }

  // Ensure the Vite dev server is running (debug binary loads frontend from localhost:1420)
  const devServerReady = await fetch('http://localhost:1420').then(r => r.ok).catch(() => false);
  if (!devServerReady) {
    console.log('[Playwright Setup] Starting Vite dev server...');
    const desktopDir = join(__dirname, '..', '..', '..');
    const devServer = spawn('yarn', ['workspace', 'soul-player-desktop', 'dev'], {
      cwd: desktopDir,
      stdio: 'ignore',
      shell: true,
      detached: false,
    });
    process.env.PLAYWRIGHT_DEV_SERVER_PID = String(devServer.pid);
    // Wait for dev server to respond
    const devDeadline = Date.now() + 30_000;
    let devReady = false;
    while (Date.now() < devDeadline) {
      await new Promise(r => setTimeout(r, 1000));
      devReady = await fetch('http://localhost:1420').then(r => r.ok).catch(() => false);
      if (devReady) break;
    }
    if (!devReady) throw new Error('[Playwright Setup] Vite dev server did not start within 30s');
    console.log('[Playwright Setup] ✓ Dev server ready');
  } else {
    console.log('[Playwright Setup] Dev server already running');
  }

  console.log(`[Playwright Setup] Launching: ${appPath}`);
  console.log(`[Playwright Setup] DATABASE_PATH: ${dbPath}`);

  // Spawn a fresh instance of the binary.
  // Debug binaries built with `cargo build` (not `cargo tauri dev`) serve frontend assets
  // from the `frontendDist` path (../dist) via the tauri://localhost scheme.
  const app = spawn(appPath, [], {
    env: {
      ...process.env,
      DATABASE_PATH: dbPath,
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

  // Wait for the main window to fully load and show [data-testid="nav-albums"].
  // The main window starts hidden (visible:false in tauri.conf.json) and is shown
  // only after Rust initialization completes (~1-5s). WebView2 may throttle JS
  // in hidden windows, so we must wait for the actual DOM element rather than
  // using a fixed timeout.
  console.log('[Playwright Setup] Waiting for main window to become ready (nav-albums)...');
  {
    const setupBrowser = await chromium.connectOverCDP(CDP_URL);
    try {
      const readyDeadline = Date.now() + 120_000;
      let mainPage = null;

      while (Date.now() < readyDeadline) {
        const ctx = setupBrowser.contexts()[0];
        if (ctx) {
          const pages = ctx.pages();
          mainPage = pages.find(
            p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
                 && !p.url().includes('splash')
          );
          if (mainPage) {
            try {
              await mainPage.waitForSelector('[data-testid="nav-albums"]', { timeout: 5_000 });
              break; // Found it — app is ready
            } catch { /* not yet */ }
          }
        }
        await new Promise(r => setTimeout(r, 1_000));
      }

      if (!mainPage) throw new Error('[Playwright Setup] Main window never appeared in CDP');

      // Verify nav-albums is accessible
      const hasNav = await mainPage.evaluate(() =>
        !!document.querySelector('[data-testid="nav-albums"]')
      ).catch(() => false);
      if (!hasNav) throw new Error('[Playwright Setup] Main window never showed nav-albums within 120 s');
    } finally {
      await setupBrowser.close();
    }
  }
  console.log('[Playwright Setup] ✓ App ready (nav-albums visible)');
}
