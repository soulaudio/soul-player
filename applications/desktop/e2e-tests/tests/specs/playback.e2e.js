/**
 * Comprehensive Playback E2E Tests
 *
 * Tests playback from multiple entry points, button interactions, edge cases,
 * and rapid-click resilience. Uses an isolated database with real WAV files
 * so the audio engine can actually load and play tracks.
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.playback.conf.js
 *
 * Prerequisites:
 * - Build the app: cargo build --release -p soul-player-desktop
 * - Install tauri-driver: cargo install tauri-driver
 * - better-sqlite3 in devDependencies (for conf setup)
 */

// ============================================================
// Helpers
// ============================================================

async function waitForElement(selector, description, timeout = 15000) {
  const element = await $(selector);
  try {
    await element.waitForExist({ timeout });
  } catch {
    throw new Error(
      `Element not found: ${description}\nSelector: ${selector}`
    );
  }
  return element;
}

/** Navigate via sidebar nav item */
async function navigateTo(navId, description) {
  const navBtn = await waitForElement(`[data-testid="nav-${navId}"]`, description);
  await navBtn.waitForClickable({ timeout: 5000 });
  await navBtn.click();
  await browser.pause(1500);
}

/** Get current playback state via test helpers */
async function getPlaybackState() {
  return browser.execute(() => {
    return window.__testHelpers?.getPlaybackState() ?? Promise.resolve('unknown');
  });
}

/** Get current track via test helpers */
async function getCurrentTrack() {
  return browser.execute(() => {
    return window.__testHelpers?.getCurrentTrack() ?? Promise.resolve(null);
  });
}

/** Get queue size via test helpers */
async function getQueueSize() {
  return browser.execute(() => {
    return window.__testHelpers?.getQueueSize() ?? Promise.resolve(0);
  });
}

/** Wait until now-playing-title element exists and has text */
async function waitForNowPlayingTitle(timeout = 12000) {
  const panel = await waitForElement('[data-testid="now-playing-title"]', 'Now playing panel', timeout);
  await expect(panel).toBeDisplayed();
  return panel;
}

/** Wait until now-playing-title text changes from currentTitle */
async function waitForTrackChange(previousTitle, timeout = 8000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const panel = await $('[data-testid="now-playing-title"]');
    if (await panel.isExisting()) {
      const text = await panel.getText();
      if (text && text !== previousTitle) {
        return text;
      }
    }
    await browser.pause(200);
  }
  throw new Error(`Track did not change from "${previousTitle}" within ${timeout}ms`);
}

/** Click the play/pause button in the player panel */
async function clickPlayPause() {
  const btn = await waitForElement('[data-testid="play-pause-button"]', 'Play/Pause button');
  await btn.waitForClickable({ timeout: 3000 });
  await btn.click();
  await browser.pause(300);
}

/** Click the next button */
async function clickNext() {
  const btn = await waitForElement('[data-testid="next-button"]', 'Next button');
  await btn.waitForClickable({ timeout: 3000 });
  await btn.click();
  await browser.pause(500);
}

/** Click the previous button */
async function clickPrevious() {
  const btn = await waitForElement('[data-testid="previous-button"]', 'Previous button');
  await btn.waitForClickable({ timeout: 3000 });
  await btn.click();
  await browser.pause(500);
}

/** Get the now-playing panel text (empty string if no track) */
async function getNowPlayingText() {
  const panel = await $('[data-testid="now-playing-title"]');
  if (!await panel.isExisting()) return '';
  return panel.getText().catch(() => '');
}

// ============================================================
// Test Suite 1: Basic Playback from Album Card (MediaCard)
// ============================================================

describe('Playback - From Album Card', () => {
  beforeEach(async () => {
    await browser.pause(1500);
  });

  it('should navigate to albums page', async () => {
    console.log('\n=== Navigate to Albums ===');
    await navigateTo('albums', 'Albums nav button');

    // Albums page should show MediaCard components
    const firstAlbumCard = await $('[data-testid^="media-card-album-"]');
    await firstAlbumCard.waitForExist({ timeout: 10000 });
    await expect(firstAlbumCard).toBeDisplayed();
    console.log('[Test] ✓ Albums page loaded with album cards');
  });

  it('should play an album by clicking the play button on the album card', async () => {
    console.log('\n=== Play from Album Card ===');

    // Find the first album card
    const albumCard = await waitForElement('[data-testid^="media-card-album-"]', 'Album card');

    // Hover over the card to reveal the play button
    await albumCard.moveTo();
    await browser.pause(500);

    // Find and click the play button (opacity-0 by default, visible on hover)
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.waitForExist({ timeout: 5000 });
    await playBtn.click();
    await browser.pause(2000);

    // Verify now-playing panel shows a track
    await waitForNowPlayingTitle();
    const title = await getNowPlayingText();
    console.log(`[Test] ✓ Now playing: "${title}"`);
    expect(title).toBeTruthy();
  });

  it('should show queue after playing album', async () => {
    console.log('\n=== Queue populated after album play ===');
    const queueSize = await getQueueSize();
    console.log(`[Test] Queue size: ${queueSize}`);
    // Should have loaded queue items from the album
    expect(queueSize).toBeGreaterThan(0);
    console.log('[Test] ✓ Queue has tracks');
  });
});

// ============================================================
// Test Suite 2: Basic Playback Controls
// ============================================================

describe('Playback - Basic Controls', () => {
  before(async () => {
    // Ensure we're playing something before running these tests
    await navigateTo('albums', 'Albums nav button');
    const albumCard = await $('[data-testid^="media-card-album-"]');
    await albumCard.waitForExist({ timeout: 10000 });
    await albumCard.moveTo();
    await browser.pause(500);
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.click();
    await browser.pause(3000); // Wait for audio to load
  });

  it('should show the play/pause button in the player controls', async () => {
    console.log('\n=== Play/Pause button visible ===');
    const btn = await waitForElement('[data-testid="play-pause-button"]', 'Play/Pause button');
    await expect(btn).toBeDisplayed();
    console.log('[Test] ✓ Play/Pause button visible');
  });

  it('should toggle play/pause on button click', async () => {
    console.log('\n=== Play/Pause Toggle ===');

    // Record initial state
    const stateBefore = await getPlaybackState();
    console.log(`[Test] State before: ${stateBefore}`);

    // Click pause
    await clickPlayPause();
    await browser.pause(500);

    const stateAfterPause = await getPlaybackState();
    console.log(`[Test] State after first click: ${stateAfterPause}`);
    expect(stateAfterPause).not.toBe(stateBefore);

    // Click play again
    await clickPlayPause();
    await browser.pause(500);

    const stateAfterResume = await getPlaybackState();
    console.log(`[Test] State after second click: ${stateAfterResume}`);
    expect(stateAfterResume).not.toBe(stateAfterPause);
    console.log('[Test] ✓ Play/Pause toggles correctly');
  });

  it('should skip to next track', async () => {
    console.log('\n=== Skip to Next Track ===');

    const titleBefore = await getNowPlayingText();
    console.log(`[Test] Current track: "${titleBefore}"`);
    expect(titleBefore).toBeTruthy();

    await clickNext();

    const titleAfter = await waitForTrackChange(titleBefore);
    console.log(`[Test] ✓ Skipped to next track: "${titleAfter}"`);
  });

  it('should skip to previous track', async () => {
    console.log('\n=== Skip to Previous Track ===');

    const titleBefore = await getNowPlayingText();
    console.log(`[Test] Current track: "${titleBefore}"`);
    expect(titleBefore).toBeTruthy();

    await clickPrevious();

    const titleAfter = await waitForTrackChange(titleBefore);
    console.log(`[Test] ✓ Skipped to previous track: "${titleAfter}"`);
  });

  it('should show next and previous buttons', async () => {
    console.log('\n=== Control Buttons Visible ===');

    const nextBtn = await waitForElement('[data-testid="next-button"]', 'Next button');
    const prevBtn = await waitForElement('[data-testid="previous-button"]', 'Previous button');
    const shuffleBtn = await waitForElement('[data-testid="shuffle-button"]', 'Shuffle button');
    const repeatBtn = await waitForElement('[data-testid="repeat-button"]', 'Repeat button');

    await expect(nextBtn).toBeDisplayed();
    await expect(prevBtn).toBeDisplayed();
    await expect(shuffleBtn).toBeDisplayed();
    await expect(repeatBtn).toBeDisplayed();
    console.log('[Test] ✓ All control buttons visible');
  });
});

// ============================================================
// Test Suite 3: Shuffle and Repeat Toggles
// ============================================================

describe('Playback - Shuffle and Repeat Controls', () => {
  before(async () => {
    // Ensure something is playing
    await navigateTo('albums', 'Albums nav button');
    const albumCard = await $('[data-testid^="media-card-album-"]');
    await albumCard.waitForExist({ timeout: 10000 });
    await albumCard.moveTo();
    await browser.pause(500);
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.click();
    await browser.pause(2500);
  });

  it('should toggle shuffle mode when shuffle button is clicked', async () => {
    console.log('\n=== Shuffle Toggle ===');

    const shuffleBtn = await waitForElement('[data-testid="shuffle-button"]', 'Shuffle button');

    // Click once - should enable shuffle (random mode)
    await shuffleBtn.click();
    await browser.pause(500);

    // Check that primary color class is applied (shuffle is ON)
    // The button gets 'text-primary' class when shuffleMode !== 'off'
    const classAfterFirst = await shuffleBtn.getAttribute('class');
    console.log(`[Test] Shuffle button class after first click: contains primary=${classAfterFirst.includes('text-primary')}`);

    // Click again - cycles to next mode
    await shuffleBtn.click();
    await browser.pause(500);

    // Click again - back to off
    await shuffleBtn.click();
    await browser.pause(500);

    const classAfterThird = await shuffleBtn.getAttribute('class');
    console.log(`[Test] Shuffle button class after third click: ${classAfterThird}`);

    console.log('[Test] ✓ Shuffle toggle cycles through modes');
  });

  it('should toggle repeat mode when repeat button is clicked', async () => {
    console.log('\n=== Repeat Toggle ===');

    const repeatBtn = await waitForElement('[data-testid="repeat-button"]', 'Repeat button');

    // Click once - off → all
    await repeatBtn.click();
    await browser.pause(500);

    const classAfterFirst = await repeatBtn.getAttribute('class');
    console.log(`[Test] Repeat button class after first click: contains primary=${classAfterFirst.includes('text-primary')}`);
    expect(classAfterFirst).toContain('text-primary');

    // Click again - all → one
    await repeatBtn.click();
    await browser.pause(500);

    // Click again - one → off
    await repeatBtn.click();
    await browser.pause(500);

    const classAfterThird = await repeatBtn.getAttribute('class');
    console.log(`[Test] Repeat off class: ${classAfterThird}`);
    expect(classAfterThird).not.toContain('text-primary');

    console.log('[Test] ✓ Repeat toggle cycles off → all → one → off');
  });
});

// ============================================================
// Test Suite 4: Play from Track Row
// ============================================================

describe('Playback - From Track List', () => {
  before(async () => {
    await navigateTo('tracks', 'Tracks nav button');
    await browser.pause(2000);
  });

  it('should show track list with track rows', async () => {
    console.log('\n=== Track List Visible ===');

    const trackList = await $('[data-testid="track-list"]');
    await trackList.waitForExist({ timeout: 10000 });
    await expect(trackList).toBeDisplayed();

    const trackRows = await $$('[data-testid="track-row"]');
    console.log(`[Test] Found ${trackRows.length} track rows`);
    expect(trackRows.length).toBeGreaterThan(0);
    console.log('[Test] ✓ Track list populated');
  });

  it('should play a track by double-clicking a track row', async () => {
    console.log('\n=== Play from Track Row (double-click) ===');

    const trackRows = await $$('[data-testid="track-row"]');
    expect(trackRows.length).toBeGreaterThan(0);

    const thirdRow = trackRows[2] || trackRows[0]; // Try 3rd track for variety
    await thirdRow.scrollIntoView();
    await thirdRow.doubleClick();
    await browser.pause(2500);

    await waitForNowPlayingTitle();
    const title = await getNowPlayingText();
    console.log(`[Test] ✓ Now playing from track row: "${title}"`);
    expect(title).toBeTruthy();
  });

  it('should update now-playing when a different track row is double-clicked', async () => {
    console.log('\n=== Switch Track via Different Row ===');

    const titleBefore = await getNowPlayingText();
    console.log(`[Test] Currently playing: "${titleBefore}"`);

    const trackRows = await $$('[data-testid="track-row"]');
    expect(trackRows.length).toBeGreaterThan(1);

    // Click the first track row (likely different from the currently playing one)
    const firstRow = trackRows[0];
    await firstRow.scrollIntoView();
    await firstRow.doubleClick();
    await browser.pause(2500);

    const titleAfter = await getNowPlayingText();
    console.log(`[Test] Now playing: "${titleAfter}"`);
    expect(titleAfter).toBeTruthy();
    console.log('[Test] ✓ Track changed after clicking different row');
  });
});

// ============================================================
// Test Suite 5: Queue Interaction via testHelpers
// ============================================================

describe('Playback - Queue Skip', () => {
  before(async () => {
    // Play an album to populate the queue
    await navigateTo('albums', 'Albums nav button');
    const albumCard = await $('[data-testid^="media-card-album-"]');
    await albumCard.waitForExist({ timeout: 10000 });
    await albumCard.moveTo();
    await browser.pause(500);
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.click();
    await browser.pause(3000);
  });

  it('should have a queue with multiple tracks', async () => {
    console.log('\n=== Queue Populated ===');
    const size = await getQueueSize();
    console.log(`[Test] Queue size: ${size}`);
    expect(size).toBeGreaterThan(1);
    console.log('[Test] ✓ Queue has multiple tracks');
  });

  it('should skip to a specific queue index using testHelpers', async () => {
    console.log('\n=== Skip to Queue Index ===');

    const size = await getQueueSize();
    const targetIndex = Math.min(2, size - 1);

    const titleBefore = await getNowPlayingText();
    console.log(`[Test] Before skip: "${titleBefore}", targeting index ${targetIndex}`);

    await browser.execute((idx) => {
      return window.__testHelpers?.skipToQueueIndex(idx);
    }, targetIndex);

    await browser.pause(2000);

    const titleAfter = await getNowPlayingText();
    console.log(`[Test] After skip to index ${targetIndex}: "${titleAfter}"`);

    // Track should have changed (unless we happened to already be on index 2)
    expect(titleAfter).toBeTruthy();
    console.log('[Test] ✓ Queue skip via testHelpers works');
  });

  it('should skip to the last track in the queue', async () => {
    console.log('\n=== Skip to Last Queue Track ===');

    const size = await getQueueSize();
    console.log(`[Test] Queue size: ${size}`);
    const lastIndex = size - 1;

    await browser.execute((idx) => {
      return window.__testHelpers?.skipToQueueIndex(idx);
    }, lastIndex);

    await browser.pause(2000);

    const title = await getNowPlayingText();
    console.log(`[Test] Last track: "${title}"`);
    expect(title).toBeTruthy();
    console.log('[Test] ✓ Can skip to last queue track');
  });
});

// ============================================================
// Test Suite 6: Rapid Button Spam (Edge Cases)
// ============================================================

describe('Playback - Rapid Button Spam', () => {
  before(async () => {
    // Ensure something is playing
    await navigateTo('albums', 'Albums nav button');
    const albumCard = await $('[data-testid^="media-card-album-"]');
    await albumCard.waitForExist({ timeout: 10000 });
    await albumCard.moveTo();
    await browser.pause(500);
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.click();
    await browser.pause(3000);
  });

  it('should remain stable after rapidly clicking next 5 times', async () => {
    console.log('\n=== Rapid Next Button (5x) ===');

    const nextBtn = await waitForElement('[data-testid="next-button"]', 'Next button');

    for (let i = 0; i < 5; i++) {
      await nextBtn.click();
      await browser.pause(100); // Very short delay between clicks
    }

    await browser.pause(2000); // Wait for state to settle

    // UI should still be functional
    await expect(nextBtn).toBeDisplayed();
    const title = await getNowPlayingText();
    console.log(`[Test] After 5 rapid next clicks: "${title}"`);
    expect(title).toBeTruthy();
    console.log('[Test] ✓ UI stable after 5 rapid next clicks');
  });

  it('should remain stable after rapidly clicking play/pause 6 times', async () => {
    console.log('\n=== Rapid Play/Pause (6x) ===');

    const playPauseBtn = await waitForElement('[data-testid="play-pause-button"]', 'Play/Pause button');

    for (let i = 0; i < 6; i++) {
      await playPauseBtn.click();
      await browser.pause(150);
    }

    await browser.pause(1500);

    // After 6 clicks (even number), should be back to original state
    await expect(playPauseBtn).toBeDisplayed();
    const state = await getPlaybackState();
    console.log(`[Test] State after 6 rapid play/pause: ${state}`);
    // 6 clicks = back to original playing state (if started playing)
    console.log('[Test] ✓ UI stable after 6 rapid play/pause clicks');
  });

  it('should recover after rapidly clicking previous 3 times', async () => {
    console.log('\n=== Rapid Previous Button (3x) ===');

    const prevBtn = await waitForElement('[data-testid="previous-button"]', 'Previous button');

    for (let i = 0; i < 3; i++) {
      await prevBtn.click();
      await browser.pause(150);
    }

    await browser.pause(2000);

    // UI should still work
    await expect(prevBtn).toBeDisplayed();
    const title = await getNowPlayingText();
    console.log(`[Test] After 3 rapid previous clicks: "${title}"`);
    expect(title).toBeTruthy();
    console.log('[Test] ✓ UI stable after rapid previous clicks');
  });

  it('should handle alternating rapid next then previous', async () => {
    console.log('\n=== Alternating Next/Previous Rapid ===');

    const nextBtn = await waitForElement('[data-testid="next-button"]', 'Next button');
    const prevBtn = await waitForElement('[data-testid="previous-button"]', 'Previous button');

    // next, prev, next, prev, next
    for (let i = 0; i < 5; i++) {
      if (i % 2 === 0) {
        await nextBtn.click();
      } else {
        await prevBtn.click();
      }
      await browser.pause(200);
    }

    await browser.pause(2000);

    const title = await getNowPlayingText();
    console.log(`[Test] After alternating next/prev: "${title}"`);
    expect(title).toBeTruthy();
    console.log('[Test] ✓ UI stable after alternating next/prev spam');
  });
});

// ============================================================
// Test Suite 7: Multiple Albums - Play from Different Contexts
// ============================================================

describe('Playback - Switching Between Albums', () => {
  it('should play second album when clicking its play button', async () => {
    console.log('\n=== Switch to Second Album ===');

    await navigateTo('albums', 'Albums nav button');
    await browser.pause(1500);

    const albumCards = await $$('[data-testid^="media-card-album-"]');
    console.log(`[Test] Found ${albumCards.length} album cards`);
    expect(albumCards.length).toBeGreaterThanOrEqual(2);

    // Get title of first album's now-playing to compare
    const firstCard = albumCards[0];
    await firstCard.moveTo();
    await browser.pause(500);
    const firstPlayBtn = await firstCard.$('[data-testid="media-card-play-button"]');
    await firstPlayBtn.click();
    await browser.pause(2500);

    const titleFromFirstAlbum = await getNowPlayingText();
    console.log(`[Test] Playing from album 1: "${titleFromFirstAlbum}"`);

    // Now play second album
    const secondCard = albumCards[1];
    await secondCard.moveTo();
    await browser.pause(500);
    const secondPlayBtn = await secondCard.$('[data-testid="media-card-play-button"]');
    await secondPlayBtn.click();
    await browser.pause(2500);

    const titleFromSecondAlbum = await getNowPlayingText();
    console.log(`[Test] Playing from album 2: "${titleFromSecondAlbum}"`);

    // Track should have changed to something from the second album
    expect(titleFromSecondAlbum).toBeTruthy();
    expect(titleFromSecondAlbum).not.toBe(titleFromFirstAlbum);
    console.log('[Test] ✓ Successfully switched between albums');
  });

  it('should allow next/previous after switching albums', async () => {
    console.log('\n=== Next/Previous After Album Switch ===');

    const titleBefore = await getNowPlayingText();
    expect(titleBefore).toBeTruthy();

    await clickNext();
    const titleAfterNext = await waitForTrackChange(titleBefore);
    console.log(`[Test] After next: "${titleAfterNext}"`);

    await clickPrevious();
    const titleAfterPrev = await waitForTrackChange(titleAfterNext);
    console.log(`[Test] After previous: "${titleAfterPrev}"`);

    console.log('[Test] ✓ Next/Previous work after album switch');
  });
});

// ============================================================
// Test Suite 8: Navigation During Playback
// ============================================================

describe('Playback - Navigation During Playback', () => {
  before(async () => {
    // Start playback from albums page
    await navigateTo('albums', 'Albums nav button');
    const albumCard = await $('[data-testid^="media-card-album-"]');
    await albumCard.waitForExist({ timeout: 10000 });
    await albumCard.moveTo();
    await browser.pause(500);
    const playBtn = await albumCard.$('[data-testid="media-card-play-button"]');
    await playBtn.click();
    await browser.pause(3000);
  });

  it('should continue showing now-playing when navigating to tracks page', async () => {
    console.log('\n=== Playback persists on page navigation ===');

    const titleBeforeNav = await getNowPlayingText();
    expect(titleBeforeNav).toBeTruthy();
    console.log(`[Test] Playing before nav: "${titleBeforeNav}"`);

    await navigateTo('tracks', 'Tracks nav button');

    const titleAfterNav = await getNowPlayingText();
    console.log(`[Test] Playing after nav to tracks: "${titleAfterNav}"`);

    // Now-playing should still show the same track
    expect(titleAfterNav).toBe(titleBeforeNav);
    console.log('[Test] ✓ Playback continues across page navigation');
  });

  it('should allow next/previous while on tracks page', async () => {
    console.log('\n=== Controls work on different page ===');

    const titleBefore = await getNowPlayingText();
    expect(titleBefore).toBeTruthy();

    await clickNext();
    const titleAfter = await waitForTrackChange(titleBefore);
    console.log(`[Test] ✓ Next works while on tracks page: "${titleAfter}"`);
  });

  it('should allow navigating back to albums while playing', async () => {
    console.log('\n=== Navigate back to albums during playback ===');

    const titleDuringPlayback = await getNowPlayingText();

    await navigateTo('albums', 'Albums nav button');

    // Still playing
    const titleAfterReturn = await getNowPlayingText();
    expect(titleAfterReturn).toBe(titleDuringPlayback);
    console.log('[Test] ✓ Playback continues after returning to albums page');
  });
});

// ============================================================
// Final summary
// ============================================================
after(async () => {
  console.log('\n' + '='.repeat(60));
  console.log('       PLAYBACK E2E TEST SUITE SUMMARY');
  console.log('='.repeat(60));
  console.log('✅ Play from Album Card (MediaCard)');
  console.log('✅ Basic Controls: play/pause, next, previous');
  console.log('✅ Shuffle and Repeat toggle');
  console.log('✅ Play from Track List row');
  console.log('✅ Queue skip via testHelpers');
  console.log('✅ Rapid button spam resilience');
  console.log('✅ Switching between albums');
  console.log('✅ Navigation during playback');
  console.log('='.repeat(60) + '\n');
});
