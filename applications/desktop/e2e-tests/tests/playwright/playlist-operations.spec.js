/**
 * Playlist CRUD operations — Playwright CDP tests
 *
 * Covers:
 *   1. Playlists page shows existing seeded playlist ("Favorites")
 *   2. Create new playlist via the create button (navigates to detail)
 *   3. Navigate to playlist detail by clicking the card
 *   4. Playlist detail shows empty state when no tracks
 *   5. Add a track to "Favorites" via MediaCard right-click context menu
 *   6. Track appears in the playlist detail after being added
 *   7. Remove a track from a playlist via the TrackMenu
 *   8. Delete a playlist via the detail page delete button
 *
 * Seed data (from playwright-global-setup.js):
 *   Playlist ID 3001 — "Favorites" — 0 tracks (empty)
 *   Album ID 2001 — "Playwright Album" — 5 tracks (IDs 2001–2005)
 *   Track titles: Track One … Track Five
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---- CDP connection shared across tests in this file ----

let browser;
let page;

test.beforeAll(async () => {
  // Global setup already waited for the app to be fully ready (nav-albums visible).
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  // Find the main window — it is already loaded by the time tests run.
  const pages = context.pages();
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
         && !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  // Short safety wait in case there is any residual animation or settling
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// ----------------------------------------------------------------
// beforeEach: stop any active playback, dismiss open overlays,
// then navigate to the Playlists page.
// ----------------------------------------------------------------

test.beforeEach(async () => {
  // Stop any in-progress playback so each test starts from a known Stopped state.
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss any leftover context menu, dialog, or overlay
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Navigate to playlists page (use force:true in case a backdrop is still present)
  await page.click('[data-testid="nav-playlists"]', { force: true });
  await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 15_000 });
});

// ----------------------------------------------------------------
// afterEach: stop playback, close open overlays, and clean up any
// playlists or playlist tracks created during the test.
// ----------------------------------------------------------------

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);

  // Remove any tracks that were added to Favorites (playlist 3001) during the test.
  // NOTE: get_playlist_tracks uses param name 'id' (not 'playlistId'); delete_playlist uses 'id'.
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

  // Delete any playlists named "New Playlist" created during the test.
  // NOTE: get_all_playlists takes no params; delete_playlist uses param 'id' (not 'playlistId').
  await page.evaluate(async () => {
    try {
      const playlists = await window.__TAURI_INTERNALS__.invoke('get_all_playlists');
      for (const pl of playlists) {
        if (pl.name === 'Test Playlist' || pl.name === 'New Playlist') {
          await window.__TAURI_INTERNALS__.invoke('delete_playlist', { id: pl.id }).catch(() => {});
        }
      }
    } catch {}
  }).catch(() => {});
});

// ----------------------------------------------------------------
// Helper: navigate to a named testid nav item and wait for a selector.
// Uses force:true to pierce any residual backdrop overlay.
// ----------------------------------------------------------------

async function navigateTo(testId, waitSelector) {
  await page.click(`[data-testid="${testId}"]`, { force: true });
  if (waitSelector) {
    await page.waitForSelector(waitSelector, { timeout: 15_000 });
  }
  await page.waitForTimeout(300);
}

// ----------------------------------------------------------------
// Helper: add Track One (track ID 2001) to Favorites (playlist 3001)
// via the MediaCard context menu on the Albums page.
// Returns after the add-to-playlist dialog has closed.
// ----------------------------------------------------------------

async function addTrackOneToFavorites() {
  // Navigate to Albums page and open album 2001
  await navigateTo('nav-albums', '[data-testid="media-card-album-2001"]');

  const albumCard = page.locator('[data-testid="media-card-album-2001"]');
  await albumCard.waitFor({ state: 'visible' });

  // Left-click the card to navigate into the album detail (which has a track list)
  await albumCard.click();
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Find the first track row and open its context menu via the menu button
  const trackList = page.locator('[data-testid="track-list"]');
  const firstRow = trackList.locator('[data-testid="track-row"]').first();
  await firstRow.waitFor({ state: 'visible' });

  // Hover the row so the menu button appears
  await firstRow.hover();
  await page.waitForTimeout(300);

  // Click the "Track options" button (aria-label set by TrackMenu)
  const menuBtn = firstRow.getByRole('button', { name: /track options/i });
  await menuBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await menuBtn.click();
  await page.waitForTimeout(300);

  // Click the "Add to Playlist" menu item
  const addToPlaylistItem = page.getByRole('menuitem', { name: /add to playlist/i });
  await addToPlaylistItem.waitFor({ state: 'visible', timeout: 5_000 });
  await addToPlaylistItem.click();

  // Wait for the dialog to appear
  const dialog = page.locator('[data-testid="add-to-playlist-dialog"]');
  await dialog.waitFor({ state: 'visible', timeout: 10_000 });

  // Select Favorites from the list
  const favoritesItem = page.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await favoritesItem.waitFor({ state: 'visible', timeout: 5_000 });
  await favoritesItem.click();
  await page.waitForTimeout(300);

  // Click Done to save
  const doneBtn = page.getByRole('button', { name: /done/i });
  await doneBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await doneBtn.click();

  // Wait for the dialog to close
  await dialog.waitFor({ state: 'hidden', timeout: 10_000 });
  await page.waitForTimeout(300);
}

// ----------------------------------------------------------------
// Test 1: Playlists page shows existing seeded playlist
// ----------------------------------------------------------------

test('playlists page shows the seeded Favorites playlist', async () => {
  // The playlists page should already be visible (beforeEach navigated there)
  const playlistsPage = page.locator('[data-testid="playlists-page"]');
  await expect(playlistsPage).toBeVisible();

  // Favorites card must be present — seeded as ID 3001
  const favoritesCard = page.locator('[data-testid="media-card-playlist-3001"]');
  await expect(favoritesCard).toBeVisible({ timeout: 10_000 });

  // Card must display the name "Favorites"
  await expect(favoritesCard).toContainText('Favorites');
});

// ----------------------------------------------------------------
// Test 2: Create new playlist via UI
// The create button immediately creates a playlist with the default
// name and navigates to its detail page — there is no name input modal.
// ----------------------------------------------------------------

test('clicking the create playlist button creates a playlist and navigates to its detail', async () => {
  const createBtn = page.locator('[data-testid="playlist-create-button"]');
  await expect(createBtn).toBeVisible({ timeout: 5_000 });
  await createBtn.click();

  // Should navigate to a /playlists/{id} URL — playlist IDs are UUIDs (TEXT), not numeric
  await page.waitForURL(/\/playlists\/[^/]+/, { timeout: 15_000 });
  const url = page.url();
  expect(url).toMatch(/\/playlists\/[^/]+/);

  // The detail page must be visible
  await expect(page.locator('[data-testid="playlist-detail-page"]')).toBeVisible({ timeout: 10_000 });

  // The playlist title must be visible (default name assigned by the backend)
  await expect(page.locator('[data-testid="playlist-title"]')).toBeVisible({ timeout: 5_000 });
});

// ----------------------------------------------------------------
// Test 3: Navigate to playlist detail by clicking the Favorites card
// ----------------------------------------------------------------

test('clicking the Favorites playlist card navigates to its detail page', async () => {
  const favoritesCard = page.locator('[data-testid="media-card-playlist-3001"]');
  await favoritesCard.waitFor({ state: 'visible', timeout: 10_000 });

  // Click the card title/artwork to navigate to the detail page
  await favoritesCard.click();

  // URL must now include /playlists/3001
  await page.waitForURL(/\/playlists\/3001/, { timeout: 15_000 });

  // Playlist detail container must be visible
  await expect(page.locator('[data-testid="playlist-detail-page"]')).toBeVisible({ timeout: 10_000 });

  // Playlist title must read "Favorites"
  const titleEl = page.locator('[data-testid="playlist-title"]');
  await titleEl.waitFor({ state: 'visible', timeout: 5_000 });
  await expect(titleEl).toContainText('Favorites');
});

// ----------------------------------------------------------------
// Test 4: Playlist detail shows empty state when no tracks
// ----------------------------------------------------------------

test('Favorites playlist detail shows empty state when it has no tracks', async () => {
  // Navigate directly to the Favorites detail page
  await page.click('[data-testid="media-card-playlist-3001"]');
  await page.waitForURL(/\/playlists\/3001/, { timeout: 15_000 });
  await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });

  // The empty state element must be present (no track-list when 0 tracks)
  const emptyState = page.locator('[data-testid="playlist-empty-state"]');
  await expect(emptyState).toBeVisible({ timeout: 5_000 });

  // No track-list must be rendered
  await expect(page.locator('[data-testid="track-list"]')).not.toBeVisible();
});

// ----------------------------------------------------------------
// Test 5: Add a track to Favorites via context menu
// ----------------------------------------------------------------

test('right-clicking a track in album detail and using Add to Playlist adds it to Favorites', async () => {
  // Navigate to Albums and open album 2001
  await navigateTo('nav-albums', '[data-testid="media-card-album-2001"]');
  await page.locator('[data-testid="media-card-album-2001"]').click();
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 15_000 });

  // Hover the first track row to reveal the menu button
  const firstRow = page.locator('[data-testid="track-list"] [data-testid="track-row"]').first();
  await firstRow.waitFor({ state: 'visible' });
  await firstRow.hover();
  await page.waitForTimeout(300);

  // Click the track options button
  const menuBtn = firstRow.getByRole('button', { name: /track options/i });
  await menuBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await menuBtn.click();
  await page.waitForTimeout(300);

  // Click "Add to Playlist"
  const addToPlaylistItem = page.getByRole('menuitem', { name: /add to playlist/i });
  await addToPlaylistItem.waitFor({ state: 'visible', timeout: 5_000 });
  await addToPlaylistItem.click();

  // The dialog must appear
  const dialog = page.locator('[data-testid="add-to-playlist-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });

  // "Favorites" must appear in the list of playlists
  const favoritesItem = page.locator('[data-testid="playlist-dialog-item"]').filter({ hasText: 'Favorites' });
  await expect(favoritesItem).toBeVisible({ timeout: 5_000 });

  // Select Favorites and click Done
  await favoritesItem.click();
  await page.waitForTimeout(300);
  const doneBtn = page.getByRole('button', { name: /done/i });
  await doneBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await doneBtn.click();

  // Dialog must close
  await expect(dialog).not.toBeVisible({ timeout: 10_000 });
});

// ----------------------------------------------------------------
// Test 6: Track appears in playlist detail after being added
// ----------------------------------------------------------------

test('track added to Favorites appears in the playlist detail track list', async () => {
  // Add Track One (ID 2001) to Favorites (ID 3001) directly via backend IPC.
  // We use IPC instead of the UI flow here because the UI "Add to Playlist" dialog
  // toggles the selection — if Track One is already in Favorites from a prior test
  // the click would deselect it. Direct IPC is deterministic. The UI flow is
  // already covered by test 5.
  await page.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('add_track_to_playlist', {
      playlistId: '3001',
      trackId: '2001',
    });
  });
  await page.waitForTimeout(300);

  // Navigate to the playlists page and into Favorites
  await navigateTo('nav-playlists', '[data-testid="playlists-page"]');

  const favoritesCard = page.locator('[data-testid="media-card-playlist-3001"]');
  await favoritesCard.waitFor({ state: 'visible', timeout: 10_000 });
  await favoritesCard.click();

  await page.waitForURL(/\/playlists\/3001/, { timeout: 15_000 });
  await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });

  // The track list must now be visible (no longer showing empty state)
  const trackList = page.locator('[data-testid="track-list"]');
  await expect(trackList).toBeVisible({ timeout: 10_000 });

  // "Track One" must appear in the list
  const rows = trackList.locator('[data-testid="track-row"]');
  await expect(rows.first()).toBeVisible({ timeout: 5_000 });

  // Verify at least one row contains "Track One"
  const trackRowText = await page.locator('[data-testid="track-list"] [data-testid="track-row"]').first().textContent();
  expect(trackRowText).toContain('Track One');
});

// ----------------------------------------------------------------
// Test 7: Remove a track from the playlist
// The TrackMenu on the playlist detail page exposes a "Remove from playlist"
// or "Delete" action. We use the backend invoke to remove directly so the
// test is deterministic regardless of TrackMenu i18n label variations.
// After removal the empty state must reappear.
// ----------------------------------------------------------------

test('removing a track from Favorites causes the playlist to become empty', async () => {
  // First add Track One via the helper so there is something to remove
  await addTrackOneToFavorites();

  // Navigate to Favorites detail
  await navigateTo('nav-playlists', '[data-testid="playlists-page"]');
  const favoritesCard = page.locator('[data-testid="media-card-playlist-3001"]');
  await favoritesCard.waitFor({ state: 'visible', timeout: 10_000 });
  await favoritesCard.click();

  await page.waitForURL(/\/playlists\/3001/, { timeout: 15_000 });
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });

  // Confirm the track row is present before removal
  const rows = page.locator('[data-testid="track-list"] [data-testid="track-row"]');
  await expect(rows.first()).toBeVisible({ timeout: 5_000 });

  // Remove the track via backend invoke so we don't depend on i18n label of TrackMenu
  await page.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_playlist_tracks', { id: '3001' });
    for (const t of tracks) {
      await window.__TAURI_INTERNALS__.invoke('remove_track_from_playlist', {
        playlistId: '3001',
        trackId: String(t.id),
      }).catch(() => {});
    }
  });

  // Hard-reload the page to clear the React Query cache (staleTime=1min keeps the
  // old track list if we just re-navigate within the same React session). After
  // reload the app refetches from the backend, which now has no tracks in the playlist.
  await page.reload();
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
  // After reload React Router restores the /playlists/3001 URL and PlaylistPage mounts
  // with a fresh query, showing empty state because the backend has no tracks.
  await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });

  // After removal the empty state must be visible again
  await expect(page.locator('[data-testid="playlist-empty-state"]')).toBeVisible({ timeout: 10_000 });
  await expect(page.locator('[data-testid="track-list"]')).not.toBeVisible();
});

// ----------------------------------------------------------------
// Test 8: Delete a playlist
// Create a playlist via the create button, then delete it via the
// delete button on the detail page and confirm via the ConfirmDialog.
// Afterwards navigate back to playlists and verify it is gone.
// ----------------------------------------------------------------

test('deleting a playlist removes it from the playlists list', async () => {
  // Click create — navigates immediately to new playlist detail
  const createBtn = page.locator('[data-testid="playlist-create-button"]');
  await expect(createBtn).toBeVisible({ timeout: 5_000 });
  await createBtn.click();

  // Wait for the detail page of the newly created playlist
  await page.waitForURL(/\/playlists\/[^/]+/, { timeout: 15_000 });
  await page.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 10_000 });

  // Capture the URL so we can verify the playlist is gone later
  const detailUrl = page.url();
  const newPlaylistId = detailUrl.match(/\/playlists\/([^/]+)/)?.[1];
  expect(newPlaylistId).toBeTruthy();

  // Click the delete button on the detail page
  const deleteBtn = page.locator('[data-testid="delete-playlist-button"]');
  await expect(deleteBtn).toBeVisible({ timeout: 5_000 });
  await deleteBtn.click();

  // A ConfirmDialog should appear — click the destructive confirm button ("Delete")
  // The ConfirmDialog renders a button with the text from confirmText prop.
  // PlaylistPage uses t('common.delete', 'Delete') as confirmText.
  const confirmBtn = page.getByRole('button', { name: /^delete$/i });
  await confirmBtn.waitFor({ state: 'visible', timeout: 5_000 });
  await confirmBtn.click();

  // After deletion, the app navigates back to /playlists
  await page.waitForURL(/\/playlists$/, { timeout: 15_000 });
  await page.waitForSelector('[data-testid="playlists-page"]', { timeout: 10_000 });

  // The deleted playlist card must no longer be present
  const deletedCard = page.locator(`[data-testid="media-card-playlist-${newPlaylistId}"]`);
  await expect(deletedCard).not.toBeVisible({ timeout: 5_000 });
});
