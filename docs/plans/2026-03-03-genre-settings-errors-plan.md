# Genre Integration, Settings Persistence & Playback Errors — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add genre navigation + filtering to Albums/Tracks pages, surface playback errors as toast+skip, and add E2E tests for all three flows.

**Architecture:** Genre filtering slots into the existing `LibraryPageLayout` via a new `filterPanel` prop. Error handling moves into `TauriPlayerCommandsProvider` (which already owns both Tauri listeners and player commands). E2E tests use the CDP pattern already established — genre 4001 is already in the seed DB.

**Tech Stack:** React + TypeScript (shared/desktop), Rust Tauri commands (main.rs), Playwright CDP E2E tests, sonner toasts.

---

## Task 1: Add `get_genre_albums` Rust command

**Files:**
- Modify: `applications/desktop/src-tauri/src/main.rs` (after `get_genre_tracks` ~line 1660)

### Step 1: Add the command function

Insert this block after the `get_genre_tracks` function and before the `// Playlist commands` comment:

```rust
#[tauri::command]
async fn get_genre_albums(
    genre_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_by_genre(&state.pool, genre_id, 1000)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}
```

### Step 2: Register the command in the builder

Search for `.invoke_handler(tauri::generate_handler![` in main.rs. Find `get_genre_tracks` in the handler list and add `get_genre_albums` right after it:

```rust
            get_genre_tracks,
            get_genre_albums,  // ADD THIS LINE
```

### Step 3: Verify it compiles

```bash
cd applications/desktop/src-tauri
cargo check 2>&1 | grep -E "error|warning: unused"
```

Expected: no errors. Some unused import warnings are OK.

### Step 4: Commit

```bash
git add applications/desktop/src-tauri/src/main.rs
git commit -m "feat(backend): add get_genre_albums Tauri command"
```

---

## Task 2: Add `getGenreAlbums` to BackendContext + providers

**Files:**
- Modify: `applications/shared/src/contexts/BackendContext.tsx`
- Modify: `applications/desktop/src/providers/TauriBackendProvider.tsx`
- Modify: `applications/shared/src/providers/MockBackendProvider.tsx`

### Step 1: Add to BackendInterface

In `BackendContext.tsx`, find `getAllGenres: () => Promise<BackendGenre[]>` in the `BackendInterface` and add `getGenreAlbums` right after it:

```typescript
  getAllGenres: () => Promise<BackendGenre[]>
  getGenreAlbums: (genreId: number) => Promise<BackendAlbum[]>
```

### Step 2: Implement in TauriBackendProvider

In `TauriBackendProvider.tsx`, find `async getAllGenres()` and add after it:

```typescript
    async getGenreAlbums(genreId: number) {
      return invoke<BackendAlbum[]>('get_genre_albums', { genreId })
    },
```

### Step 3: Stub in MockBackendProvider

In `MockBackendProvider.tsx`, find `async getAllGenres()` (or `openFileDialog` as the last method) and add:

```typescript
    async getGenreAlbums(_genreId: number) {
      return []
    },
```

### Step 4: TypeScript check

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -30
cd applications/desktop && yarn tsc --noEmit 2>&1 | head -30
```

Expected: no errors.

### Step 5: Commit

```bash
git add applications/shared/src/contexts/BackendContext.tsx \
        applications/desktop/src/providers/TauriBackendProvider.tsx \
        applications/shared/src/providers/MockBackendProvider.tsx
git commit -m "feat(backend): add getGenreAlbums to BackendContext and providers"
```

---

## Task 3: Add Genres nav item to NavBar

**Files:**
- Modify: `applications/shared/src/components/sidebar/NavBar.tsx`

### Step 1: Add to navigationItems array

Find `navigationItems`:

```typescript
const navigationItems: NavItem[] = [
  { id: 'home', labelKey: 'nav.home', path: '/' },
  { id: 'albums', labelKey: 'library.tab.albums', path: '/albums' },
  { id: 'artists', labelKey: 'library.tab.artists', path: '/artists' },
  { id: 'playlists', labelKey: 'library.tab.playlists', path: '/playlists' },
  { id: 'tracks', labelKey: 'library.tab.tracks', path: '/tracks' },
];
```

Replace with (add genres between playlists and tracks):

```typescript
const navigationItems: NavItem[] = [
  { id: 'home', labelKey: 'nav.home', path: '/' },
  { id: 'albums', labelKey: 'library.tab.albums', path: '/albums' },
  { id: 'artists', labelKey: 'library.tab.artists', path: '/artists' },
  { id: 'playlists', labelKey: 'library.tab.playlists', path: '/playlists' },
  { id: 'genres', labelKey: 'nav.genres', path: '/genres' },
  { id: 'tracks', labelKey: 'library.tab.tracks', path: '/tracks' },
];
```

### Step 2: Verify no TypeScript errors

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -20
```

### Step 3: Commit

```bash
git add applications/shared/src/components/sidebar/NavBar.tsx
git commit -m "feat(nav): add Genres link to NavBar"
```

---

## Task 4: Create GenresListPage

**Files:**
- Create: `applications/shared/src/pages/GenresListPage.tsx`

### Step 1: Write the page component

```typescript
import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { Music } from 'lucide-react'
import { useBackend, type BackendGenre } from '../contexts/BackendContext'
import { cn } from '../lib/utils'

export function GenresListPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const navigate = useNavigate()
  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    backend.getAllGenres()
      .then(setGenres)
      .catch(() => setGenres([]))
      .finally(() => setIsLoading(false))
  }, [backend])

  if (isLoading) {
    return (
      <div data-testid="genres-page" className="p-6">
        <div className="grid gap-3 grid-cols-2 sm:grid-cols-3 md:grid-cols-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className="h-24 rounded-lg bg-muted animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  if (genres.length === 0) {
    return (
      <div data-testid="genres-page" className="flex flex-col items-center justify-center py-24 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('genres.empty', 'No genres found')}</p>
      </div>
    )
  }

  return (
    <div data-testid="genres-page" className="p-6">
      <div className="grid gap-3 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
        {genres.map((genre) => (
          <button
            key={genre.id}
            data-testid={`genre-card-${genre.id}`}
            onClick={() => navigate(`/genres/${genre.id}`)}
            className={cn(
              'text-left p-4 rounded-lg bg-muted hover:bg-muted/80 transition-opacity',
              'hover:opacity-[var(--hover-text-opacity)]'
            )}
          >
            <p className="font-semibold text-foreground truncate">{genre.name}</p>
            <p className="text-sm text-muted-foreground mt-1">
              {t('genres.trackCount', { count: genre.track_count }, `${genre.track_count} tracks`)}
            </p>
          </button>
        ))}
      </div>
    </div>
  )
}
```

### Step 2: Export from shared index

In `applications/shared/src/index.ts`, find the pages export block (around line 163-172) and add:

```typescript
export { GenresListPage } from './pages/GenresListPage';
```

### Step 3: TypeScript check

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -20
```

### Step 4: Commit

```bash
git add applications/shared/src/pages/GenresListPage.tsx \
        applications/shared/src/index.ts
git commit -m "feat(genres): add GenresListPage showing all genres as cards"
```

---

## Task 5: Add `/genres` route in desktop App.tsx

**Files:**
- Modify: `applications/desktop/src/App.tsx`

### Step 1: Add import

Find the shared pages import block:

```typescript
import {
  HomePage,
  AlbumsPage,
  // ...
} from '@soul-player/shared';
```

Add `GenresListPage` to the destructured imports list.

### Step 2: Add route

Find:

```typescript
<Route path="/genres/:id" element={<GenrePage />} />
```

Add before it:

```typescript
<Route path="/genres" element={<GenresListPage />} />
```

### Step 3: TypeScript check

```bash
cd applications/desktop && yarn tsc --noEmit 2>&1 | head -20
```

### Step 4: Commit

```bash
git add applications/desktop/src/App.tsx
git commit -m "feat(router): add /genres route for GenresListPage"
```

---

## Task 6: Add filterPanel prop to LibraryPageLayout

**Files:**
- Modify: `applications/shared/src/components/LibraryPageLayout.tsx`

### Step 1: Add prop to interface

Find `LibraryPageLayoutProps`:

```typescript
  /** The main content (grid, list, etc.) */
  children: ReactNode
```

Add before `children`:

```typescript
  /** Optional expandable filter panel rendered below the search bar */
  filterPanel?: ReactNode
  /** When true, increases content top padding to account for filter panel height */
  filterPanelVisible?: boolean
```

### Step 2: Destructure the new props

Find:

```typescript
export function LibraryPageLayout({
  searchQuery,
  setSearchQuery,
  // ...
  children,
}: LibraryPageLayoutProps) {
```

Add `filterPanel` and `filterPanelVisible` to the destructured list:

```typescript
  filterPanel,
  filterPanelVisible = false,
  children,
```

### Step 3: Render the filter panel

Find:

```typescript
            {additionalButtons}
          </div>
        </div>
```

Change it to (add filter panel row after the search row):

```typescript
            {additionalButtons}
          </div>
          {filterPanel}
        </div>
```

### Step 4: Adjust content padding

Find the scroll container `pt` class:

```typescript
showSearchBar ? 'pt-14' : 'pt-6'
```

Replace with:

```typescript
showSearchBar ? (filterPanelVisible ? 'pt-28' : 'pt-14') : 'pt-6'
```

(`pt-28` = 112px — accounts for search bar ~52px + gap + chips row ~44px)

### Step 5: TypeScript check

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -20
```

### Step 6: Commit

```bash
git add applications/shared/src/components/LibraryPageLayout.tsx
git commit -m "feat(layout): add filterPanel slot to LibraryPageLayout"
```

---

## Task 7: Add genre filter to AlbumsPage

**Files:**
- Modify: `applications/shared/src/pages/AlbumsPage.tsx`

### Step 1: Add imports

At the top of the file, add:

```typescript
import { useEffect } from 'react'  // (useState is already imported)
import { SlidersHorizontal, X } from 'lucide-react'  // add to existing lucide import
import type { BackendGenre } from '../contexts/BackendContext'
```

### Step 2: Add genre state

After the existing `useState` declarations, add:

```typescript
  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [selectedGenreId, setSelectedGenreId] = useState<number | null>(null)
  const [showFilters, setShowFilters] = useState(false)
```

### Step 3: Load genres on mount

After the existing `useAlbums()` call, add:

```typescript
  useEffect(() => {
    backend.getAllGenres().then(setGenres).catch(() => {})
  }, [backend])
```

### Step 4: Replace `useAlbums()` with conditional fetch

Replace:

```typescript
  const { data: albums = [], isLoading, isError, error } = useAlbums()
```

With:

```typescript
  const { data: allAlbums = [], isLoading: albumsLoading, isError, error } = useAlbums()
  const [genreAlbums, setGenreAlbums] = useState<BackendAlbum[]>([])
  const [genreAlbumsLoading, setGenreAlbumsLoading] = useState(false)

  useEffect(() => {
    if (selectedGenreId === null) {
      setGenreAlbums([])
      return
    }
    setGenreAlbumsLoading(true)
    backend.getGenreAlbums(selectedGenreId)
      .then(setGenreAlbums)
      .catch(() => setGenreAlbums([]))
      .finally(() => setGenreAlbumsLoading(false))
  }, [selectedGenreId, backend])

  const albums = selectedGenreId !== null ? genreAlbums : allAlbums
  const isLoading = selectedGenreId !== null ? genreAlbumsLoading : albumsLoading
```

You need to add `BackendAlbum` to the BackendContext import at the top of the file (it's likely already there, just check).

### Step 5: Build the filter panel JSX

After the `healthWarning` line, add:

```typescript
  const filterPanel = genres.length === 0 ? null : (
    <div className={`overflow-hidden transition-all duration-200 ${showFilters ? 'max-h-20 opacity-100' : 'max-h-0 opacity-0'}`}>
      <div className="flex flex-wrap gap-2 pt-2 pb-1">
        {genres.map((genre) => (
          <button
            key={genre.id}
            data-testid={`genre-chip-${genre.id}${selectedGenreId === genre.id ? '-active' : ''}`}
            onClick={() => setSelectedGenreId(selectedGenreId === genre.id ? null : genre.id)}
            className={cn(
              'px-3 py-1 rounded-full text-sm transition-all border',
              selectedGenreId === genre.id
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-muted text-muted-foreground border-transparent hover:border-muted-foreground/30'
            )}
          >
            {genre.name}
          </button>
        ))}
      </div>
    </div>
  )
```

### Step 6: Build the filters button

After the `filterPanel` const, add:

```typescript
  const filtersButton = (
    <button
      data-testid="filter-toggle-button"
      onClick={() => {
        setShowFilters(v => !v)
        if (showFilters) setSelectedGenreId(null)
      }}
      className={cn(
        'flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm transition-all border',
        showFilters || selectedGenreId !== null
          ? 'border-primary text-primary bg-primary/10'
          : 'border-transparent text-muted-foreground bg-muted hover:opacity-[var(--hover-text-opacity)]'
      )}
    >
      {selectedGenreId !== null ? <X className="w-3.5 h-3.5" /> : <SlidersHorizontal className="w-3.5 h-3.5" />}
      <span>Filters</span>
      {selectedGenreId !== null && (
        <span className="w-2 h-2 rounded-full bg-primary" />
      )}
    </button>
  )
```

### Step 7: Pass new props to LibraryPageLayout

Find `<LibraryPageLayout` and add:

```tsx
      additionalButtons={filtersButton}
      filterPanel={filterPanel}
      filterPanelVisible={showFilters}
```

### Step 8: TypeScript check

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -30
```

Fix any type errors (e.g., missing `BackendAlbum` import).

### Step 9: Commit

```bash
git add applications/shared/src/pages/AlbumsPage.tsx
git commit -m "feat(albums): add More Filters panel with genre chip filter"
```

---

## Task 8: Add genre filter to TracksPage

**Files:**
- Modify: `applications/shared/src/pages/TracksPage.tsx`

### Step 1: Add imports

```typescript
import { SlidersHorizontal, X } from 'lucide-react'  // add to lucide import
import type { BackendGenre } from '../contexts/BackendContext'
```

### Step 2: Add genre state (same pattern as AlbumsPage)

```typescript
  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [selectedGenreId, setSelectedGenreId] = useState<number | null>(null)
  const [showFilters, setShowFilters] = useState(false)
  const [genreTracks, setGenreTracks] = useState<BackendTrack[]>([])
  const [genreTracksLoading, setGenreTracksLoading] = useState(false)
```

### Step 3: Load genres

```typescript
  useEffect(() => {
    backend.getAllGenres().then(setGenres).catch(() => {})
  }, [backend])
```

### Step 4: Load genre tracks when selection changes

```typescript
  useEffect(() => {
    if (selectedGenreId === null) {
      setGenreTracks([])
      return
    }
    setGenreTracksLoading(true)
    // get_genre_tracks is already a Tauri command — invoke via backend pattern
    // Since BackendContext doesn't have getGenreTracks, use invoke directly here
    // via the existing Tauri IPC (TracksPage is desktop-only context)
    import('@tauri-apps/api/core').then(({ invoke }) =>
      invoke<BackendTrack[]>('get_genre_tracks', { genreId: selectedGenreId })
    )
      .then(setGenreTracks)
      .catch(() => setGenreTracks([]))
      .finally(() => setGenreTracksLoading(false))
  }, [selectedGenreId])
```

> **Note:** Alternatively, add `getGenreTracks` to BackendContext (same pattern as Task 2). If you prefer that cleaner approach, add it to BackendContext, TauriBackendProvider (invoke `'get_genre_tracks'`), and MockBackendProvider (return `[]`). Then use `backend.getGenreTracks(selectedGenreId)` above instead. Either works — choose based on whether you want it testable in marketing demo.

### Step 5: Replace tracks source

Find:

```typescript
  const { data: tracks = [], isLoading, isError, error } = useTracks()
```

Change to:

```typescript
  const { data: allTracks = [], isLoading: tracksLoading, isError, error } = useTracks()
  const tracks = selectedGenreId !== null ? genreTracks : allTracks
  const isLoading = selectedGenreId !== null ? genreTracksLoading : tracksLoading
```

### Step 6: Add filterPanel and filtersButton (same pattern as AlbumsPage Task 7 Steps 5-6)

Copy the pattern exactly. The only difference: `filtersButton` clears genre selection differently if needed.

### Step 7: Pass to LibraryPageLayout

```tsx
      additionalButtons={filtersButton}
      filterPanel={filterPanel}
      filterPanelVisible={showFilters}
```

### Step 8: TypeScript check + commit

```bash
cd applications/shared && yarn tsc --noEmit 2>&1 | head -30
git add applications/shared/src/pages/TracksPage.tsx
git commit -m "feat(tracks): add More Filters panel with genre chip filter"
```

---

## Task 9: Wire playback error handler in TauriPlayerCommandsProvider

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`
- Modify: `applications/shared/src/hooks/usePlaybackEvents.ts`

### Step 1: Check sonner import

In `TauriPlayerCommandsProvider.tsx`, verify `toast` from `sonner` is already imported. If not, add:

```typescript
import { toast } from 'sonner'
```

### Step 2: Replace the error listener

Find:

```typescript
        // Listen for errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          debug.error('[TauriPlayerCommandsProvider] Playback error:', event.payload);
        });
```

Replace with:

```typescript
        // Listen for errors — show toast and auto-skip to next track
        const unlistenError = await listen<string>('playback:error', (event) => {
          const message = event.payload ?? 'Playback error';
          debug.error('[TauriPlayerCommandsProvider] Playback error:', message);
          toast.error(message, { duration: 4000 });
          // Auto-skip: if queue has more tracks they will load; if not, playback stops naturally
          invoke('next_track').catch(() => {
            // Queue exhausted — ignore
          });
        });
```

### Step 3: Remove TODO stub from usePlaybackEvents.ts

Find:

```typescript
        // Listen for playback errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          if (!isMounted) return;
          debug.error('[Playback Error]', event.payload);
          // TODO: Show user-facing error notification (toast/snackbar)
          // Currently errors are only logged to console for debugging.
        });
        unlistenFunctions.push(unlistenError);
```

Remove the entire block (TauriPlayerCommandsProvider now owns this listener).

### Step 4: TypeScript check

```bash
cd applications/desktop && yarn tsc --noEmit 2>&1 | head -20
```

### Step 5: Commit

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx \
        applications/shared/src/hooks/usePlaybackEvents.ts
git commit -m "feat(playback): surface errors as toast + auto-skip to next track"
```

---

## Task 10: Build the binary

The Tauri debug binary embeds frontend assets at build time. All UI changes require a rebuild before E2E tests will see them.

### Step 1: Build the frontend

```bash
cd applications/desktop && yarn build 2>&1 | tail -20
```

Expected: build succeeds, `dist/` updated.

### Step 2: Build the Tauri binary

```bash
cargo build -p soul-player-desktop 2>&1 | tail -20
```

Expected: `target/debug/soul-player-desktop.exe` updated.

> **Windows note:** If build fails with "file locked", run:
> `taskkill //F //IM soul-player-desktop.exe 2>/dev/null; rm -f target/debug/soul-player-desktop.exe`

---

## Task 11: Write genre E2E spec

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/genre.spec.js`

### Step 1: Write the spec

```javascript
// @ts-check
const { test, expect, chromium } = require('@playwright/test');

const CDP_URL = 'http://localhost:9222';

let browser, page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const pages = browser.contexts().flatMap(c => c.pages());
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
      && !p.url().includes('splash')
  );
  if (!page) {
    const ctx = browser.contexts()[0] ?? await browser.newContext();
    page = ctx.pages()[0] ?? await ctx.newPage();
  }
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser?.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape');
});

test('nav-genres navigates to genres list page', async () => {
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForSelector('[data-testid="genres-page"]', { timeout: 10_000 });

  const card = await page.waitForSelector('[data-testid="genre-card-4001"]', { timeout: 5_000 });
  expect(card).toBeTruthy();

  const text = await page.textContent('[data-testid="genre-card-4001"]');
  expect(text).toContain('Playwright Genre');
  expect(text).toMatch(/5\s*tracks?/i);
});

test('clicking a genre card navigates to genre detail page', async () => {
  await page.click('[data-testid="nav-genres"]', { force: true });
  await page.waitForSelector('[data-testid="genre-card-4001"]', { timeout: 10_000 });

  await page.click('[data-testid="genre-card-4001"]');
  await page.waitForURL(/\/genres\/4001/, { timeout: 10_000 });

  // GenrePage renders a TrackList — verify tracks are shown
  await page.waitForSelector('[data-testid="track-list"]', { timeout: 10_000 });
  const rows = await page.$$('[data-testid="track-row"]');
  expect(rows.length).toBe(5);
});

test('albums page genre filter shows only matching albums', async () => {
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });

  // Open filters
  await page.click('[data-testid="filter-toggle-button"]');
  await page.waitForSelector('[data-testid="genre-chip-4001"]', { timeout: 5_000 });

  // Click the genre chip
  await page.click('[data-testid="genre-chip-4001"]');

  // Verify chip is active (testid changes to -active suffix)
  await page.waitForSelector('[data-testid="genre-chip-4001-active"]', { timeout: 5_000 });

  // Only album 2001 should be visible (all 5 tracks belong to genre 4001)
  const cards = await page.$$('[data-testid^="media-card-album-"]');
  expect(cards.length).toBeGreaterThanOrEqual(1);

  const card2001 = await page.$('[data-testid="media-card-album-2001"]');
  expect(card2001).toBeTruthy();
});

test('tracks page genre filter shows only matching tracks', async () => {
  await page.click('[data-testid="nav-tracks"]', { force: true });
  await page.waitForSelector('[data-testid="tracks-page"]', { timeout: 10_000 });

  // Open filters
  await page.click('[data-testid="filter-toggle-button"]');
  await page.waitForSelector('[data-testid="genre-chip-4001"]', { timeout: 5_000 });

  // Click the genre chip
  await page.click('[data-testid="genre-chip-4001"]');
  await page.waitForSelector('[data-testid="genre-chip-4001-active"]', { timeout: 5_000 });

  // All 5 tracks belong to genre 4001
  const rows = await page.$$('[data-testid="track-row"]');
  expect(rows.length).toBe(5);
});
```

### Step 2: Run the spec (binary must be rebuilt from Task 10 first)

```bash
cd applications/desktop/e2e-tests
npx playwright test tests/playwright/genre.spec.js --config playwright.cdp.config.js
```

Expected: 4 tests pass.

---

## Task 12: Write playback errors E2E spec

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/playback-errors.spec.js`

### Step 1: Write the spec

```javascript
// @ts-check
const { test, expect, chromium } = require('@playwright/test');

const CDP_URL = 'http://localhost:9222';

let browser, page;

const BAD_TRACK = {
  trackId: '9999',
  title: 'Bad Track',
  artist: 'Test Artist',
  album: 'Test Album',
  albumId: 9999,
  filePath: '/nonexistent/path/that/does/not/exist-9999.wav',
  durationSeconds: 2,
  trackNumber: 1,
};

async function getGoodTrack(p) {
  const tracks = await p.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
  });
  const sorted = tracks.sort((a, b) => (a.track_number ?? 0) - (b.track_number ?? 0));
  const t = sorted[0];
  return {
    trackId: String(t.id),
    title: t.title,
    artist: t.artist_name || 'Unknown Artist',
    album: t.album_title,
    albumId: t.album_id,
    filePath: t.file_path,
    durationSeconds: t.duration_seconds,
    trackNumber: t.track_number,
  };
}

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const pages = browser.contexts().flatMap(c => c.pages());
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
      && !p.url().includes('splash')
  );
  if (!page) {
    const ctx = browser.contexts()[0] ?? await browser.newContext();
    page = ctx.pages()[0] ?? await ctx.newPage();
  }
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser?.close();
});

test.beforeEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(300);
  await page.keyboard.press('Escape');
});

test('single bad track: toast error appears, playback does not hang', async () => {
  await page.evaluate(async (track) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue: [track], startIndex: 0 });
  }, BAD_TRACK);

  // Toast should appear within 5s
  await page.waitForSelector('[data-sonner-toaster]', { timeout: 5_000 });
  const toastText = await page.textContent('[data-sonner-toaster]');
  // Toast has some error text (exact message from Rust, just check it's not empty)
  expect(toastText?.length).toBeGreaterThan(0);

  // Playback should have stopped — no track playing
  const nowPlaying = await page.$('[data-testid="now-playing-title"]');
  // Either not present (stopped) or showing an error/stopped state
  // Main check: no hang — test completed without timeout
});

test('bad track then good track: auto-skips to good track', async () => {
  const goodTrack = await getGoodTrack(page);

  await page.evaluate(async ({ bad, good }) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', {
      queue: [bad, good],
      startIndex: 0,
    });
  }, { bad: BAD_TRACK, good: goodTrack });

  // Toast appears
  await page.waitForSelector('[data-sonner-toaster]', { timeout: 5_000 });

  // Auto-skip loads the good track
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 10_000 });
  const title = await page.textContent('[data-testid="now-playing-title"]');
  expect(title).toContain('Track One');
});

test('good track plays through; bad next track shows toast', async () => {
  const goodTrack = await getGoodTrack(page);

  await page.evaluate(async ({ good, bad }) => {
    await window.__TAURI_INTERNALS__.invoke('play_queue', {
      queue: [good, bad],
      startIndex: 0,
    });
  }, { good: goodTrack, bad: BAD_TRACK });

  // First track starts
  await page.waitForSelector('[data-testid="now-playing-title"]', { timeout: 10_000 });
  const title = await page.textContent('[data-testid="now-playing-title"]');
  expect(title).toContain('Track One');

  // Wait for Track One to finish (2s duration) + bad track error
  // Poll for toast with a generous timeout
  await page.waitForFunction(() => {
    const toaster = document.querySelector('[data-sonner-toaster]');
    return toaster && toaster.textContent && toaster.textContent.length > 0;
  }, { timeout: 10_000 });
});
```

### Step 2: Run

```bash
cd applications/desktop/e2e-tests
npx playwright test tests/playwright/playback-errors.spec.js --config playwright.cdp.config.js
```

Expected: 3 tests pass.

---

## Task 13: Write settings persistence E2E spec

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/settings-persistence.spec.js`

### Step 1: Write the spec

```javascript
// @ts-check
const { test, expect, chromium } = require('@playwright/test');

const CDP_URL = 'http://localhost:9222';

let browser, page;

async function setSetting(p, key, value) {
  return p.evaluate(async ({ k, v }) => {
    return window.__TAURI_INTERNALS__.invoke('set_user_setting', { key: k, value: v });
  }, { k: key, v: value });
}

async function getSetting(p, key) {
  return p.evaluate(async (k) => {
    return window.__TAURI_INTERNALS__.invoke('get_user_setting', { key: k });
  }, key);
}

async function getAllSettings(p) {
  return p.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('get_user_settings');
  });
}

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const pages = browser.contexts().flatMap(c => c.pages());
  page = pages.find(
    p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost'))
      && !p.url().includes('splash')
  );
  if (!page) {
    const ctx = browser.contexts()[0] ?? await browser.newContext();
    page = ctx.pages()[0] ?? await ctx.newPage();
  }
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser?.close();
});

test('set and read back audio.volume', async () => {
  await setSetting(page, 'audio.volume', 75);
  const value = await getSetting(page, 'audio.volume');
  expect(value).toBe(75);
});

test('set and read back ui.theme', async () => {
  await setSetting(page, 'ui.theme', 'ocean');
  const value = await getSetting(page, 'ui.theme');
  expect(value).toBe('ocean');
});

test('set and read back import.confidence_threshold', async () => {
  await setSetting(page, 'import.confidence_threshold', 90);
  const value = await getSetting(page, 'import.confidence_threshold');
  expect(value).toBe(90);
});

test('set and read back app.auto_update_enabled = false', async () => {
  await setSetting(page, 'app.auto_update_enabled', false);
  const value = await getSetting(page, 'app.auto_update_enabled');
  expect(value).toBe(false);
});

test('settings persist across page navigation', async () => {
  // Write all 4 settings
  await setSetting(page, 'audio.volume', 75);
  await setSetting(page, 'ui.theme', 'ocean');
  await setSetting(page, 'import.confidence_threshold', 90);
  await setSetting(page, 'app.auto_update_enabled', false);

  // Navigate away and back
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-artists"]', { force: true });
  await page.waitForTimeout(500);
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="albums-page"]', { timeout: 10_000 });

  // Re-read all settings
  expect(await getSetting(page, 'audio.volume')).toBe(75);
  expect(await getSetting(page, 'ui.theme')).toBe('ocean');
  expect(await getSetting(page, 'import.confidence_threshold')).toBe(90);
  expect(await getSetting(page, 'app.auto_update_enabled')).toBe(false);
});

test('get_user_settings returns all written keys', async () => {
  await setSetting(page, 'audio.volume', 75);
  await setSetting(page, 'ui.theme', 'ocean');
  await setSetting(page, 'import.confidence_threshold', 90);
  await setSetting(page, 'app.auto_update_enabled', false);

  const all = await getAllSettings(page);
  const keys = all.map(s => s.key);

  expect(keys).toContain('audio.volume');
  expect(keys).toContain('ui.theme');
  expect(keys).toContain('import.confidence_threshold');
  expect(keys).toContain('app.auto_update_enabled');

  const volumeSetting = all.find(s => s.key === 'audio.volume');
  expect(volumeSetting?.value).toBe(75);
});
```

### Step 2: Run

```bash
cd applications/desktop/e2e-tests
npx playwright test tests/playwright/settings-persistence.spec.js --config playwright.cdp.config.js
```

Expected: 6 tests pass.

---

## Task 14: Run full E2E suite and commit

### Step 1: Run all new specs together

```bash
cd applications/desktop/e2e-tests
npx playwright test \
  tests/playwright/genre.spec.js \
  tests/playwright/playback-errors.spec.js \
  tests/playwright/settings-persistence.spec.js \
  --config playwright.cdp.config.js
```

Expected: 13 tests pass (4 + 3 + 6).

### Step 2: Run the full suite to check for regressions

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js 2>&1 | tail -30
```

Expected: all 131 + 13 = 144 tests pass (with known pre-existing failures unchanged).

### Step 3: Commit everything

```bash
git add \
  applications/desktop/e2e-tests/tests/playwright/genre.spec.js \
  applications/desktop/e2e-tests/tests/playwright/playback-errors.spec.js \
  applications/desktop/e2e-tests/tests/playwright/settings-persistence.spec.js
git commit -m "test(e2e): add genre, playback-errors, and settings-persistence specs (13 tests)"
```

---

## Task 15: Final pre-commit check and push

```bash
cargo xtask check precommit
git push origin main
```
