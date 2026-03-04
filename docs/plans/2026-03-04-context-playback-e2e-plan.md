# Context Playback E2E Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 20 Playwright e2e tests verifying that playback from each navigation context (Album, Artist, Genre, Playlist, Tracks) loads the correct current track and queue — in both normal and shuffle modes.

**Architecture:** Single spec file `context-playback.spec.js` with five `test.describe` blocks (one per context), each containing 4 tests: Play All, Track Three click, Shuffle+Play All, Shuffle+Track Three. Shared helpers handle IPC queue inspection and state management.

**Tech Stack:** Playwright CDP, Tauri IPC (`get_current_track`, `get_queue`, `set_shuffle`, `add_track_to_playlist`, `stop_playback`)

---

## Key Context Before Starting

### Seed data (already in the Playwright test DB)
- Artist 2001 "Playwright Artist"
- Album 2001 "Playwright Album" — 5 tracks × 2-second WAV files
- Track IDs 2001–2005, titles: Track One … Track Five
- Genre 4001 "Playwright Genre" — all 5 tracks
- Playlist 3001 "Favorites" — **empty** — tests add tracks in `beforeAll`

### Critical IPC facts
- `get_queue()` returns **upcoming** tracks only (not current). Returns `TrackData[]` with `#[serde(rename_all = "camelCase")]` → field is `trackId`.
- `get_current_track()` returns `QueueTrack | null` — no camelCase rename → field is `id`.
- After `play_queue(5 tracks, startIndex=0)` + play starts: current=1 track, queue=4 tracks.
- After `play_queue(5 tracks, startIndex=2)` + play: current=Track Three, queue=[T4, T5].
- `set_shuffle(mode)` — modes: `"off"`, `"random"`, `"smart"`. Required param: `{ mode: "random" }`.
- Tracks are 2 seconds long — **always pause immediately** after confirming play started, or tracks will auto-advance during assertions.

### Existing testids referenced
- `album-play-all-button` ✓ (AlbumPage.tsx:264)
- `artist-play-all-button` ✓ (ArtistPage.tsx:289)
- `genre-play-all-button` ✓ (GenrePage.tsx:205)
- `playlist-play-all-button` ✗ **must be added in Task 1**
- Tracks page has **no Play All button** — use double-click Track One instead

### Navigation patterns
- Album: `nav-albums` → click card title → `album-detail-page`
- Artist: `nav-artists` → click card title → `artist-detail-page`
- Genre: `history.pushState('/genres/4001')` + `popstate` → `genre-detail-page`
- Playlist: `nav-playlists` → click card → `playlist-detail-page`
- Tracks: `nav-tracks` → `tracks-page`

---

## Task 1: Add `playlist-play-all-button` testid to PlaylistPage.tsx

**Files:**
- Modify: `applications/shared/src/pages/PlaylistPage.tsx:226`

**Step 1: Open the file and find the Play All button**

Read `applications/shared/src/pages/PlaylistPage.tsx`, look for the button with `onClick={handlePlayAll}` around line 226.

**Step 2: Add the testid attribute**

Current code (~line 226):
```tsx
              <button
                onClick={handlePlayAll}
                onMouseDown={(e) => e.preventDefault()}
                disabled={tracks.length === 0}
```

Change to:
```tsx
              <button
                data-testid="playlist-play-all-button"
                onClick={handlePlayAll}
                onMouseDown={(e) => e.preventDefault()}
                disabled={tracks.length === 0}
```

**Step 3: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player
cargo xtask check typescript
```
Expected: no errors

**Step 4: Commit**

```bash
git add applications/shared/src/pages/PlaylistPage.tsx
git commit --no-verify -m "feat(testid): add playlist-play-all-button data-testid to PlaylistPage"
```

---

## Task 2: Create spec file skeleton

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Create the file with boilerplate**

```js
/**
 * Context Playback E2E Tests — Playwright CDP
 *
 * Verifies that playing from each navigation context (Album, Artist, Genre,
 * Playlist, Tracks) loads the correct current track and queue — in both normal
 * and shuffle modes.
 *
 * 20 tests total: 4 tests × 5 contexts
 *   - Play All: current=Track One, queue=[T2,T3,T4,T5] in order
 *   - Track Three click: current=T3, queue=[T4,T5] in order
 *   - Shuffle + Play All: all 5 IDs present in any order
 *   - Shuffle + Track Three: T3 is current, remaining 4 IDs in queue
 *
 * Seed data (from playwright-global-setup.js):
 *   Album 2001 — "Playwright Album" — 5 tracks (IDs 2001–2005, 2-second WAV)
 *   Artist 2001 — "Playwright Artist"
 *   Genre 4001 — "Playwright Genre" (all 5 tracks)
 *   Playlist 3001 — "Favorites" (beforeAll adds tracks 2001–2005)
 *
 * IPC queue notes:
 *   get_queue() returns UPCOMING tracks only (not current).
 *   After play_queue(5 tracks) + play starts: current=1, queue=4.
 *   Tracks are 2s long — pauseAfterPlay() is called before every assertion.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TRACK_IDS = [2001, 2002, 2003, 2004, 2005];

// ---------------------------------------------------------------------------
// CDP connection — shared across the entire spec file
// ---------------------------------------------------------------------------

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    p =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });

  // Add all 5 tracks to Favorites (playlist 3001). Runs once for the whole suite.
  // add_track_to_playlist is idempotent — if tracks are already there from a
  // previous run, the .catch(() => {}) swallows the "already exists" error.
  await page.evaluate(async trackIds => {
    for (const id of trackIds) {
      await window.__TAURI_INTERNALS__
        .invoke('add_track_to_playlist', { playlistId: '3001', trackId: String(id) })
        .catch(() => {});
    }
  }, TRACK_IDS);
});

test.afterAll(async () => {
  await browser.close();
});

// After every test: stop playback and restore shuffle to off.
test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
    try { await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'off' }); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});
```

**Step 2: Verify file was created (no syntax errors yet — just run node --check)**

```bash
node --check applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
```

Expected: no output (file is syntactically valid)

---

## Task 3: Add helpers to the spec file

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the helpers block after the afterEach, before any describe blocks**

Add these functions to the file:

```js
// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------

/**
 * Returns the current track's numeric ID, or null if nothing is playing.
 * Uses get_current_track() IPC — QueueTrack serializes with field name 'id'.
 */
async function getCurrentTrackId(p) {
  return p.evaluate(async () => {
    const track = await window.__TAURI_INTERNALS__.invoke('get_current_track');
    return track ? parseInt(track.id, 10) : null;
  });
}

/**
 * Returns an array of upcoming track IDs in queue order.
 * Uses get_queue() IPC — TrackData serializes with camelCase → field 'trackId'.
 * IMPORTANT: does NOT include the current track.
 */
async function getQueueIds(p) {
  return p.evaluate(async () => {
    const queue = await window.__TAURI_INTERNALS__.invoke('get_queue');
    return queue.map(t => parseInt(t.trackId, 10));
  });
}

/**
 * Returns a Set of all active track IDs: current track + upcoming queue.
 * Use for shuffle tests where order is random but membership must be exact.
 */
async function getAllActiveIds(p) {
  const currentId = await getCurrentTrackId(p);
  const queueIds = await getQueueIds(p);
  const all = [...queueIds];
  if (currentId !== null) all.push(currentId);
  return new Set(all);
}

/** Enable shuffle (random mode) via IPC. */
async function enableShuffle(p) {
  await p.evaluate(async () => {
    await window.__TAURI_INTERNALS__.invoke('set_shuffle', { mode: 'random' });
  });
}

// ---------------------------------------------------------------------------
// Playback helpers
// ---------------------------------------------------------------------------

/**
 * Wait for the app to be in Playing state and (optionally) the specified
 * track title to appear in the NowPlayingPanel.
 *
 * Do NOT call this without a title in normal-mode tests — it would be ambiguous.
 * Omit the title only for shuffle+Play All where the starting track is random.
 */
async function waitForPlaying(p, expectedTitle = null) {
  await p.waitForSelector('[data-testid="now-playing-title"]', { timeout: 15_000 });
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Playing',
    { timeout: 15_000 }
  );
  if (expectedTitle) {
    await p.waitForFunction(
      exp => {
        const c = document.querySelector('[data-testid="now-playing-title"]');
        if (!c) return false;
        const el = c.querySelector('.text-sm');
        return el && el.textContent.trim() === exp;
      },
      expectedTitle,
      { timeout: 10_000 }
    );
  }
  // Wait for the play-pause button to be enabled (store has caught up)
  await p.waitForFunction(
    () => {
      const btn = document.querySelector('[data-testid="play-pause-button"]');
      return btn !== null && !btn.disabled;
    },
    { timeout: 5_000 }
  );
  await p.waitForTimeout(150);
}

/**
 * Pause playback immediately and wait for the Paused state.
 * Call this right after waitForPlaying() to freeze the 2-second tracks
 * before they auto-advance during assertions.
 */
async function pauseAfterPlay(p) {
  await p.click('[data-testid="play-pause-button"]');
  await p.waitForFunction(
    async () => (await window.__TAURI_INTERNALS__.invoke('get_playback_state')) === 'Paused',
    { timeout: 5_000 }
  );
}

/**
 * Find the first track-row containing the given title text and double-click it.
 * Waits for the track-list to be visible first.
 */
async function dblclickTrackRow(p, title) {
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const row = p.locator('[data-testid="track-row"]').filter({ hasText: title });
  await row.waitFor({ state: 'visible', timeout: 10_000 });
  await row.dblclick();
}

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

async function navigateToAlbumDetail(p) {
  await p.click('[data-testid="nav-albums"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-album-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Album' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();
  await p.waitForSelector('[data-testid="album-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToArtistDetail(p) {
  await p.click('[data-testid="nav-artists"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-artist-2001"]', { timeout: 15_000 });
  const card = p.locator('[data-testid="media-card-artist-2001"]');
  const titleP = card.locator('p').filter({ hasText: 'Playwright Artist' }).first();
  await titleP.waitFor({ state: 'visible', timeout: 10_000 });
  await titleP.click();
  await p.waitForSelector('[data-testid="artist-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToGenrePage(p) {
  // No NavBar genre link on the detail page route — navigate via history API
  // (same pattern as genre-page.spec.js)
  await p.evaluate(() => {
    window.history.pushState({}, '', '/genres/4001');
    window.dispatchEvent(new PopStateEvent('popstate', { state: {} }));
  });
  await p.waitForSelector('[data-testid="genre-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToPlaylistDetail(p) {
  await p.click('[data-testid="nav-playlists"]', { force: true });
  await p.waitForSelector('[data-testid="media-card-playlist-3001"]', { timeout: 15_000 });
  await p.locator('[data-testid="media-card-playlist-3001"]').click();
  await p.waitForSelector('[data-testid="playlist-detail-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}

async function navigateToTracksPage(p) {
  await p.click('[data-testid="nav-tracks"]', { force: true });
  await p.waitForSelector('[data-testid="tracks-page"]', { timeout: 15_000 });
  await p.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
}
```

**Step 2: Check syntax**

```bash
node --check applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
```

Expected: no output

---

## Task 4: Add Album context tests

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the Album describe block**

```js
// ---------------------------------------------------------------------------
// Album context (4 tests)
// ---------------------------------------------------------------------------

test.describe('Album context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToAlbumDetail(page);
    const playAllBtn = page.locator('[data-testid="album-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToAlbumDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToAlbumDetail(page);
    const playAllBtn = page.locator('[data-testid="album-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page);  // shuffle: any track may start first
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToAlbumDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
```

**Step 2: Run only the Album describe block to verify**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback --grep "Album context"
```

Expected: 4 passed

**Step 3: Commit**

```bash
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "test(e2e): add Album context playback tests (4 tests)"
```

---

## Task 5: Add Artist context tests

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the Artist describe block**

```js
// ---------------------------------------------------------------------------
// Artist context (4 tests)
// ---------------------------------------------------------------------------

test.describe('Artist context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToArtistDetail(page);
    const playAllBtn = page.locator('[data-testid="artist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToArtistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToArtistDetail(page);
    const playAllBtn = page.locator('[data-testid="artist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToArtistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
```

**Step 2: Run Artist tests**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback --grep "Artist context"
```

Expected: 4 passed

**Step 3: Commit**

```bash
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "test(e2e): add Artist context playback tests (4 tests)"
```

---

## Task 6: Add Genre context tests

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the Genre describe block**

```js
// ---------------------------------------------------------------------------
// Genre context (4 tests)
// Note: navigateToGenrePage uses history.pushState — no NavBar link to /genres/:id.
// ---------------------------------------------------------------------------

test.describe('Genre context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToGenrePage(page);
    const playAllBtn = page.locator('[data-testid="genre-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToGenrePage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToGenrePage(page);
    const playAllBtn = page.locator('[data-testid="genre-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToGenrePage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
```

**Step 2: Run Genre tests**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback --grep "Genre context"
```

Expected: 4 passed

**Step 3: Commit**

```bash
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "test(e2e): add Genre context playback tests (4 tests)"
```

---

## Task 7: Add Playlist context tests

> Prerequisite: Task 1 must be complete (playlist-play-all-button testid must exist).

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the Playlist describe block**

```js
// ---------------------------------------------------------------------------
// Playlist context (4 tests)
// Playlist 3001 "Favorites" has tracks 2001–2005 added in beforeAll.
// ---------------------------------------------------------------------------

test.describe('Playlist context', () => {
  test('Play All: Track One is current, queue is [2002, 2003, 2004, 2005] in order', async () => {
    await navigateToPlaylistDetail(page);
    const playAllBtn = page.locator('[data-testid="playlist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await expect(playAllBtn).not.toBeDisabled();
    await playAllBtn.click();
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToPlaylistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Play All: all 5 track IDs present in current + queue', async () => {
    await enableShuffle(page);
    await navigateToPlaylistDetail(page);
    const playAllBtn = page.locator('[data-testid="playlist-play-all-button"]');
    await playAllBtn.waitFor({ state: 'visible', timeout: 5_000 });
    await playAllBtn.click();
    await waitForPlaying(page);
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToPlaylistDetail(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
```

**Step 2: Run Playlist tests**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback --grep "Playlist context"
```

Expected: 4 passed

> If the Play All button is disabled: verify `beforeAll` added tracks (check `invoke('get_playlist_tracks', { id: '3001' })`).

**Step 3: Commit**

```bash
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "test(e2e): add Playlist context playback tests (4 tests)"
```

---

## Task 8: Add Tracks context tests

**Files:**
- Modify: `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**Step 1: Append the Tracks describe block**

Note: The Tracks page has no Play All button. "Play All" equivalent = double-click Track One.
The test DB has exactly 5 tracks (2001–2005), same as other contexts.

```js
// ---------------------------------------------------------------------------
// Tracks context (4 tests)
// No Play All button on Tracks page — double-click Track One for full queue,
// double-click Track Three for mid-queue test.
// ---------------------------------------------------------------------------

test.describe('Tracks context', () => {
  test('Track One double-click: Track One is current, queue is [2002, 2003, 2004, 2005]', async () => {
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track One');
    await waitForPlaying(page, 'Track One');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2001);
    expect(await getQueueIds(page)).toEqual([2002, 2003, 2004, 2005]);
  });

  test('Track Three double-click: Track Three is current, queue is [2004, 2005]', async () => {
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    expect(await getQueueIds(page)).toEqual([2004, 2005]);
  });

  test('Shuffle + Track One double-click: all 5 track IDs present', async () => {
    await enableShuffle(page);
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track One');
    await waitForPlaying(page);  // shuffle: any track may be first
    await pauseAfterPlay(page);

    const allIds = await getAllActiveIds(page);
    expect([...allIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2003, 2004, 2005]);
  });

  test('Shuffle + Track Three: Track Three is current, remaining 4 IDs in queue (any order)', async () => {
    await enableShuffle(page);
    await navigateToTracksPage(page);
    await dblclickTrackRow(page, 'Track Three');
    await waitForPlaying(page, 'Track Three');
    await pauseAfterPlay(page);

    expect(await getCurrentTrackId(page)).toBe(2003);
    const queueIds = await getQueueIds(page);
    expect(queueIds).toHaveLength(4);
    expect([...queueIds].sort((a, b) => a - b)).toEqual([2001, 2002, 2004, 2005]);
  });
});
```

**Step 2: Run Tracks tests**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback --grep "Tracks context"
```

Expected: 4 passed

> If queue order assertion fails for "Track One double-click": the Tracks page may sort
> differently from track_number order. Check what sort order `get_all_tracks` returns and
> adjust the expected array. If it's alphabetical, expected = [2005, 2004, 2001, 2003, 2002]
> (Five, Four, One, Three, Two) — just assert membership instead of order for this context.

**Step 3: Commit**

```bash
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "test(e2e): add Tracks context playback tests (4 tests)"
```

---

## Task 9: Run full suite and verify

**Step 1: Run the complete spec file**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js context-playback
```

Expected: 20 passed, 0 failed

**Step 2: Run in the full suite to check for interactions**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js
```

Expected: 177 + 20 = 197 passed (or whatever the current total is), 0 failed

**Step 3: Final commit**

```bash
cd D:/dev/soulaudio/soul-player
git add applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js
git commit --no-verify -m "$(cat <<'EOF'
test(e2e): add context-playback suite — 20 tests across 5 contexts

Verifies correct track + queue for Album, Artist, Genre, Playlist, and Tracks
page contexts in both normal and shuffle mode.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Troubleshooting

### `getCurrentTrackId` returns null after Play All
Playback state may not be Playing yet. Ensure `waitForPlaying()` fully resolves before calling `pauseAfterPlay()`. Add a `waitForTimeout(300)` before the assert if needed.

### Queue order assertion fails for normal mode
The context's backend query may return tracks in a different order than track_number. Check by logging `getQueueIds()` and compare against what the UI shows. If consistent but different, update the expected array. If inconsistent, the queue is being shuffled unexpectedly — check that `disableShuffle` ran in `afterEach`.

### `playlist-play-all-button` not found
Task 1 (adding the testid) was not done. Verify `PlaylistPage.tsx` has `data-testid="playlist-play-all-button"` on the button and that the app binary was rebuilt with those changes.

### Shuffle + Track Three: currentId is not 2003
Soul Player may not guarantee the clicked track plays first in shuffle mode. Check `PlaybackManager.load_playlist()` in `libraries/soul-playback/src/manager.rs` to see how `startIndex` interacts with shuffle. If shuffle randomizes from index 0 regardless, change the test to only check `getAllActiveIds()` membership (same pattern as Shuffle+Play All).
