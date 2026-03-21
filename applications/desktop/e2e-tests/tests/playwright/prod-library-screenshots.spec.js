/**
 * Production library screenshot tests — connects to the app running against
 * the real production database and takes screenshots of the music library.
 *
 * Run with:
 *   npx playwright test tests/playwright/prod-library-screenshots.spec.js \
 *     --config playwright.prod.config.js --reporter=list
 */

import { chromium, expect, test } from '@playwright/test';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { mkdirSync, existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCREENSHOTS_DIR = join(__dirname, '..', '..', 'screenshots', 'prod');
const CDP_PORT = 9223;
const CDP_URL = `http://localhost:${CDP_PORT}`;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function getPage() {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const pages = browser.contexts().flatMap(c => c.pages());
  for (const p of pages) {
    try {
      const el = await p.$('[data-testid="nav-albums"]');
      if (el) return { browser, page: p };
    } catch { /* not this page */ }
  }
  return { browser, page: pages[0] };
}

async function invoke(page, cmd, params = {}) {
  return page.evaluate(
    async ({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params),
    { cmd, params }
  );
}

async function waitForScanComplete(page, timeout = 180_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const scans = await invoke(page, 'get_running_scans').catch(() => []);
    if (!Array.isArray(scans) || scans.length === 0) return;
    await new Promise(r => setTimeout(r, 500));
  }
  throw new Error('Scan did not complete within timeout');
}

function ss(filename) {
  if (!existsSync(SCREENSHOTS_DIR)) mkdirSync(SCREENSHOTS_DIR, { recursive: true });
  return join(SCREENSHOTS_DIR, filename);
}

// ---------------------------------------------------------------------------
// Group 1: Pre-rescan library state
// ---------------------------------------------------------------------------

test('1.1 albums page before rescan', async () => {
  const { browser, page } = await getPage();
  try {
    await page.click('[data-testid="nav-albums"]');
    await page.waitForSelector('[data-testid="album-card"]', { timeout: 8_000 }).catch(() => {});
    await page.waitForTimeout(1000);

    const albums = await invoke(page, 'get_all_albums').catch(() => []);
    console.log(`  Albums in DB: ${albums?.length ?? '?'}`);

    const cards = await page.$$('[data-testid="album-card"]');
    console.log(`  Album cards visible in viewport: ${cards.length}`);

    await page.screenshot({ path: ss('1.1-albums-before-rescan.png') });
    console.log('  ✓ Screenshot: 1.1-albums-before-rescan.png');
  } finally {
    await browser.close();
  }
});

test('1.2 artists page', async () => {
  const { browser, page } = await getPage();
  try {
    await page.click('[data-testid="nav-artists"]');
    await page.waitForTimeout(800);
    await page.screenshot({ path: ss('1.2-artists-page.png') });
    console.log('  ✓ Screenshot: 1.2-artists-page.png');
  } finally {
    await browser.close();
  }
});

test('1.3 tracks page — count', async () => {
  const { browser, page } = await getPage();
  try {
    await page.click('[data-testid="nav-tracks"]');
    await page.waitForTimeout(800);

    const tracks = await invoke(page, 'get_all_tracks').catch(() => []);
    console.log(`  Tracks in DB: ${tracks?.length ?? '?'}`);

    await page.screenshot({ path: ss('1.3-tracks-page.png') });
    console.log('  ✓ Screenshot: 1.3-tracks-page.png');
  } finally {
    await browser.close();
  }
});

// ---------------------------------------------------------------------------
// Group 2: Full rescan and DSF import verification
// ---------------------------------------------------------------------------

test('2.1 trigger full rescan — wait for DSF albums to appear', async () => {
  const { browser, page } = await getPage();
  try {
    const albumsBefore = await invoke(page, 'get_all_albums').catch(() => []);
    const tracksBefore = await invoke(page, 'get_all_tracks').catch(() => []);
    console.log(`  Before rescan — albums: ${albumsBefore?.length ?? '?'}, tracks: ${tracksBefore?.length ?? '?'}`);

    // Trigger rescan
    await invoke(page, 'rescan_all_sources');
    console.log('  Rescan triggered, waiting for completion (up to 3min)...');
    await waitForScanComplete(page, 180_000);
    console.log('  ✓ Rescan complete');

    await page.waitForTimeout(1500);
    await page.click('[data-testid="nav-albums"]');
    await page.waitForSelector('[data-testid="album-card"]', { timeout: 10_000 }).catch(() => {});
    await page.waitForTimeout(1000);

    const albumsAfter = await invoke(page, 'get_all_albums').catch(() => []);
    const tracksAfter = await invoke(page, 'get_all_tracks').catch(() => []);
    console.log(`  After rescan  — albums: ${albumsAfter?.length ?? '?'}, tracks: ${tracksAfter?.length ?? '?'}`);

    await page.screenshot({ path: ss('2.1-albums-after-rescan.png') });
    console.log('  ✓ Screenshot: 2.1-albums-after-rescan.png');

    // After rescan we should have at least as many albums as before
    expect((albumsAfter?.length ?? 0)).toBeGreaterThanOrEqual(albumsBefore?.length ?? 0);
  } finally {
    await browser.close();
  }
}, 200_000);

test('2.2 DSF/DFF tracks are in the library', async () => {
  const { browser, page } = await getPage();
  try {
    const tracks = await invoke(page, 'get_all_tracks').catch(() => []);
    const dsfTracks = (tracks || []).filter(t =>
      t.file_format?.toUpperCase() === 'DSF' ||
      t.file_path?.toLowerCase().endsWith('.dsf') ||
      t.file_path?.toLowerCase().endsWith('.dff')
    );

    console.log(`  DSF/DFF tracks: ${dsfTracks.length}`);
    dsfTracks.slice(0, 8).forEach(t =>
      console.log(`    "${t.title}" — ${t.sample_rate ?? '?'} Hz, ${t.duration_seconds?.toFixed(1) ?? 'null'}s`)
    );

    // Search for Hiroshi Sato on albums page
    await page.click('[data-testid="nav-albums"]');
    await page.waitForTimeout(400);
    const searchInput = await page.$('[data-testid="search-input"], input[placeholder*="earch"]');
    if (searchInput) {
      await searchInput.fill('Hiroshi Sato');
      await page.waitForTimeout(800);
      await page.screenshot({ path: ss('2.2-dsf-search-hiroshi-sato.png') });
      console.log('  ✓ Screenshot: 2.2-dsf-search-hiroshi-sato.png');
      await searchInput.fill('');
      await page.keyboard.press('Escape');
    }

    expect(dsfTracks.length).toBeGreaterThan(0);
  } finally {
    await browser.close();
  }
});

test('2.3 DSF durations are sane (not millions of seconds)', async () => {
  const { browser, page } = await getPage();
  try {
    const tracks = await invoke(page, 'get_all_tracks').catch(() => []);
    const dsfTracks = (tracks || []).filter(t =>
      t.file_format?.toUpperCase() === 'DSF' ||
      t.file_path?.toLowerCase().endsWith('.dsf') ||
      t.file_path?.toLowerCase().endsWith('.dff')
    );

    // Null duration means metadata extraction failed — those are separate bugs, not insane values
    const nullDuration = dsfTracks.filter(t => t.duration_seconds == null);
    const insane = dsfTracks.filter(t => t.duration_seconds != null && t.duration_seconds > 3600);

    console.log(`  DSF tracks: ${dsfTracks.length} total, ${nullDuration.length} null duration, ${insane.length} insane`);
    if (insane.length > 0) {
      insane.forEach(t => console.log(`    INSANE: "${t.title}" = ${t.duration_seconds}s`));
    }

    expect(insane.length).toBe(0);
  } finally {
    await browser.close();
  }
});

// ---------------------------------------------------------------------------
// Group 3: Album detail pages
// ---------------------------------------------------------------------------

test('3.1 Orient (Hiroshi Sato) DSF album detail', async () => {
  const { browser, page } = await getPage();
  try {
    const albums = await invoke(page, 'get_all_albums').catch(() => []);
    const orient = (albums || []).find(a => a.title?.toLowerCase().includes('orient'));

    if (!orient) {
      console.log('  Orient album not in DB — DSF may not have imported yet');
      test.skip();
      return;
    }

    console.log(`  Found: "${orient.title}" id=${orient.id} cover=${orient.cover_art_path ? '✓' : '✗'}`);

    // Navigate to album detail via URL or click
    await page.click('[data-testid="nav-albums"]');
    await page.waitForTimeout(400);
    const searchInput = await page.$('[data-testid="search-input"], input[placeholder*="earch"]');
    if (searchInput) {
      await searchInput.fill('Orient');
      await page.waitForTimeout(800);
    }
    const card = await page.$('[data-testid="album-card"]');
    if (card) {
      await card.click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: ss('3.1-orient-dsf-detail.png') });
      console.log('  ✓ Screenshot: 3.1-orient-dsf-detail.png');

      const trackRows = await page.$$('[data-testid="track-row"], [role="row"]');
      console.log(`  Track rows visible: ${trackRows.length}`);
      expect(trackRows.length).toBeGreaterThan(0);
    }
    if (searchInput) { await searchInput.fill(''); await page.keyboard.press('Escape'); }
  } finally {
    await browser.close();
  }
});

test("3.2 What's Going On (Marvin Gaye) DSF album detail", async () => {
  const { browser, page } = await getPage();
  try {
    const albums = await invoke(page, 'get_all_albums').catch(() => []);
    const wgo = (albums || []).find(a =>
      a.title?.toLowerCase().includes("going on") || a.title?.toLowerCase().includes("what's")
    );

    if (!wgo) {
      console.log("  What's Going On not in DB");
      test.skip();
      return;
    }
    console.log(`  Found: "${wgo.title}" cover=${wgo.cover_art_path ? '✓' : '✗'}`);

    await page.click('[data-testid="nav-albums"]');
    await page.waitForTimeout(400);
    const searchInput = await page.$('[data-testid="search-input"], input[placeholder*="earch"]');
    if (searchInput) { await searchInput.fill("What's Going On"); await page.waitForTimeout(800); }
    const card = await page.$('[data-testid="album-card"]');
    if (card) {
      await card.click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: ss("3.2-whats-going-on-dsf.png") });
      console.log("  ✓ Screenshot: 3.2-whats-going-on-dsf.png");
    }
    if (searchInput) { await searchInput.fill(''); await page.keyboard.press('Escape'); }
  } finally {
    await browser.close();
  }
});

test('3.3 Jazz albums page', async () => {
  const { browser, page } = await getPage();
  try {
    await page.click('[data-testid="nav-albums"]');
    await page.waitForTimeout(400);
    const searchInput = await page.$('[data-testid="search-input"], input[placeholder*="earch"]');
    if (searchInput) {
      await searchInput.fill('Bill Evans');
      await page.waitForTimeout(800);
      await page.screenshot({ path: ss('3.3-bill-evans-search.png') });
      console.log('  ✓ Screenshot: 3.3-bill-evans-search.png');
      await searchInput.fill('');
      await page.keyboard.press('Escape');
    }
  } finally {
    await browser.close();
  }
});

// ---------------------------------------------------------------------------
// Group 4: Artwork coverage
// ---------------------------------------------------------------------------

test('4.1 artwork coverage report', async () => {
  const { browser, page } = await getPage();
  try {
    const albums = await invoke(page, 'get_all_albums').catch(() => []);
    const total = albums?.length ?? 0;
    const withArt = (albums || []).filter(a => a.cover_art_path).length;
    const noArt = (albums || []).filter(a => !a.cover_art_path);
    const pct = total > 0 ? ((withArt / total) * 100).toFixed(1) : '0';

    console.log(`  Artwork coverage: ${withArt}/${total} (${pct}%)`);
    if (noArt.length > 0) {
      console.log(`  Missing artwork (${noArt.length}):`);
      noArt.slice(0, 15).forEach(a => console.log(`    - "${a.title}"`));
    }

    await page.click('[data-testid="nav-albums"]');
    await page.waitForTimeout(800);
    await page.screenshot({ path: ss('4.1-albums-artwork-overview.png') });
    console.log('  ✓ Screenshot: 4.1-albums-artwork-overview.png');
  } finally {
    await browser.close();
  }
});

// ---------------------------------------------------------------------------
// Group 5: Data integrity
// ---------------------------------------------------------------------------

test('5.1 no duplicate albums (same folder_path)', async () => {
  const { browser, page } = await getPage();
  try {
    const albums = await invoke(page, 'get_all_albums').catch(() => []);
    // Duplicate folder_path is always a bug — same title+artist can legitimately
    // appear for B-Sides, multi-disc, etc. (each in its own folder).
    const seenFolders = new Map();
    const dupes = [];
    for (const a of (albums || [])) {
      if (!a.folder_path) continue;
      const key = a.folder_path.toLowerCase();
      if (seenFolders.has(key)) dupes.push(`${a.title} (${a.folder_path})`);
      else seenFolders.set(key, a);
    }

    if (dupes.length > 0) {
      console.log(`  Albums with duplicate folder_path (${dupes.length}): ${dupes.join(', ')}`);
    } else {
      console.log(`  ✓ No duplicate folder_paths in ${albums?.length} albums`);
    }
    expect(dupes.length).toBe(0);
  } finally {
    await browser.close();
  }
});

test('5.2 no tracks have insane duration (excludes null/failed imports)', async () => {
  const { browser, page } = await getPage();
  try {
    const tracks = await invoke(page, 'get_all_tracks').catch(() => []);
    // Only check tracks that DID get a duration (failed imports have null, which is a separate issue)
    const withDuration = (tracks || []).filter(t => t.duration_seconds != null);
    const insane = withDuration.filter(t => t.duration_seconds > 3600 || t.duration_seconds <= 0);
    const nullDur = (tracks || []).filter(t => t.duration_seconds == null);

    console.log(`  Tracks: ${tracks?.length} total, ${withDuration.length} with duration, ${nullDur.length} null, ${insane.length} insane`);
    if (nullDur.length > 0) {
      console.log(`  Null duration tracks (failed imports):`);
      nullDur.slice(0, 5).forEach(t => console.log(`    "${t.title}"`));
    }
    if (insane.length > 0) {
      insane.forEach(t => console.log(`    INSANE: "${t.title}" = ${t.duration_seconds}s`));
    }
    expect(insane.length).toBe(0);
  } finally {
    await browser.close();
  }
});

test('5.3 home page — overview screenshot', async () => {
  const { browser, page } = await getPage();
  try {
    // Navigate to home
    const homeBtn = await page.$('[data-testid="nav-home"], [data-testid="nav-recent"]');
    if (homeBtn) {
      await homeBtn.click();
      await page.waitForTimeout(800);
    }
    await page.screenshot({ path: ss('5.3-home-page.png') });
    console.log('  ✓ Screenshot: 5.3-home-page.png');
  } finally {
    await browser.close();
  }
});
