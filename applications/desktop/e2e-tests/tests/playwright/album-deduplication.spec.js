/**
 * Album deduplication — Playwright CDP regression tests
 *
 * Regression guard for: albums with similar incremental names (e.g. "Vol I" / "Vol II")
 * being incorrectly merged into one album by the fuzzy Levenshtein matcher.
 *
 * Root cause: find_or_create_album_cached used a 60% Levenshtein threshold, causing
 * "Vol I" and "Vol II" (91% similar) to be merged into the same album on import.
 * Fix: removed Levenshtein fallback for albums — only exact/normalized-exact matches
 *       can merge albums now. Different titles always create distinct albums.
 *
 * Test strategy: WAV files without embedded tags fall back to folder-name heuristics
 * in extract_metadata() (metadata.rs). The scanner derives artist/album from the
 * folder hierarchy when tags are absent:
 *
 *   <import-root>/
 *     Regression Artist/         ← grandparent → artist
 *       Vol I/                   ← parent → album "Vol I"
 *         track.wav
 *       Vol II/                  ← parent → album "Vol II"
 *         track.wav
 *
 * After importing, the DB must contain two separate albums for "Regression Artist",
 * not one merged album.
 *
 * NOTE: All WAV files are given unique content (unique byte written at offset 44+)
 * to avoid skip_duplicates hash collisions across tests.
 */

import { test, expect, chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- Shared state ----

let browser;
let page;

// ---- Counter for unique WAV content (avoids hash collisions across tests) ----

let _wavCounter = 0;

/**
 * Create a minimal silent WAV buffer with a unique identifier embedded in the
 * audio data section, so every call produces a distinct file hash.
 * This prevents skip_duplicates from silently swallowing re-imports.
 */
function createUniqueWavBuffer(durationSeconds = 2) {
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

  // Write a unique value into the first sample of audio data so every file
  // gets a distinct SHA-256 hash, preventing false duplicate detection.
  const unique = (Date.now() * 1000 + (++_wavCounter)) % 0x7fff;
  if (dataSize >= 2) buf.writeInt16LE(unique, 44);

  return buf;
}

// ---- Helper: wait for import to finish ----

async function waitForImportComplete(page, timeoutMs = 30_000) {
  // Brief delay so the background import task has time to set is_importing=true
  // before we start polling. Very fast imports (< a few ms) may complete before
  // the first poll otherwise, causing a false "already done" reading.
  await page.waitForTimeout(200);

  await page.waitForFunction(
    async () => {
      const importing = await window.__TAURI_INTERNALS__.invoke('is_importing');
      return importing === false;
    },
    { timeout: timeoutMs },
  );
}

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash'),
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
  await waitForImportComplete(page, 15_000).catch(() => {});
});

test.afterAll(async () => {
  await browser.close();
});

// ================================================================
// Test 1: Similar incremental album names are imported as separate
//         albums, not merged.
//
// Regression: "Vol I" and "Vol II" were merged (91% Levenshtein)
// into the same album when the fuzzy threshold was 60%.
// ================================================================

test('albums with similar incremental names (Vol I / Vol II) are kept separate', async () => {
  test.setTimeout(60_000);

  // Create folder structure: grandparent = artist, parent = album.
  // When WAV tags are absent, metadata.rs falls back to folder names.
  const importDir = join(tmpdir(), `soul-dedup-vol-${Date.now()}`);
  const vol1Dir = join(importDir, 'Regression Artist', 'Vol I');
  const vol2Dir = join(importDir, 'Regression Artist', 'Vol II');
  mkdirSync(vol1Dir, { recursive: true });
  mkdirSync(vol2Dir, { recursive: true });
  writeFileSync(join(vol1Dir, 'track.wav'), createUniqueWavBuffer(2));
  writeFileSync(join(vol2Dir, 'track.wav'), createUniqueWavBuffer(2));

  const albumsBefore = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );

  await page.evaluate(async dir => {
    await window.__TAURI_INTERNALS__.invoke('import_directory', { directory: dir });
  }, importDir);

  await waitForImportComplete(page);

  const albumsAfter = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );

  const regressionAlbums = albumsAfter.filter(a => a.artist_name === 'Regression Artist');
  const titles = regressionAlbums.map(a => a.title).sort();

  // Both volumes must exist as separate albums.
  expect(titles).toContain('Vol I');
  expect(titles).toContain('Vol II');
  expect(regressionAlbums.length).toBe(2);

  // Sanity: total album count increased by exactly 2.
  expect(albumsAfter.length).toBe(albumsBefore.length + 2);
});

// ================================================================
// Test 2: Albums with identical names for different artists stay
//         separate (artist-scoped album deduplication).
//
// Verifies that the artist_id dimension of the album key is working:
// two artists can both have a "Self Titled" album without collision.
// ================================================================

test('same album title for different artists creates two separate albums', async () => {
  test.setTimeout(60_000);

  // Use artist names that are clearly distinct (low Levenshtein similarity to each
  // other and to all other test artists) so the artist fuzzy matcher does NOT merge them.
  const importDir = join(tmpdir(), `soul-dedup-same-name-${Date.now()}`);
  const alpha = join(importDir, 'Thunderbird Ensemble', 'Self Titled');
  const beta = join(importDir, 'Jade Dragon Collective', 'Self Titled');
  mkdirSync(alpha, { recursive: true });
  mkdirSync(beta, { recursive: true });
  writeFileSync(join(alpha, 'track.wav'), createUniqueWavBuffer(2));
  writeFileSync(join(beta, 'track.wav'), createUniqueWavBuffer(2));

  const albumsBefore = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );

  await page.evaluate(async dir => {
    await window.__TAURI_INTERNALS__.invoke('import_directory', { directory: dir });
  }, importDir);

  await waitForImportComplete(page);

  const albumsAfter = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );

  const selfTitled = albumsAfter.filter(
    a => a.title === 'Self Titled' &&
         (a.artist_name === 'Thunderbird Ensemble' || a.artist_name === 'Jade Dragon Collective'),
  );

  expect(selfTitled.length).toBe(2);
  const artistNames = selfTitled.map(a => a.artist_name).sort();
  expect(artistNames).toEqual(['Jade Dragon Collective', 'Thunderbird Ensemble']);

  expect(albumsAfter.length).toBe(albumsBefore.length + 2);
});

// ================================================================
// Test 3: Exact same album name + same artist deduplicates correctly.
//
// Positive case: importing the same album twice must NOT create a
// duplicate album entry — the second import finds the existing row.
// ================================================================

test('importing the same album twice does not create a duplicate album', async () => {
  test.setTimeout(60_000);

  // Use a clearly distinct artist name so the artist fuzzy matcher won't merge it
  // with any other test artist ("Regression Artist", "Thunderbird Ensemble", etc.).
  const importDir = join(tmpdir(), `soul-dedup-dup-${Date.now()}`);
  const albumDir = join(importDir, 'Opal Frequency', 'Same Album');
  mkdirSync(albumDir, { recursive: true });
  // Write two tracks with unique hashes so neither is a duplicate of each other.
  writeFileSync(join(albumDir, 'track1.wav'), createUniqueWavBuffer(2));
  writeFileSync(join(albumDir, 'track2.wav'), createUniqueWavBuffer(2));

  // First import: creates the album.
  await page.evaluate(async dir => {
    await window.__TAURI_INTERNALS__.invoke('import_directory', { directory: dir });
  }, importDir);
  await waitForImportComplete(page);

  const albumsAfterFirst = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );
  const afterFirst = albumsAfterFirst.filter(
    a => a.title === 'Same Album' && a.artist_name === 'Opal Frequency',
  );

  // Diagnostic: log what's in DB if assertion is about to fail
  if (afterFirst.length !== 1) {
    console.log('[DIAG test3] afterFirst:', JSON.stringify(afterFirst));
    console.log('[DIAG test3] all "Same Album" entries:',
      JSON.stringify(albumsAfterFirst.filter(a => a.title === 'Same Album')));
    console.log('[DIAG test3] all "Opal" entries:',
      JSON.stringify(albumsAfterFirst.filter(a => a.artist_name && a.artist_name.includes('Opal'))));
  }
  expect(afterFirst.length).toBe(1);

  // Second import of the same directory: tracks are duplicates (same hash),
  // so they're skipped — but the album row must NOT be created again.
  await page.evaluate(async dir => {
    await window.__TAURI_INTERNALS__.invoke('import_directory', { directory: dir });
  }, importDir);
  await waitForImportComplete(page);

  const albumsAfterSecond = await page.evaluate(
    async () => window.__TAURI_INTERNALS__.invoke('get_all_albums'),
  );
  const afterSecond = albumsAfterSecond.filter(
    a => a.title === 'Same Album' && a.artist_name === 'Opal Frequency',
  );

  // Still exactly one album.
  expect(afterSecond.length).toBe(1);
  expect(afterSecond[0].id).toBe(afterFirst[0].id);
});
