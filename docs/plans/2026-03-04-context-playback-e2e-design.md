# Context Playback E2E Design

> **For Claude:** After approval, use superpowers:writing-plans to create the implementation plan.

**Goal:** Verify that triggering playback from each navigation context (Album, Artist, Genre, Playlist, Tracks) loads the correct track as current and builds the correct queue, in both normal and shuffle modes.

**Verification depth (B+C):**
- B: Correct track title is playing + queue contains correct track IDs in correct order
- C: Shuffle awareness — clicked track is current, all expected IDs present in any order

---

## Architecture

**Single file:** `applications/desktop/e2e-tests/tests/playwright/context-playback.spec.js`

**20 tests** — 4 tests × 5 contexts (Album, Artist, Genre, Playlist, Tracks):
- Test 1: Play All → Track One current, remaining 4 in strict order
- Test 2: Click Track Three → Track Three current, [Track Four, Track Five] in strict order
- Test 3: Shuffle + Play All → all 5 IDs present, any order
- Test 4: Shuffle + click Track Three → Track Three current, remaining 4 any order

**Describe blocks:** `Album context`, `Artist context`, `Genre context`, `Playlist context`, `Tracks context`

---

## Seed Data

From `playwright-global-setup.js` (already seeded):

| Entity | ID | Details |
|--------|-----|---------|
| Artist | 2001 | "Playwright Artist" |
| Album | 2001 | "Playwright Album", 5 × 2-second tracks |
| Tracks | 2001–2005 | Track One through Track Five |
| Genre | 4001 | "Playwright Genre", all 5 tracks |
| Playlist | 3001 | "Favorites", **empty** — must add tracks in `beforeAll` |

---

## Helpers

```js
async function pauseAfterPlay(page)
// Clicks play-pause-button immediately after playback starts.
// Critical: tracks are only 2s long. Must pause before expiry to freeze state for assertions.

async function getCurrentTrackId(page)
// Returns: number — invoke('get_current_track').then(t => t.id)

async function getQueueIds(page)
// Returns: number[] — invoke('get_queue').then(q => q.map(t => t.id))
// NOTE: get_queue() returns UPCOMING tracks only (not current).
// After play_queue(5) + play, returns 4 tracks.

async function getAllActiveIds(page)
// Returns: Set — currentTrackId ∪ queueIds. Used for shuffle membership checks.

async function enableShuffle(page)
async function disableShuffle(page)
// invoke('set_shuffle', { enabled: true/false })
```

---

## Suite Lifecycle

```
beforeAll:
  1. App already running (global setup)
  2. Add tracks 2001–2005 to playlist 3001 via add-to-playlist dialog
  3. disableShuffle()

afterEach:
  1. Stop playback (invoke 'stop' or navigate away to clear state)
  2. disableShuffle()
```

---

## Test Cases per Context

### Normal mode (Tests 1 & 2)

**Test 1 — Play All:**
- Trigger: click "Play All" button (all contexts) or double-click Track One (`/tracks` page — no Play All button)
- Assert: `getCurrentTrackId() === 2001`
- Assert: `getQueueIds() deepEqual [2002, 2003, 2004, 2005]` (strict order)

**Test 2 — Mid-queue click (Track Three):**
- Trigger: double-click Track Three row
- Assert: `getCurrentTrackId() === 2003`
- Assert: `getQueueIds() deepEqual [2004, 2005]` (strict order)

### Shuffle mode (Tests 3 & 4)

**Test 3 — Shuffle + Play All:**
- Setup: `enableShuffle()`
- Trigger: Play All
- Assert: `getAllActiveIds()` contains exactly `{2001, 2002, 2003, 2004, 2005}` (any order)
- Note: do not assert which track is current — shuffle picks randomly

**Test 4 — Shuffle + click Track Three:**
- Setup: `enableShuffle()`
- Trigger: double-click Track Three
- Assert: `getCurrentTrackId() === 2003` (clicked track is always current in shuffle)
- Assert: `getQueueIds()` contains exactly `{2001, 2002, 2004, 2005}` (any order)

---

## Navigation Map

| Context | Route | Play All trigger | Track row selector |
|---------|-------|------------------|--------------------|
| Album | `/albums/2001` | "Play All" button | `track-row` in track list |
| Artist | `/artists/2001` | "Play All" button | `track-row` in track list |
| Genre | `/genres/4001` | "Play All" button | `track-row` in track list |
| Playlist | `/playlists/3001` | "Play All" button | `track-row` in track list |
| Tracks | `/tracks` | double-click Track One | `track-row` |

---

## Key Implementation Notes

1. **pause-immediately pattern**: Always `pauseAfterPlay()` before any assertions. Without it, a 2s track may expire and auto-advance before `get_queue()` returns.

2. **Queue math**: `get_queue()` returns upcoming only. After Play All (5 tracks): current=1, queue=4. After clicking Track Three: current=Track Three, queue=2.

3. **Tracks page**: Only 5 tracks exist in the Playwright test DB — double-clicking Track One gives a clean 5-track queue identical to other contexts.

4. **Playlist beforeAll**: Playlist 3001 is seeded empty. `beforeAll` must add all 5 tracks before any test runs. Use the add-to-playlist dialog (same as `playlist-operations.spec.js`) or direct IPC if available.

5. **Shuffle assumption**: Double-clicking a specific track in shuffle mode plays that track immediately; the rest are shuffled. If Soul Player behaves differently, Test 4's `getCurrentTrackId() === 2003` assertion will catch it.
