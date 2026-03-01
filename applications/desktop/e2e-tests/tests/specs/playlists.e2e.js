/**
 * Playlists E2E Tests
 *
 * Covers: create, add track via TrackMenu, navigate to detail, play from card,
 * and add album via card right-click.
 *
 * Requires: wdio.playlists.conf.js (seeds 1 artist, 1 album, 5 tracks, 1 playlist)
 */

// ---- Helpers ----

async function waitForEl(selector, desc, timeout = 15000) {
  const el = await $(selector);
  await el.waitForExist({ timeout, timeoutMsg: `Expected "${desc}" (${selector}) to exist within ${timeout}ms` });
  await el.waitForDisplayed({ timeout, timeoutMsg: `Expected "${desc}" (${selector}) to be displayed within ${timeout}ms` });
  return el;
}

async function navigateTo(navId) {
  const navEl = await $(`[data-testid="${navId}"]`);
  await navEl.waitForExist({ timeout: 10000, timeoutMsg: `Nav item "${navId}" not found` });
  await navEl.click();
  await browser.pause(800);
}

async function hoverElement(el) {
  await el.moveTo();
  await browser.pause(300);
}

async function waitForNowPlaying(timeout = 12000) {
  const el = await $('[data-testid="now-playing-title"]');
  await el.waitForExist({ timeout, timeoutMsg: `Now playing title did not appear within ${timeout}ms` });
  await el.waitForDisplayed({ timeout, timeoutMsg: `Now playing title not displayed within ${timeout}ms` });
  return el;
}

// ---- Suite 1: Playlist Create ----

describe('Playlist: Create', () => {
  it('should navigate to playlists tab and create a new playlist', async () => {
    await navigateTo('nav-playlists');

    const createBtn = await waitForEl('[data-testid="playlist-create-button"]', 'playlist create button');
    await createBtn.click();

    // Wait for navigation to a new playlist URL
    await browser.waitUntil(
      async () => {
        const url = await browser.getUrl();
        return url.includes('/playlists/');
      },
      {
        timeout: 10000,
        timeoutMsg: 'Expected URL to contain /playlists/ after creating a playlist',
      }
    );

    const url = await browser.getUrl();
    expect(url).toContain('/playlists/');
  });
});

// ---- Suite 2: Playlist Add Track via TrackMenu ----

describe('Playlist: Add track via TrackMenu', () => {
  before(async () => {
    await navigateTo('nav-albums');
    const albumCard = await waitForEl('[data-testid="media-card-album-2001"]', 'album card 2001');
    await albumCard.click();
    await browser.pause(1500);
  });

  it('adds a track to the Favorites playlist via the track context menu', async () => {
    // Step 1: hover first track row and open menu
    const trackList = await waitForEl('[data-testid="track-list"]', 'track list');
    const firstRow = await trackList.$('[data-testid="track-row"]');
    await hoverElement(firstRow);

    const menuBtn = await firstRow.$('[aria-label="Track options"]');
    await menuBtn.waitForExist({ timeout: 5000 });
    await menuBtn.click();
    await browser.pause(500);

    // Step 2: find and click "Add to Playlist" menu item
    const menuItems = await $$('[role="menuitem"]');
    let addToPlaylistItem = null;
    for (const item of menuItems) {
      const text = await item.getText();
      if (text.includes('Playlist')) { addToPlaylistItem = item; break; }
    }
    expect(addToPlaylistItem).toBeTruthy();
    await addToPlaylistItem.click();
    await browser.pause(500);

    // Step 3: assert dialog is open
    const dialog = await waitForEl('[data-testid="add-to-playlist-dialog"]', 'Add to Playlist dialog');
    await expect(dialog).toBeDisplayed();

    // Step 4: assert Favorites is listed
    const items = await $$('[data-testid="playlist-dialog-item"]');
    expect(items.length).toBeGreaterThanOrEqual(1);
    const firstText = await items[0].getText();
    expect(firstText).toContain('Favorites');

    // Step 5: select Favorites and save
    await items[0].click();
    await browser.pause(300);
    const doneBtn = await $('button=Done');
    await doneBtn.waitForClickable({ timeout: 3000 });
    await doneBtn.click();
    await browser.waitUntil(
      async () => !(await $('[data-testid="add-to-playlist-dialog"]').isExisting()),
      { timeout: 5000, timeoutMsg: 'Dialog did not close after clicking Done' }
    );
  });
});

// ---- Suite 3: Playlist Navigate to Detail ----

describe('Playlist: Navigate to detail', () => {
  it('should navigate to the Favorites playlist and show its tracks', async () => {
    await navigateTo('nav-playlists');

    // Find the playlist card for 'Favorites'
    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="media-card-playlist-"]');
        for (const card of cards) {
          const text = await card.getText();
          if (text.includes('Favorites')) {
            await card.click();
            return true;
          }
        }
        return false;
      },
      {
        timeout: 10000,
        timeoutMsg: 'Could not find a playlist card containing "Favorites"',
      }
    );

    await browser.pause(1500);

    const trackList = await waitForEl('[data-testid="track-list"]', 'track list');
    const trackRows = await trackList.$$('[data-testid="track-row"]');
    expect(trackRows.length).toBeGreaterThanOrEqual(1);
  });
});

// ---- Suite 4: Playlist Play from Card ----

describe('Playlist: Play from card', () => {
  before(async () => {
    await navigateTo('nav-playlists');
  });

  it('plays the Favorites playlist from its card hover play button', async () => {
    // Find the Favorites card by name (it's the only seeded playlist with tracks)
    let favoritesCard = null;
    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="media-card-playlist-"]');
        for (const card of cards) {
          const text = await card.getText();
          if (text.includes('Favorites')) { favoritesCard = card; return true; }
        }
        return false;
      },
      { timeout: 10000, timeoutMsg: 'Could not find Favorites playlist card' }
    );
    await hoverElement(favoritesCard);
    const playBtn = await favoritesCard.$('[data-testid="media-card-play-button"]');
    await playBtn.waitForClickable({ timeout: 5000 });
    await playBtn.click();
    const nowPlaying = await waitForNowPlaying(12000);
    await expect(nowPlaying).toBeDisplayed();
  });
});

// ---- Suite 5: Playlist Add Album via Card Right-Click ----

describe('Playlist: Add album via card right-click', () => {
  before(async () => {
    await navigateTo('nav-albums');
  });

  it('should add the album to a playlist via right-clicking the album card', async () => {
    const albumCard = await waitForEl('[data-testid="media-card-album-2001"]', 'album card 2001');

    // Right-click the card artwork to open the Add to Playlist dialog
    await albumCard.click({ button: 'right' });
    await browser.pause(500);

    // Dialog should appear
    const dialog = await waitForEl('[data-testid="add-to-playlist-dialog"]', 'add-to-playlist dialog');
    expect(await dialog.isDisplayed()).toBe(true);

    // Click the first dialog item
    const dialogItems = await $$('[data-testid="playlist-dialog-item"]');
    expect(dialogItems.length).toBeGreaterThan(0);
    await dialogItems[0].click();

    // Click Done
    const doneBtn = await $('button=Done');
    await doneBtn.waitForExist({ timeout: 8000, timeoutMsg: 'Done button not found' });
    await doneBtn.click();

    // Dialog should close
    await browser.waitUntil(
      async () => {
        const d = await $('[data-testid="add-to-playlist-dialog"]');
        return !(await d.isExisting());
      },
      {
        timeout: 8000,
        timeoutMsg: 'Expected add-to-playlist dialog to close after clicking Done',
      }
    );
  });
});
