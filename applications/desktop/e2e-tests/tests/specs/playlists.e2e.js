/**
 * Playlists E2E Tests
 *
 * Covers: create, add track via TrackMenu, navigate to detail, play from card,
 * and add album via card button.
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

  it('should open the track options menu for the first track', async () => {
    const trackList = await waitForEl('[data-testid="track-list"]', 'track list');
    const firstRow = await trackList.$('[data-testid="track-row"]');
    await firstRow.waitForExist({ timeout: 10000, timeoutMsg: 'First track row not found' });

    await hoverElement(firstRow);

    const optionsBtn = await firstRow.$('[aria-label="Track options"]');
    await optionsBtn.waitForExist({ timeout: 8000, timeoutMsg: 'Track options button not found' });
    await optionsBtn.click();

    // Find and click the menu item containing 'Playlist'
    await browser.waitUntil(
      async () => {
        const menuItems = await $$('[role="menuitem"]');
        for (const item of menuItems) {
          const text = await item.getText();
          if (text.includes('Playlist')) {
            await item.click();
            return true;
          }
        }
        return false;
      },
      {
        timeout: 8000,
        timeoutMsg: 'Could not find a menu item containing "Playlist"',
      }
    );
  });

  it('should display the add-to-playlist dialog', async () => {
    const dialog = await waitForEl('[data-testid="add-to-playlist-dialog"]', 'add-to-playlist dialog');
    expect(await dialog.isDisplayed()).toBe(true);
  });

  it('should list playlists in the dialog including Favorites', async () => {
    const dialogItems = await $$('[data-testid="playlist-dialog-item"]');
    expect(dialogItems.length).toBeGreaterThan(0);

    const firstItemText = await dialogItems[0].getText();
    expect(firstItemText).toContain('Favorites');
  });

  it('should add the track to the playlist and close the dialog', async () => {
    const dialogItems = await $$('[data-testid="playlist-dialog-item"]');
    await dialogItems[0].click();

    const doneBtn = await $('button=Done');
    await doneBtn.waitForExist({ timeout: 8000, timeoutMsg: 'Done button not found' });
    await doneBtn.click();

    await browser.waitUntil(
      async () => {
        const dialog = await $('[data-testid="add-to-playlist-dialog"]');
        return !(await dialog.isExisting());
      },
      {
        timeout: 8000,
        timeoutMsg: 'Expected add-to-playlist dialog to close after clicking Done',
      }
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

  it('should play the first playlist by clicking the play button on its card', async () => {
    // Wait for at least one playlist card to appear
    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="media-card-playlist-"]');
        return cards.length > 0;
      },
      {
        timeout: 10000,
        timeoutMsg: 'No playlist cards found on the playlists page',
      }
    );

    const cards = await $$('[data-testid^="media-card-playlist-"]');
    const firstCard = cards[0];

    await hoverElement(firstCard);

    const playBtn = await firstCard.$('[data-testid="media-card-play-button"]');
    await playBtn.waitForExist({ timeout: 8000, timeoutMsg: 'Play button not found on playlist card' });
    await playBtn.click();

    await waitForNowPlaying();
  });
});

// ---- Suite 5: Playlist Add Album via Card Button ----

describe('Playlist: Add album via card button', () => {
  before(async () => {
    await navigateTo('nav-albums');
  });

  it('should add the album to a playlist via the card add-to-playlist button', async () => {
    const albumCard = await waitForEl('[data-testid="media-card-album-2001"]', 'album card 2001');
    await hoverElement(albumCard);

    const addBtn = await albumCard.$('[data-testid="media-card-add-to-playlist-button"]');
    await addBtn.waitForExist({ timeout: 8000, timeoutMsg: 'Add-to-playlist button not found on album card' });
    await addBtn.click();

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
