# Design: Playlist "Add to" Feature + E2E Tests

**Date:** 2026-03-01
**Status:** Approved

---

## Problem

"Add to Playlist" is only accessible from track row context menus (`TrackMenu`). Users cannot add an entire album, artist, or playlist's tracks to a playlist from card views or album detail pages. The `AddToPlaylistDialog` is too narrow (`max-w-sm` = 384px) and shows generic placeholder icons instead of real playlist artwork.

---

## Approach: Entity-Aware Dialog + MediaCard Button (Approach A)

Extend `AddToPlaylistDialog` to support two modes via a discriminated union, add a hover button to `MediaCard`, wire up album detail page and left sidebar.

---

## Section 1: `AddToPlaylistDialog` Refactor

### Prop Interface

Change from:
```typescript
interface AddToPlaylistDialogProps {
  open: boolean
  onClose: () => void
  trackId: number
  trackTitle?: string
}
```

To a discriminated union:
```typescript
type AddToPlaylistDialogProps = {
  open: boolean
  onClose: () => void
} & (
  | { mode: 'track'; trackId: number; trackTitle?: string }
  | { mode: 'entity'; entityType: 'album' | 'artist' | 'playlist'; entityId: number | string; entityName?: string }
)
```

### Entity Mode Behavior

- Dialog fetches all tracks for the entity internally via `useBackend()` (`getAlbumTracks`, `getArtistTracks`, `getPlaylistTracks`)
- Presents playlists as a checkbox list — no pre-selection (no "already contains" check, too expensive for batch)
- On save: batch-adds all entity tracks to each selected playlist via `addTrackToPlaylist` mutations
- Header: "Add Album to Playlist" / "Add Artist to Playlist" / "Add Playlist to Playlist" (localized)

### Track Mode Behavior

Unchanged — existing add/remove management flow (with `getPlaylistsContainingTrack` pre-selection) stays.

### Layout Fixes

- Width: `max-w-sm` → `max-w-md` (384px → 448px)
- List height: `max-h-64` → `max-h-80` (256px → 320px)
- Playlist row artwork: replace generic `ListMusic` placeholder with real `ArtworkImage` (type: playlist)
- Add `data-testid="add-to-playlist-dialog"` to dialog root
- Add `data-testid="playlist-dialog-item"` to each playlist row button

---

## Section 2: MediaCard + "..." Button

### MediaCard Prop Addition

```typescript
interface MediaCardProps {
  // ... existing props ...
  onAddToPlaylist?: () => void  // When provided, shows ListPlus button on hover
}
```

### Button Placement

- Small `ListPlus` button overlaid on artwork, **bottom-right** corner
- Appears on hover (like the centered play button)
- `data-testid="media-card-add-to-playlist-button"`
- Calls `onAddToPlaylist()` directly — no dropdown (single action = single button)

### Page-Level State (AlbumsPage, ArtistsPage, PlaylistsPage)

Each page adds:
```typescript
const [entityForPlaylist, setEntityForPlaylist] = useState<{
  type: 'album' | 'artist' | 'playlist'
  id: number | string
  name: string
} | null>(null)
```

Passes to each card:
```tsx
<MediaCard
  onAddToPlaylist={() => setEntityForPlaylist({ type: 'album', id: album.id, name: album.title })}
  ...
/>
```

Renders one shared dialog per page:
```tsx
{entityForPlaylist && (
  <AddToPlaylistDialog
    open={!!entityForPlaylist}
    onClose={() => setEntityForPlaylist(null)}
    mode="entity"
    entityType={entityForPlaylist.type}
    entityId={entityForPlaylist.id}
    entityName={entityForPlaylist.name}
  />
)}
```

### AlbumPage Header Button

Add `ListPlus` button alongside existing Play All button. Same entity mode dialog.
`data-testid="album-page-add-to-playlist"`

### Left Sidebar (NowPlayingPanel)

`NowPlayingPanel` already has a Heart button calling `onAddToPlaylist?.()`
`LeftSidebar` already accepts `onAddToPlaylist` prop
Verify `MainLayout` wires this up with dialog state for the current track. If not, add:
- State in `MainLayout`: `addCurrentTrackToPlaylistOpen: boolean`
- Pass `onAddToPlaylist={() => setAddCurrentTrackToPlaylistOpen(true)}` to `LeftSidebar`
- Render `<AddToPlaylistDialog mode="track" trackId={currentTrack.id} .../>` in MainLayout

---

## Section 3: E2E Tests

### New Files

- `applications/desktop/e2e-tests/wdio.playlists.conf.js`
- `applications/desktop/e2e-tests/tests/specs/playlists.e2e.js`

### Database Seed

- 1 user (id=1)
- 1 artist: "E2E Playlist Artist" (id=2001)
- 1 album: "Playlist Test Album" (id=2001) with 5 tracks (ids 2001–2005)
- All tracks point to the same shared silent WAV file
- 1 pre-seeded playlist: "Favorites" (empty, id=3001)

### Test Scenarios

1. **Create playlist** — PlaylistsPage → "New Playlist" button → type name → verify playlist appears in grid
2. **Add track via TrackMenu** — Navigate to album detail → track row "..." → Add to Playlist → select "Favorites" → Done → verify track count increments
3. **Navigate to playlist detail** — Click playlist card → verify track(s) listed
4. **Play playlist from card** — PlaylistsPage → hover card → play button → verify playback starts + now-playing shows track title
5. **Add album via card button** — AlbumsPage → hover album card → `ListPlus` button → select playlist → Done → verify all 5 tracks added

### New data-testid Values

| Component | Testid |
|-----------|--------|
| `AddToPlaylistDialog` root | `add-to-playlist-dialog` |
| Each playlist row in dialog | `playlist-dialog-item` |
| MediaCard ListPlus button | `media-card-add-to-playlist-button` |
| AlbumPage header button | `album-page-add-to-playlist` |
| PlaylistsPage "New Playlist" | `playlist-create-button` (verify/add) |

---

## Files to Touch

| File | Change |
|------|--------|
| `applications/shared/src/components/AddToPlaylistDialog.tsx` | Discriminated union props, entity mode, width/height fix, artwork |
| `applications/shared/src/components/MediaCard.tsx` | `onAddToPlaylist` prop + ListPlus button overlay |
| `applications/shared/src/pages/AlbumsPage.tsx` | State + dialog wiring |
| `applications/shared/src/pages/ArtistsPage.tsx` | State + dialog wiring |
| `applications/shared/src/pages/PlaylistsPage.tsx` | State + dialog wiring |
| `applications/shared/src/pages/AlbumPage.tsx` | ListPlus button in header + dialog state |
| `applications/desktop/src/layouts/MainLayout.tsx` | Wire sidebar heart button to dialog |
| `applications/desktop/e2e-tests/wdio.playlists.conf.js` | New E2E config |
| `applications/desktop/e2e-tests/tests/specs/playlists.e2e.js` | New E2E spec |

---

## Localization Keys Needed

```json
{
  "playlist.addAlbumToPlaylist": "Add Album to Playlist",
  "playlist.addArtistToPlaylist": "Add Artist to Playlist",
  "playlist.addPlaylistToPlaylist": "Add Playlist to Playlist",
  "playlist.addingAll": "Adding all tracks from",
  "playlist.selectPlaylists": "Select playlists to add to"
}
```
