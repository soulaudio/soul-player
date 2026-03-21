/**
 * Playwright global setup — seeds a test database, launches the Tauri binary
 * with CDP enabled, and waits until the debugger endpoint is ready.
 */

import { spawn, execSync } from 'child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync, rmSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir, homedir } from 'os';
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

  // Longer WAV files for endurance/long-running tests
  const longWavPath = join(audioDir, 'test-track-long.wav');
  writeFileSync(longWavPath, createSilentWavBuffer(30));

  const medWavPath = join(audioDir, 'test-track-med.wav');
  writeFileSync(medWavPath, createSilentWavBuffer(15));

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

    // Separate artist for endurance albums so Artist context tests are not affected
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(2002, 'Endurance Artist');

    // Album 2002 — "Long Album" — 5 tracks × 30-second WAV files
    // Used by long-running / endurance stress tests
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(2002, 'Long Album', 2002, 2024);
    const longTitles = ['Long One', 'Long Two', 'Long Three', 'Long Four', 'Long Five'];
    longTitles.forEach((title, i) => {
      const tid = 3001 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(tid, title, 2002, 2002, i + 1, 1, 30.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(tid, 1, 'available', longWavPath);
    });

    // Album 2003 — "Marathon Album" — 10 tracks × 15-second WAV files
    // Used by extended queue endurance tests
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(2003, 'Marathon Album', 2002, 2024);
    const marathonTitles = ['Marathon 01', 'Marathon 02', 'Marathon 03', 'Marathon 04', 'Marathon 05',
                            'Marathon 06', 'Marathon 07', 'Marathon 08', 'Marathon 09', 'Marathon 10'];
    marathonTitles.forEach((title, i) => {
      const tid = 4001 + i;
      db.prepare(`
        INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(tid, title, 2002, 2003, i + 1, 1, 15.0, 'wav');
      db.prepare(`
        INSERT INTO track_sources (track_id, source_id, status, local_file_path)
        VALUES (?, ?, ?, ?)
      `).run(tid, 1, 'available', medWavPath);
    });

    // Seed one playlist
    db.prepare(`
      INSERT INTO playlists (id, name, owner_id, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?)
    `).run('3001', 'Favorites', '1', now, now);

    // Seed a genre and link the first 5 album-2001 tracks to it.
    // Track 2006 (Collab Track) is added to the genre after it's inserted below.
    // SQLite allows explicit INTEGER PRIMARY KEY inserts even with AUTOINCREMENT,
    // as long as the value hasn't been used before.
    db.prepare(`
      INSERT OR IGNORE INTO genres (id, name, canonical_name)
      VALUES (?, ?, ?)
    `).run(4001, 'Playwright Genre', 'playwright genre');

    // track_genres uses INTEGER track_id (after migration 20250106000009)
    // Only link Album 2001 tracks to the genre so existing genre tests aren't affected
    const genreTrackIds = [2001, 2002, 2003, 2004, 2005];
    const insertTrackGenre = db.prepare(
      'INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?, ?)'
    );
    for (const tid of genreTrackIds) {
      insertTrackGenre.run(tid, 4001);
    }

    // Populate track_artists junction so get_by_artist queries work (they now join via junction).
    const insertTrackArtist = db.prepare(
      'INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?, ?, ?)'
    );
    // Album 2001 tracks → Playwright Artist (2001)
    for (const tid of [2001, 2002, 2003, 2004, 2005]) {
      insertTrackArtist.run(String(tid), 2001, 0);
    }
    // Long Album tracks (3001-3005) → Endurance Artist (2002)
    for (const tid of [3001, 3002, 3003, 3004, 3005]) {
      insertTrackArtist.run(String(tid), 2002, 0);
    }
    // Marathon Album tracks (4001-4010) → Endurance Artist (2002)
    for (let tid = 4001; tid <= 4010; tid++) {
      insertTrackArtist.run(String(tid), 2002, 0);
    }

    // Multi-artist test data: "Featured Artist" appears only on the collab track
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(2003, 'Featured Artist');

    // "Collab Track" — track 2006, in Playwright Album, 2 artists in the junction
    db.prepare(`
      INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(2006, 'Collab Track', 2001, 2001, 6, 1, 2.0, 'wav');
    db.prepare(`
      INSERT INTO track_sources (track_id, source_id, status, local_file_path)
      VALUES (?, ?, ?, ?)
    `).run(String(2006), 1, 'available', wavPath);
    // Primary: Playwright Artist (position 0); Featured: Featured Artist (position 1)
    insertTrackArtist.run(String(2006), 2001, 0);
    insertTrackArtist.run(String(2006), 2003, 1);
    // Add Collab Track to genre 4001 (track now exists)
    insertTrackGenre.run(2006, 4001);

    // Seed Favorites playlist with all 6 album-2001 tracks
    // (must run after all tracks including Collab Track 2006 are inserted)
    const insertPlaylistTrack = db.prepare(
      'INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)'
    );
    for (let i = 0; i < 6; i++) {
      const tid = i < 5 ? 2001 + i : 2006;
      insertPlaylistTrack.run('3001', tid, i);
    }

    // Seed a library_sources record so the app skips the onboarding screen.
    // The desktop app uses device_id = 'desktop-local' (hardcoded in library_settings.rs).
    db.prepare(`
      INSERT OR IGNORE INTO library_sources (user_id, device_id, name, source_type, path, enabled, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run('1', 'desktop-local', 'Test Music', 'watched', audioDir, 1, now, now);

    // ── DSD seed data ─────────────────────────────────────────────────────────
    // Artist 5001 / Album 5001 / Tracks 5001 (.dsf) and 5002 (.dff)
    // Used by dsd-playback.spec.js.
    //
    // The DSF/DFF files contain minimal valid headers with real audio data so that
    // DsdAudioSource can open and play them.  We build them programmatically here
    // to avoid committing binary assets.

    const DSF_BLOCK_SIZE = 4096;
    const DSD_RATE = 2_822_400;

    // Build a minimal DSF file: DSD chunk + fmt chunk + data chunk.
    // 8 blocks × 4096 samples per channel × 2 channels = ~11.6ms of audio — enough
    // for the decoder thread to fill its ring buffer and report is_ready().
    function buildDsf(numBlocks) {
      const channels = 2;
      const sampleCount = BigInt(numBlocks * DSF_BLOCK_SIZE);
      const audioDataLen = numBlocks * DSF_BLOCK_SIZE * channels;
      const totalSize = BigInt(92 + audioDataLen); // 28 + 52 + 12 = 92 header bytes

      const buf = Buffer.alloc(92 + audioDataLen, 0);
      let o = 0;

      // DSD chunk (28 bytes)
      buf.write('DSD ', o, 'ascii'); o += 4;
      buf.writeBigUInt64LE(28n, o); o += 8;
      buf.writeBigUInt64LE(totalSize, o); o += 8;
      buf.writeBigUInt64LE(0n, o); o += 8; // no ID3

      // fmt chunk (52 bytes)
      buf.write('fmt ', o, 'ascii'); o += 4;
      buf.writeBigUInt64LE(52n, o); o += 8;
      buf.writeUInt32LE(1, o); o += 4; // format version
      buf.writeUInt32LE(0, o); o += 4; // format ID (DSD raw)
      buf.writeUInt32LE(2, o); o += 4; // channel type: stereo
      buf.writeUInt32LE(channels, o); o += 4;
      buf.writeUInt32LE(DSD_RATE, o); o += 4;
      buf.writeUInt32LE(1, o); o += 4; // bits_per_sample = 1 (LSB-first)
      buf.writeBigUInt64LE(sampleCount, o); o += 8;
      buf.writeUInt32LE(DSF_BLOCK_SIZE, o); o += 4;
      buf.writeUInt32LE(0, o); o += 4; // reserved

      // data chunk header (12 bytes)
      buf.write('data', o, 'ascii'); o += 4;
      buf.writeBigUInt64LE(BigInt(12 + audioDataLen), o); o += 8;

      // Audio: alternating 0x69 pattern (non-zero DSD data)
      buf.fill(0x69, o, o + audioDataLen);

      return buf;
    }

    // Build a minimal DSDIFF (.dff) file.
    function buildDff(numSamplesPerChannel) {
      const channels = 2;
      const audioData = Buffer.alloc(numSamplesPerChannel * channels, 0x96); // non-zero

      // PROP inner chunks
      const propInner = Buffer.alloc(0);
      const parts = [];
      parts.push(Buffer.from('SND ', 'ascii'));

      // FS chunk
      const fs = Buffer.alloc(12 + 4);
      fs.write('FS  ', 0, 'ascii');
      fs.writeBigUInt64BE(4n, 4);
      fs.writeUInt32BE(DSD_RATE, 12);
      parts.push(fs);

      // CHNL chunk: 12 hdr + 2 + 4 + 4 = 22
      const chnl = Buffer.alloc(12 + 10);
      chnl.write('CHNL', 0, 'ascii');
      chnl.writeBigUInt64BE(10n, 4);
      chnl.writeUInt16BE(channels, 12);
      chnl.write('MLFT', 14, 'ascii');
      chnl.write('MRGT', 18, 'ascii');
      parts.push(chnl);

      // CMPR chunk
      const cmprName = Buffer.from('not compressed');
      const cmprDataSize = 4 + 1 + cmprName.length + 1; // 20
      const cmpr = Buffer.alloc(12 + cmprDataSize);
      cmpr.write('CMPR', 0, 'ascii');
      cmpr.writeBigUInt64BE(BigInt(cmprDataSize), 4);
      cmpr.write('DSD ', 12, 'ascii');
      cmpr.writeUInt8(cmprName.length, 16);
      cmprName.copy(cmpr, 17);
      // pad byte at end (already zero from alloc)
      parts.push(cmpr);

      const propPayload = Buffer.concat(parts);

      // PROP chunk: 12 hdr + propPayload
      const prop = Buffer.alloc(12 + propPayload.length);
      prop.write('PROP', 0, 'ascii');
      prop.writeBigUInt64BE(BigInt(propPayload.length), 4);
      propPayload.copy(prop, 12);

      // FVER chunk
      const fver = Buffer.alloc(12 + 4);
      fver.write('FVER', 0, 'ascii');
      fver.writeBigUInt64BE(4n, 4);
      fver.writeUInt32BE(0x01050000, 12);

      // DSD sound data chunk
      const dsdChunk = Buffer.alloc(12 + audioData.length);
      dsdChunk.write('DSD ', 0, 'ascii');
      dsdChunk.writeBigUInt64BE(BigInt(audioData.length), 4);
      audioData.copy(dsdChunk, 12);

      // FRM8 inner = 'DSD ' form type + fver + prop + dsdChunk
      const formType = Buffer.from('DSD ', 'ascii');
      const inner = Buffer.concat([formType, fver, prop, dsdChunk]);

      // FRM8 outer
      const out = Buffer.alloc(12 + inner.length);
      out.write('FRM8', 0, 'ascii');
      out.writeBigUInt64BE(BigInt(inner.length), 4);
      inner.copy(out, 12);

      return out;
    }

    // Create DSF and DFF audio files
    const dsfPath = join(audioDir, 'dsd-track-one.dsf');
    const dffPath = join(audioDir, 'dsd-track-two.dff');
    // 512 blocks × 4096 samples = 2 097 152 DSD samples per channel ≈ 0.74s
    writeFileSync(dsfPath, buildDsf(512));
    // 524288 samples per channel ≈ 0.19s
    writeFileSync(dffPath, buildDff(524288));

    // DSD Artist + Album
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(5001, 'DSD Artist');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(5001, 'DSD Album', 5001, 2024);

    // Track 5001 — DSF file
    db.prepare(`
      INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(5001, 'DSD Track One', 5001, 5001, 1, 1, 0.74, 'dsf');
    db.prepare(`
      INSERT INTO track_sources (track_id, source_id, status, local_file_path)
      VALUES (?, ?, ?, ?)
    `).run(5001, 1, 'available', dsfPath);

    // Track 5002 — DFF file
    db.prepare(`
      INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(5002, 'DSD Track Two', 5001, 5001, 2, 1, 0.19, 'dff');
    db.prepare(`
      INSERT INTO track_sources (track_id, source_id, status, local_file_path)
      VALUES (?, ?, ?, ?)
    `).run(5002, 1, 'available', dffPath);

    // track_artists junction for DSD tracks
    insertTrackArtist.run(String(5001), 5001, 0);
    insertTrackArtist.run(String(5002), 5001, 0);

    // Add a DSF file to the import dir so the scan test (test 5) can discover it.
    writeFileSync(join(importDir, 'dsd-import-01.dsf'), buildDsf(32));

    console.log('[Playwright Setup] ✓ DSD seed data created');
    // ── end DSD seed ──────────────────────────────────────────────────────────

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

  // Clear WebView2 cache to ensure fresh JS is served after dist rebuilds.
  // WebView2 persists its cache in the app's EBWebView folder; --disable-cache only
  // disables the HTTP network cache but does NOT clear the existing code cache.
  const webView2CacheDir = join(
    process.env.LOCALAPPDATA || join(homedir(), 'AppData', 'Local'),
    'com.soulaudio.player', 'EBWebView', 'Default', 'Cache'
  );
  try {
    if (existsSync(webView2CacheDir)) {
      rmSync(webView2CacheDir, { recursive: true, force: true });
      console.log('[Playwright Setup] ✓ Cleared WebView2 cache at', webView2CacheDir);
    }
  } catch (e) {
    console.log('[Playwright Setup] ⚠ Could not clear WebView2 cache:', e.message);
  }

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
      // Enable Edge WebView2 remote debugging so Playwright can connect via CDP.
      // --disable-cache: prevents WebView2 from serving stale JS after a dist rebuild.
      // Without this, the browser cache can serve old JS (e.g. without role="menuitem")
      // even after `yarn build` produces updated assets, causing `[role="menuitem"]`
      // queries to find 0 elements and time out.
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT} --disable-cache`,
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

      // Clear browser cache to ensure fresh JS files are loaded (not stale WebView2 cache)
      try {
        const cdpSession = await mainPage.context().newCDPSession(mainPage);
        await cdpSession.send('Network.clearBrowserCache');
        console.log('[Playwright Setup] ✓ Browser cache cleared');
        await cdpSession.detach();
      } catch (e) {
        console.log('[Playwright Setup] ⚠ Could not clear browser cache:', e.message);
      }

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
  process.env.SOUL_CDP_URL = CDP_URL;
}
