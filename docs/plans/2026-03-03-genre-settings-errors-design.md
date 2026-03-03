# Genre Integration, Settings Persistence & Playback Error Handling

**Date:** 2026-03-03
**Status:** Approved

---

## Overview

Three independent features implemented together:

1. **Genre integration** — expose genres in NavBar + `GenresListPage`, add "More filters" panel with genre chips to Albums and Tracks pages
2. **Playback error handling** — surface `playback:error` events as toast notifications with auto-skip to next track
3. **Settings persistence E2E** — full round-trip IPC tests for user settings (write → read-back → navigate → re-verify)

---

## 1. Genre Integration

### Backend addition

`get_genre_albums` Tauri command (Rust side already has `albums::get_by_genre(pool, genre_id, limit)`, just needs a command wrapper):

```rust
#[tauri::command]
async fn get_genre_albums(genre_id: i64, state: State<'_, AppState>) -> Result<Vec<FrontendAlbum>, String>
```

- Add to `BackendInterface` in `BackendContext.tsx`: `getGenreAlbums(genreId: number): Promise<Album[]>`
- Implement in `TauriBackendProvider.tsx` (invoke `'get_genre_albums'`)
- Stub in `MockBackendProvider.tsx` (return `[]`)

### Navigation

`NavBar.tsx` (shared) gets a new entry between Playlists and Tracks:

```tsx
<NavItem to="/genres" icon={<Guitar size={18} />} label={t('nav.genres')} testId="nav-genres" />
```

`i18n` key: `nav.genres` → `"Genres"` (English)

### GenresListPage

New file: `applications/shared/src/pages/GenresListPage.tsx`

- Route: `/genres` (registered in desktop `App.tsx`)
- Calls `get_all_genres()` on mount
- Renders responsive grid of genre cards: genre name + `"N tracks"` badge
- Each card navigates to `/genres/:id`
- Empty state: `"No genres found"` message
- `data-testid`: `genres-page`, `genre-card-{id}`

### "More filters" panel (Albums + Tracks pages)

Both `AlbumsPage.tsx` and `TracksPage.tsx` get the same pattern added to their search header:

```
[ Search input _________________ ] [ Filters ⌄ ]
---------------------------------------------------
  [ Rock × ]  [ Jazz ]  [ Electronic ]   (expanded)
```

- `Filters` button with `SlidersHorizontal` icon; shows an active indicator (blue dot) when a genre is selected
- Clicking toggles a chip row; chips loaded from `get_all_genres()` (cached, only fetched once per mount)
- **Single-select**: clicking a chip activates it and re-fetches the list filtered by genre; clicking the active chip again clears the filter and re-fetches all
- Albums with genre active → call `getGenreAlbums(genreId)` instead of `getAllAlbums()`
- Tracks with genre active → call `get_genre_tracks(genreId)` (already a Tauri command) instead of `getAllTracks()`
- Filter state is local (`useState`), not persisted
- `data-testid`: `filter-toggle-button`, `genre-chip-{id}`, `genre-chip-{id}-active`

### Routing

`applications/desktop/src/App.tsx`: add `<Route path="/genres" element={<GenresListPage />} />`
(The `/genres/:id` route for `GenrePage` already exists.)

### E2E — `genre.spec.js` (4 tests)

Seed already includes genre 4001 "Playwright Genre" linked to all 5 tracks. No seed changes needed.

| # | Test | Assert |
|---|------|--------|
| 1 | Click `nav-genres` | `genres-page` visible, `genre-card-4001` shows "Playwright Genre" with "5 tracks" |
| 2 | Click `genre-card-4001` | Navigates to `/genres/4001`, 5 track rows in TrackList |
| 3 | Albums page → Filters → click "Playwright Genre" chip | Only album 2001 visible |
| 4 | Tracks page → Filters → click "Playwright Genre" chip | 5 tracks visible |

---

## 2. Playback Error Handling

### Current state

`usePlaybackEvents.ts` already listens to `'playback:error'`:
```typescript
const unlistenError = await listen<string>('playback:error', (event) => {
  debug.error('[Playback Error]', event.payload);
  // TODO: Show user-facing error notification
});
```

### Change

Move the error handler into `TauriPlayerCommandsProvider.tsx` (has access to both Tauri events and player commands). Remove the TODO stub from `usePlaybackEvents.ts`.

New behavior in the provider:
```typescript
const unlistenError = await listen<string>('playback:error', (event) => {
  toast.error(event.payload ?? 'Playback error');
  // Auto-skip to next track (if queue exhausted, skipNext stops naturally)
  skipNextRef.current?.();
});
```

`toast.error()` from `sonner` — `<Toaster />` is already in `App.tsx`.

`data-testid` on the toast container: sonner injects `data-sonner-toaster`; individual toasts have no testid by default — use `page.getByText()` in tests instead.

### E2E — `playback-errors.spec.js` (3 tests)

No DB changes needed. Tests construct queue objects with a bad `filePath` directly via IPC:

```javascript
const badTrack = {
  trackId: '9999',
  title: 'Bad Track',
  artist: 'Test',
  album: 'Test',
  albumId: 9999,
  filePath: '/nonexistent/fake-track-that-does-not-exist.wav',
  durationSeconds: 2,
  trackNumber: 1,
};
```

| # | Test | Setup | Assert |
|---|------|--------|--------|
| 1 | Single bad track | `play_queue([badTrack])` | Toast with error text appears; playback stops (no hang, no crash) |
| 2 | Bad then good | `play_queue([badTrack, Track One])` | Toast appears; `now-playing-title` shows "Track One" |
| 3 | Good then bad (auto-advance) | `play_queue([Track One, badTrack])` | Track One plays; after natural end, toast appears |

---

## 3. Settings Persistence

### No production code changes

The settings system is complete. Tests exercise the existing IPC commands:
- `set_user_setting(key, value)` — upserts a JSON value
- `get_user_setting(key)` — reads back a single key
- `get_user_settings()` — returns `UserSetting[]` for all keys

### E2E — `settings-persistence.spec.js` (6 tests)

`beforeEach`: clean up test keys via `set_user_setting` overwrite (idempotent).

| # | Test | Assert |
|---|------|--------|
| 1 | Set `audio.volume` → `75` | `get_user_setting('audio.volume')` === `75` |
| 2 | Set `ui.theme` → `"ocean"` | `get_user_setting('ui.theme')` === `"ocean"` |
| 3 | Set `import.confidence_threshold` → `90` | read-back === `90` |
| 4 | Set `app.auto_update_enabled` → `false` | read-back === `false` |
| 5 | Set all 4 → navigate Albums → Artists → re-read all 4 | All values unchanged after navigation |
| 6 | `get_user_settings()` after writing all 4 | Returned array contains all 4 keys with correct values |

---

## Files Changed

### Production

| File | Change |
|------|--------|
| `applications/desktop/src-tauri/src/main.rs` | Add `get_genre_albums` command |
| `applications/shared/src/contexts/BackendContext.tsx` | Add `getGenreAlbums()` to interface |
| `applications/desktop/src/providers/TauriBackendProvider.tsx` | Implement `getGenreAlbums()` |
| `applications/shared/src/providers/MockBackendProvider.tsx` | Stub `getGenreAlbums()` returning `[]` |
| `applications/shared/src/components/sidebar/NavBar.tsx` | Add Genres nav item |
| `applications/shared/src/pages/GenresListPage.tsx` | **New** |
| `applications/shared/src/pages/AlbumsPage.tsx` | More filters panel + genre chip filter |
| `applications/shared/src/pages/TracksPage.tsx` | More filters panel + genre chip filter |
| `applications/desktop/src/App.tsx` | Add `/genres` route |
| `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx` | Wire error event → toast + skipNext |
| `applications/shared/src/hooks/usePlaybackEvents.ts` | Remove TODO error stub |

### Tests

| File | Tests |
|------|-------|
| `applications/desktop/e2e-tests/tests/playwright/genre.spec.js` | 4 |
| `applications/desktop/e2e-tests/tests/playwright/playback-errors.spec.js` | 3 |
| `applications/desktop/e2e-tests/tests/playwright/settings-persistence.spec.js` | 6 |

**Total new E2E tests: 13**
