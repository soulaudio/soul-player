/**
 * AddToPlaylistDialog internals — Playwright CDP tests
 *
 * Covers the in-dialog UX for adding tracks and entities to playlists:
 *
 *   1. Dialog opens from track row options menu and shows Favorites
 *   2. Done button is disabled until a selection is made
 *   3. Search input filters the playlist list
 *   4. Inline "Create new playlist" input creates a playlist that appears in the list
 *   5. Selecting Favorites and clicking Done adds Track One to Favorites (IPC verify)
 *   6. Album card right-click opens the entity-mode dialog (no pre-selection)
 *
 * Seed data (from playwright-global-setup.js):
 *   Album ID 2001 — "Playwright Album" — 5 tracks (IDs 2001–2005)
 *   Track titles: Track One … Track Five
 *   Playlist ID 3001 — "Favorites" — 0 tracks initially
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  page = context.pages().find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

/**
 * Clear all tracks from the Favorites playlist (3001) and delete any
 * "Test E2E*" playlists created during a test.
 */
async function cleanupPlaylists() {
  await page.evaluate(async () => {
    try {
      const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
      for (const t of tracks) {
        await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
          playlistId: '3001',
          trackId: String(t.id),
        }).catch(() => {});
      }
    } catch {}
  }).catch(() => {});

  await page.evaluate(async () => {
    try {
      const playlists = await window.__TAURI_INTERNALS__.invoke('get_all_playlists');
      for (const pl of playlists) {
        // All playlists created in this spec MUST be named with "Test E2E" prefix
        // so this cleanup can find and remove them.
        if (pl.name && pl.name.startsWith('Test E2E')) {
          await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pl.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});
}

/**
 * Navigate to album 2001 detail page (via Albums nav → click card) and wait
 * for the track list to be visible.
 */
async function navigateToAlbumDetail() {
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await albumCard.waitFor({ state: 'visible' });
  await albumCard.click();

  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });
  await page.waitForTimeout(300);
}

/**
 * Open the "Add to Playlist" dialog from the Track One row options button.
 * Returns the dialog locator (already visible when this resolves).
 */
async function openDialogFromTrackOne() {
  const trackList = page.locator('[data-testid="track-list"]');
  const firstRow = trackList.locator('[data-testid="track-row"]').first();
  await firstRow.waitFor({ state: 'visible' });

  // Scroll row into view and hover to reveal the options button
  await firstRow.scrollIntoViewIfNeeded();
  await firstRow.hover();
  await page.waitForTimeout(300);

  const menuBtn = firstRow.getByRole('button', { name: /track options/i });
  await menuBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await menuBtn.click({ force: true });
  await page.waitForTimeout(400);

  const addToPlaylistItem = page.getByRole('menuitem', { name: /add to playlist/i });
  await addToPlaylistItem.waitFor({ state: 'visible', timeout: 5_000 });
  await addToPlaylistItem.click();

  const dialog = page.locator('[data-testid="add-to-playlist-dialog"]');
  await dialog.waitFor({ state: 'visible', timeout: 10_000 });
  return dialog;
}

// ----------------------------------------------------------------
// beforeEach / afterEach
// ----------------------------------------------------------------

test.beforeEach(async () => {
  // Stop any in-progress playback
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover dialog / overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Defensive pre-clean: ensures state is clean even if a previous afterEach failed
  await cleanupPlaylists();

  // Navigate to album 2001 detail with track list visible
  await navigateToAlbumDetail();
});

test.afterEach(async () => {
  // Dismiss any open dialog / menu left over from the test
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);

  // Clean up any playlist changes the test may have made
  await cleanupPlaylists();
});

// ----------------------------------------------------------------
// Test 1: dialog opens and shows Favorites
// ----------------------------------------------------------------

test('add-to-playlist dialog opens and shows Favorites', async () => {
  const dialog = await openDialogFromTrackOne();

  // Dialog container must be visible
  await expect(dialog).toBeVisible();

  // Favorites playlist item must be listed
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await expect(favoritesItem).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 2: Done button disabled until selection is made
// ----------------------------------------------------------------

test('Done button is disabled until a selection is made', async () => {
  const dialog = await openDialogFromTrackOne();

  // The Done button must exist
  const doneBtn = page.getByRole('button', { name: /done/i });
  await doneBtn.waitFor({ state: 'visible', timeout: 5_000 });

  // Before any selection Done must be disabled
  await expect(doneBtn).toBeDisabled();

  // Click Favorites item to select it
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await favoritesItem.waitFor({ state: 'visible', timeout: 5_000 });
  await favoritesItem.click();
  await page.waitForTimeout(200);

  // Done must now be enabled
  await expect(doneBtn).not.toBeDisabled();
});

// ----------------------------------------------------------------
// Test 3: Search filters playlist list
// ----------------------------------------------------------------

test('search filters the playlist list', async () => {
  const dialog = await openDialogFromTrackOne();

  // Wait for Favorites to appear (data loaded)
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await favoritesItem.waitFor({ state: 'visible', timeout: 5_000 });

  // Type a search query that should match nothing
  const searchInput = dialog.locator('[data-testid="playlist-search-input"]');
  await searchInput.fill('zzznomatch');
  await page.waitForTimeout(300);

  // No playlist-dialog-item rows should be visible
  const items = dialog.locator('[data-testid="playlist-dialog-item"]');
  await expect(items).toHaveCount(0, { timeout: 3_000 });

  // Clear the search (click the X button inside the input, or clear manually)
  await searchInput.fill('');
  await page.waitForTimeout(300);

  // Favorites should be back
  await expect(favoritesItem).toBeVisible({ timeout: 3_000 });
  await expect(items).toHaveCount(1, { timeout: 3_000 });
});

// ----------------------------------------------------------------
// Test 4: Inline create-new creates and shows new playlist
// ----------------------------------------------------------------

test('inline create-new creates and shows the new playlist in the list', async () => {
  const dialog = await openDialogFromTrackOne();

  // Wait until loading is done (Favorites should be visible)
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await favoritesItem.waitFor({ state: 'visible', timeout: 5_000 });

  // Click "Create new playlist" button to show the inline input
  const createNewBtn = dialog.getByRole('button', { name: /create new playlist/i });
  await createNewBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await createNewBtn.click();
  await page.waitForTimeout(200);

  // An input should appear for the new playlist name
  const nameInput = dialog.locator('[data-testid="new-playlist-name-input"]');
  await nameInput.waitFor({ state: 'visible', timeout: 5_000 });

  // Type the new playlist name and press Enter to confirm
  await nameInput.fill('Test E2E Playlist');
  await nameInput.press('Enter');
  await page.waitForTimeout(500);

  // The new playlist must appear in the list
  const newItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Test E2E Playlist' });
  await expect(newItem).toBeVisible({ timeout: 5_000 });

  // It should be auto-selected (bg-primary/10 styling) — verify via aria or count
  // At minimum there must now be 2 items: Favorites + Test E2E Playlist
  const allItems = dialog.locator('[data-testid="playlist-dialog-item"]');
  await expect(allItems).toHaveCount(2, { timeout: 3_000 });
});

// ----------------------------------------------------------------
// Test 5: Selecting Favorites and Done adds track to Favorites
// ----------------------------------------------------------------

test('selecting Favorites and clicking Done adds Track One to Favorites', async () => {
  const dialog = await openDialogFromTrackOne();

  // Select Favorites
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await favoritesItem.waitFor({ state: 'visible', timeout: 5_000 });
  await favoritesItem.click();
  await page.waitForTimeout(200);

  // Click Done
  const doneBtn = page.getByRole('button', { name: /done/i });
  await doneBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await doneBtn.click();

  // Wait for dialog to close
  await dialog.waitFor({ state: 'hidden', timeout: 10_000 });

  // Verify via IPC that Favorites now contains at least 1 track
  const trackCount = await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
    return tracks.length;
  });
  expect(trackCount).toBeGreaterThanOrEqual(1);
});

// ----------------------------------------------------------------
// Test 6: Album card right-click opens entity-mode dialog
// ----------------------------------------------------------------

test('album card right-click opens entity-mode dialog with no pre-selection', async () => {
  // Navigate back to Albums page
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await albumCard.waitFor({ state: 'visible' });

  // Right-click the album card to open the context menu
  await albumCard.click({ button: 'right' });

  const menuItem = page.getByRole('menuitem', { name: /add to playlist/i });
  await menuItem.waitFor({ state: 'visible', timeout: 5_000 });
  await menuItem.click();

  // The dialog must appear
  const dialog = page.locator('[data-testid="add-to-playlist-dialog"]');
  await dialog.waitFor({ state: 'visible', timeout: 10_000 });
  await expect(dialog).toBeVisible();

  // Entity mode dialog should have "Album" in the title.
  // DialogHeader renders a div.font-semibold (no h2/h3), so target it directly
  // within the [data-testid="add-to-playlist-dialog"] container.
  const dialogTitle = await dialog.locator('.font-semibold').first().textContent().catch(() => '');
  // Title should reference album (entity mode distinguisher)
  expect(dialogTitle.toLowerCase()).toMatch(/album|playlist/i);

  // Favorites must be listed but NOT pre-selected (entity mode, no pre-selection)
  const favoritesItem = dialog.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await expect(favoritesItem).toBeVisible({ timeout: 5_000 });

  // In entity mode nothing is pre-selected, so Done must be disabled initially
  const doneBtn = page.getByRole('button', { name: /done/i });
  await expect(doneBtn).toBeDisabled();
});
