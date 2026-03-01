# Playlist "Add to" Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose "Add to Playlist" from album/artist/playlist cards, the album detail page header, and wire up the left sidebar heart button; fix the dialog width/layout; write E2E tests.

**Architecture:** Extend `AddToPlaylistDialog` to a discriminated union (`mode: 'track' | 'entity'`); in entity mode the dialog fetches tracks internally and shows a batch-add flow. Add `onAddToPlaylist?` prop to `MediaCard` (shows a `ListPlus` button on hover, bottom-right of artwork). Thin wrappers `AlbumCard`/`ArtistCard`/`PlaylistCard` pass the prop through. Pages manage a single `entityForPlaylist` state + one dialog instance.

**Tech Stack:** React 18 + TypeScript + Tailwind v4 + Radix UI + react-i18next + WebdriverIO E2E

---

## Task 1: Extend `AddToPlaylistDialog` — discriminated union props + layout fixes

**Files:**
- Modify: `applications/shared/src/components/AddToPlaylistDialog.tsx`
- Modify: `applications/shared/src/i18n/en-US.json`
- Modify: `applications/shared/src/i18n/de.json`
- Modify: `applications/shared/src/i18n/ja.json`

### Step 1: Add new i18n keys to all three locale files

In `en-US.json`, inside the `"playlist"` object, add:
```json
"addAlbumToPlaylist": "Add Album to Playlist",
"addArtistToPlaylist": "Add Artist to Playlist",
"addPlaylistToPlaylist": "Add Playlist to Playlist",
"addingAllTracks": "Adding all tracks"
```

Add the same keys with the same English values to `de.json` and `ja.json` (translators will update later).

### Step 2: Replace `AddToPlaylistDialog.tsx` entirely

The new file keeps all existing track-mode logic and adds entity mode:

```tsx
import { useState, useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, ListMusic, Plus, Check, X } from 'lucide-react';
import { Dialog, DialogContent, DialogHeader, DialogBody, DialogFooter } from './ui/Dialog';
import { ArtworkImage } from './ArtworkImage';
import { useBackend, type BackendPlaylist } from '../contexts/BackendContext';
import { useAddTrackToPlaylist, useRemoveTrackFromPlaylist, useCreatePlaylist } from '../hooks/queries/usePlaylistMutations';
import { usePlatform } from '../contexts/PlatformContext';
import { debug } from '../utils/debug';

// Discriminated union: track mode (single track add/remove) OR entity mode (batch add)
type AddToPlaylistDialogProps = {
  open: boolean;
  onClose: () => void;
} & (
  | { mode: 'track'; trackId: number; trackTitle?: string }
  | { mode: 'entity'; entityType: 'album' | 'artist' | 'playlist'; entityId: number | string; entityName?: string }
);

export function AddToPlaylistDialog(props: AddToPlaylistDialogProps) {
  const { open, onClose } = props;
  const { t } = useTranslation();
  const backend = useBackend();
  const { isDesktop } = usePlatform();
  const [playlists, setPlaylists] = useState<BackendPlaylist[]>([]);
  // track mode: which playlists originally contained the track
  const [containingPlaylistIds, setContainingPlaylistIds] = useState<Set<string>>(new Set());
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [newPlaylistName, setNewPlaylistName] = useState('');
  const [showNewPlaylistInput, setShowNewPlaylistInput] = useState(false);

  const addTrackMutation = useAddTrackToPlaylist();
  const removeTrackMutation = useRemoveTrackFromPlaylist();
  const createPlaylistMutation = useCreatePlaylist();

  // Dialog title based on mode
  const dialogTitle = useMemo(() => {
    if (props.mode === 'track') return t('playlist.addToPlaylist', 'Add to Playlist');
    switch (props.entityType) {
      case 'album': return t('playlist.addAlbumToPlaylist', 'Add Album to Playlist');
      case 'artist': return t('playlist.addArtistToPlaylist', 'Add Artist to Playlist');
      case 'playlist': return t('playlist.addPlaylistToPlaylist', 'Add Playlist to Playlist');
    }
  }, [props, t]);

  // Load playlists (+ track-mode: which playlists contain the track)
  useEffect(() => {
    if (!open) return;
    const load = async () => {
      setIsLoading(true);
      setSelectedIds(new Set());
      setContainingPlaylistIds(new Set());
      try {
        if (!backend.getAllPlaylists) { setIsLoading(false); return; }
        if (props.mode === 'track') {
          const [playlistsResult, containingIds] = await Promise.all([
            backend.getAllPlaylists(),
            backend.getPlaylistsContainingTrack!(props.trackId),
          ]);
          setPlaylists(playlistsResult);
          const set = new Set(containingIds);
          setContainingPlaylistIds(set);
          setSelectedIds(new Set(set));
        } else {
          const playlistsResult = await backend.getAllPlaylists();
          setPlaylists(playlistsResult);
          // No pre-selection for entity mode
        }
      } catch (err) {
        debug.error('[AddToPlaylistDialog] Failed to load:', err);
      } finally {
        setIsLoading(false);
      }
    };
    load();
    setSearchQuery('');
    setNewPlaylistName('');
    setShowNewPlaylistInput(false);
  }, [open, props.mode === 'track' ? (props as any).trackId : (props as any).entityId, backend, props.mode]);

  const filteredPlaylists = useMemo(() => {
    if (!searchQuery.trim()) return playlists;
    const q = searchQuery.toLowerCase();
    return playlists.filter(p => p.name.toLowerCase().includes(q) || p.description?.toLowerCase().includes(q));
  }, [playlists, searchQuery]);

  const togglePlaylist = useCallback((id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }, []);

  const handleCreatePlaylist = async () => {
    if (!newPlaylistName.trim()) return;
    createPlaylistMutation.mutate({ name: newPlaylistName.trim() }, {
      onSuccess: (p) => {
        setPlaylists(prev => [p, ...prev]);
        setSelectedIds(prev => new Set([...prev, p.id]));
        setNewPlaylistName('');
        setShowNewPlaylistInput(false);
      },
      onError: (err) => debug.error('Failed to create playlist:', err),
    });
  };

  const handleSave = async () => {
    if (props.mode === 'track') {
      // Existing track mode: diff-based add/remove
      const trackId = props.trackId;
      const toAdd = Array.from(selectedIds).filter(id => !containingPlaylistIds.has(id));
      const toRemove = Array.from(containingPlaylistIds).filter(id => !selectedIds.has(id));
      let pending = toAdd.length + toRemove.length;
      if (pending === 0) { onClose(); return; }
      let hasError = false;
      const onDone = () => { pending--; if (pending === 0 && !hasError) onClose(); };
      const onErr = (err: unknown) => { hasError = true; debug.error('Save failed:', err); pending--; };
      toAdd.forEach(id => addTrackMutation.mutate({ playlistId: id, trackId }, { onSuccess: onDone, onError: onErr }));
      toRemove.forEach(id => removeTrackMutation.mutate({ playlistId: id, trackId }, { onSuccess: onDone, onError: onErr }));
    } else {
      // Entity mode: fetch all entity tracks, then batch-add to selected playlists
      if (selectedIds.size === 0) { onClose(); return; }
      setIsSaving(true);
      try {
        let tracks: Awaited<ReturnType<typeof backend.getAllTracks>> = [];
        if (props.entityType === 'album') tracks = await backend.getAlbumTracks(Number(props.entityId));
        else if (props.entityType === 'artist') tracks = await backend.getArtistTracks(Number(props.entityId));
        else if (props.entityType === 'playlist') tracks = await backend.getPlaylistTracks(String(props.entityId));

        const trackIds = tracks.map(t => t.id);
        await Promise.all(
          Array.from(selectedIds).flatMap(playlistId =>
            trackIds.map(trackId =>
              backend.addTrackToPlaylist!(playlistId, trackId).catch(err =>
                debug.error(`Failed to add track ${trackId} to playlist ${playlistId}:`, err)
              )
            )
          )
        );
        onClose();
      } catch (err) {
        debug.error('[AddToPlaylistDialog] Entity batch add failed:', err);
      } finally {
        setIsSaving(false);
      }
    }
  };

  const hasChanges = useMemo(() => {
    if (props.mode === 'entity') return selectedIds.size > 0;
    if (selectedIds.size !== containingPlaylistIds.size) return true;
    return Array.from(selectedIds).some(id => !containingPlaylistIds.has(id));
  }, [selectedIds, containingPlaylistIds, props.mode]);

  return (
    <Dialog open={open} onClose={onClose}>
      <DialogContent className="max-w-md" data-testid="add-to-playlist-dialog">
        <DialogHeader onClose={onClose}>{dialogTitle}</DialogHeader>
        <DialogBody>
          {/* Context label */}
          {props.mode === 'track' && props.trackTitle && (
            <div className="mb-4 p-3 rounded-lg bg-muted/50">
              <p className="text-sm text-muted-foreground">{t('playlist.addingTrack', 'Adding track')}</p>
              <p className="font-medium truncate">{props.trackTitle}</p>
            </div>
          )}
          {props.mode === 'entity' && props.entityName && (
            <div className="mb-4 p-3 rounded-lg bg-muted/50">
              <p className="text-sm text-muted-foreground">{t('playlist.addingAllTracks', 'Adding all tracks')}</p>
              <p className="font-medium truncate">{props.entityName}</p>
            </div>
          )}

          {/* Search */}
          <div className="relative mb-4">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder={t('playlist.searchPlaylists', 'Search playlists...')}
              className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
            />
            {searchQuery && (
              <button onClick={() => setSearchQuery('')} className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]">
                <X className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* Create new playlist */}
          {showNewPlaylistInput ? (
            <div className="flex items-center gap-2 mb-4">
              <input
                type="text"
                value={newPlaylistName}
                onChange={e => setNewPlaylistName(e.target.value)}
                placeholder={t('playlist.newPlaylistName', 'New Playlist')}
                className="flex-1 px-3 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
                autoFocus
                onKeyDown={e => { if (e.key === 'Enter') handleCreatePlaylist(); if (e.key === 'Escape') setShowNewPlaylistInput(false); }}
              />
              <button onClick={handleCreatePlaylist} disabled={!newPlaylistName.trim()} className="px-3 py-2 bg-primary text-primary-foreground rounded-lg text-sm font-medium hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]">
                {t('common.save', 'Save')}
              </button>
              <button onClick={() => { setShowNewPlaylistInput(false); setNewPlaylistName(''); }} className="p-2 hover:bg-foreground/[var(--hover-bg-opacity)] rounded-lg transition-colors duration-[var(--transition-duration)]">
                <X className="w-4 h-4" />
              </button>
            </div>
          ) : (
            <button onClick={() => setShowNewPlaylistInput(true)} className="flex items-center gap-2 w-full px-3 py-2 mb-4 rounded-lg border border-dashed border-border hover:border-primary hover:bg-foreground/[var(--hover-bg-opacity)] transition-all duration-[var(--transition-duration)] text-sm text-muted-foreground hover:opacity-[var(--hover-text-opacity)]">
              <Plus className="w-4 h-4" />
              {t('playlist.createNew', 'Create new playlist')}
            </button>
          )}

          {/* Playlist list */}
          <div className="max-h-80 overflow-y-auto -mx-6 px-6">
            {isLoading ? (
              <div className="flex items-center justify-center py-8 text-muted-foreground">
                <div className="animate-spin w-5 h-5 border-2 border-current border-t-transparent rounded-full" />
              </div>
            ) : filteredPlaylists.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                {searchQuery ? <p>{t('library.noSearchResults', 'No results found')}</p> : (
                  <><ListMusic className="w-8 h-8 mx-auto mb-2 opacity-50" /><p>{t('playlist.noPlaylists', 'No playlists yet')}</p></>
                )}
              </div>
            ) : (
              <div className="space-y-1">
                {filteredPlaylists.map(playlist => {
                  const isSelected = selectedIds.has(playlist.id);
                  const wasInPlaylist = containingPlaylistIds.has(playlist.id);
                  return (
                    <button
                      key={playlist.id}
                      data-testid="playlist-dialog-item"
                      onClick={() => togglePlaylist(playlist.id)}
                      className={`w-full flex items-center gap-3 p-3 rounded-lg transition-colors duration-[var(--transition-duration)] text-left ${isSelected ? 'bg-primary/10 border border-primary/30' : 'hover:bg-foreground/[var(--hover-bg-opacity)] border border-transparent'}`}
                    >
                      {/* Playlist artwork */}
                      <div className="w-10 h-10 rounded-md bg-muted flex items-center justify-center flex-shrink-0 overflow-hidden">
                        {isDesktop ? (
                          <ArtworkImage
                            playlistId={playlist.id}
                            alt={playlist.name}
                            className="w-full h-full object-cover"
                            fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
                            fallbackIcon="playlist"
                          />
                        ) : (
                          <ListMusic className="w-4 h-4 text-muted-foreground" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-medium truncate">{playlist.name}</p>
                        <p className="text-sm text-muted-foreground">
                          {t('library.tracks', '{{count}} tracks', { count: playlist.track_count })}
                        </p>
                      </div>
                      <div className={`w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0 transition-colors ${isSelected ? 'bg-primary text-primary-foreground' : 'border-2 border-muted-foreground/30'}`}>
                        {isSelected && <Check className="w-4 h-4" />}
                      </div>
                      {wasInPlaylist && !isSelected && (
                        <span className="text-xs text-muted-foreground ml-2">{t('playlist.willRemove', 'will remove')}</span>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </DialogBody>
        <DialogFooter>
          <button onClick={onClose} className="px-4 py-2 text-sm rounded-lg border border-border hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]">
            {t('common.cancel', 'Cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={!hasChanges || isSaving || addTrackMutation.isPending || removeTrackMutation.isPending}
            className="px-4 py-2 text-sm rounded-lg bg-primary text-primary-foreground hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
          >
            {isSaving || addTrackMutation.isPending || removeTrackMutation.isPending ? t('common.saving', 'Saving...') : t('common.done', 'Done')}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

### Step 3: Check TypeScript compiles

```bash
cargo xtask check typescript
```
Expected: `✓ Shared TypeScript OK` (and desktop, marketing)

### Step 4: Fix `MainLayout.tsx` to use `mode="track"`

In `applications/desktop/src/layouts/MainLayout.tsx` lines 114–121, update the `<AddToPlaylistDialog>` call to add `mode="track"`:

```tsx
{currentTrack && showAddToPlaylist && (
  <AddToPlaylistDialog
    open={showAddToPlaylist}
    onClose={() => setShowAddToPlaylist(false)}
    mode="track"
    trackId={currentTrack.id}
    trackTitle={currentTrack.title}
  />
)}
```

### Step 5: TypeScript check again + commit

```bash
cargo xtask check typescript
git add applications/shared/src/components/AddToPlaylistDialog.tsx \
        applications/shared/src/i18n/en-US.json \
        applications/shared/src/i18n/de.json \
        applications/shared/src/i18n/ja.json \
        applications/desktop/src/layouts/MainLayout.tsx
git commit -m "feat(playlist): extend dialog to entity mode, fix width/height/artwork"
```

---

## Task 2: Add `onAddToPlaylist` prop + ListPlus button to `MediaCard`

**Files:**
- Modify: `applications/shared/src/components/MediaCard.tsx`

### Step 1: Add import and prop

At the top of `MediaCard.tsx`, add `ListPlus` to the lucide import (line 8):
```tsx
import { Play, Pause, Disc3, Users, ListMusic, ListPlus } from 'lucide-react'
```

Add `onAddToPlaylist?: () => void` to `MediaCardProps` interface (after `priority?: boolean`):
```tsx
/** When provided, shows an "Add to Playlist" button on hover in the bottom-right of the artwork */
onAddToPlaylist?: () => void
```

Add it to the destructured props in `MediaCardComponent` (line 69–79):
```tsx
const MediaCardComponent = ({
  ...existing props...
  onAddToPlaylist,
}: MediaCardProps) => {
```

### Step 2: Add the button inside the artwork div

In the artwork `div` (around line 250–273), after the existing play/pause button, add the ListPlus button. It sits at the bottom-right of the artwork overlay:

```tsx
{/* Add to Playlist button - bottom right, visible on hover */}
{onAddToPlaylist && (
  <button
    onClick={(e) => { e.stopPropagation(); onAddToPlaylist(); }}
    onMouseDown={(e) => e.preventDefault()}
    data-testid="media-card-add-to-playlist-button"
    className="absolute bottom-2 right-2 w-8 h-8 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-lg opacity-0 group-hover:opacity-100 transition-all duration-200"
    aria-label={t('playlist.addToPlaylist')}
  >
    <ListPlus className="w-4 h-4 text-white" />
  </button>
)}
```

### Step 3: TypeScript check + commit

```bash
cargo xtask check typescript
git add applications/shared/src/components/MediaCard.tsx
git commit -m "feat(cards): add onAddToPlaylist prop + ListPlus button to MediaCard"
```

---

## Task 3: Pass `onAddToPlaylist` through card wrapper components

**Files:**
- Modify: `applications/shared/src/components/AlbumCard.tsx`
- Modify: `applications/shared/src/components/ArtistCard.tsx`
- Modify: `applications/shared/src/components/PlaylistCard.tsx`

### Step 1: Update `AlbumCard.tsx`

Add `onAddToPlaylist?: () => void` to `AlbumCardProps` interface, and pass it to `<MediaCard>`:

```tsx
interface AlbumCardProps {
  album: AlbumCardAlbum
  className?: string
  showArtist?: boolean
  priority?: boolean
  onAddToPlaylist?: () => void  // NEW
}

const AlbumCardComponent = ({ album, className = 'w-full', showArtist = true, priority = false, onAddToPlaylist }: AlbumCardProps) => {
  return (
    <MediaCard
      ...existing props...
      onAddToPlaylist={onAddToPlaylist}  // NEW
    />
  )
}
```

### Step 2: Update `ArtistCard.tsx`

Same pattern:
```tsx
interface ArtistCardProps {
  artist: BackendArtist
  className?: string
  priority?: boolean
  onAddToPlaylist?: () => void  // NEW
}

export function ArtistCard({ artist, className = 'w-full', priority = false, onAddToPlaylist }: ArtistCardProps) {
  ...
  return (
    <MediaCard
      ...existing props...
      onAddToPlaylist={onAddToPlaylist}  // NEW
    />
  )
}
```

### Step 3: Update `PlaylistCard.tsx`

Same pattern:
```tsx
interface PlaylistCardProps {
  playlist: BackendPlaylist
  className?: string
  priority?: boolean
  onAddToPlaylist?: () => void  // NEW
}

export function PlaylistCard({ playlist, className = 'w-full', priority = false, onAddToPlaylist }: PlaylistCardProps) {
  ...
  return (
    <MediaCard
      ...existing props...
      onAddToPlaylist={onAddToPlaylist}  // NEW
    />
  )
}
```

### Step 4: TypeScript check + commit

```bash
cargo xtask check typescript
git add applications/shared/src/components/AlbumCard.tsx \
        applications/shared/src/components/ArtistCard.tsx \
        applications/shared/src/components/PlaylistCard.tsx
git commit -m "feat(cards): pass onAddToPlaylist through AlbumCard, ArtistCard, PlaylistCard"
```

---

## Task 4: Wire `AlbumsPage` — state + dialog

**Files:**
- Modify: `applications/shared/src/pages/AlbumsPage.tsx`

### Step 1: Add imports and state

At the top of `AlbumsPage.tsx`, add:
```tsx
import { useState } from 'react'  // already imported, add useState if not there
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
```

Inside `AlbumsPage()`, after existing state declarations, add:
```tsx
const [entityForPlaylist, setEntityForPlaylist] = useState<{
  id: number | string
  name: string
} | null>(null)
```

### Step 2: Pass `onAddToPlaylist` to each `AlbumCard`

In both the virtualized render (`renderItem`) and the non-virtualized `map`, update `<AlbumCard>` to include:
```tsx
onAddToPlaylist={() => setEntityForPlaylist({ id: album.id, name: album.title })}
```

Example for the non-virtualized map (lines ~116–124):
```tsx
{filteredAlbums.map((album, index) => (
  <AlbumCard
    key={album.id}
    album={album}
    showArtist={true}
    priority={index < 24}
    onAddToPlaylist={() => setEntityForPlaylist({ id: album.id, name: album.title })}
  />
))}
```

Do the same in the `renderItem` callback (lines ~107–113).

### Step 3: Render the dialog

Just before the closing `</LibraryPageLayout>` tag (or after it, outside), add:
```tsx
{entityForPlaylist && (
  <AddToPlaylistDialog
    open={!!entityForPlaylist}
    onClose={() => setEntityForPlaylist(null)}
    mode="entity"
    entityType="album"
    entityId={entityForPlaylist.id}
    entityName={entityForPlaylist.name}
  />
)}
```

### Step 4: TypeScript check + commit

```bash
cargo xtask check typescript
git add applications/shared/src/pages/AlbumsPage.tsx
git commit -m "feat(albums): wire add-to-playlist on album cards"
```

---

## Task 5: Wire `ArtistsPage` — state + dialog

**Files:**
- Modify: `applications/shared/src/pages/ArtistsPage.tsx`

Exact same pattern as Task 4, but for artists. Key differences:
- State: `entityForPlaylist: { id: number, name: string } | null`
- Pass: `onAddToPlaylist={() => setEntityForPlaylist({ id: artist.id, name: artist.name })}`
- Dialog: `entityType="artist"` and `entityId={entityForPlaylist.id}`

Find where `<ArtistCard>` is rendered in both virtualized and non-virtualized branches and add the prop. Add the dialog after.

```bash
cargo xtask check typescript
git add applications/shared/src/pages/ArtistsPage.tsx
git commit -m "feat(artists): wire add-to-playlist on artist cards"
```

---

## Task 6: Wire `PlaylistsPage` — state + dialog + testid on create button

**Files:**
- Modify: `applications/shared/src/pages/PlaylistsPage.tsx`

### Step 1: Add `data-testid` to the existing "Create Playlist" button

Find the button (around line 117–123):
```tsx
<button
  onClick={handleCreatePlaylist}
  className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
>
```

Add `data-testid="playlist-create-button"` to it.

### Step 2: Add state + pass prop + render dialog

Same pattern as Tasks 4–5:
- State: `entityForPlaylist: { id: string, name: string } | null`
- Pass: `onAddToPlaylist={() => setEntityForPlaylist({ id: playlist.id, name: playlist.name })}` to each `<PlaylistCard>`
- Dialog: `entityType="playlist"`, `entityId={entityForPlaylist.id}`

```bash
cargo xtask check typescript
git add applications/shared/src/pages/PlaylistsPage.tsx
git commit -m "feat(playlists): wire add-to-playlist on playlist cards, testid on create btn"
```

---

## Task 7: Add "Add to Playlist" button to `AlbumPage` header

**Files:**
- Modify: `applications/shared/src/pages/AlbumPage.tsx`

### Step 1: Add `ListPlus` to imports (line 11)

```tsx
import { ArrowLeft, Play, Clock, Disc3, Pencil, ListPlus } from 'lucide-react'
```

### Step 2: Add state for album-level dialog

After the existing `selectedTrackForPlaylist` state (lines 42–46), add:
```tsx
const [albumForPlaylist, setAlbumForPlaylist] = useState(false)
```

### Step 3: Add `ListPlus` button next to "Play All" (line 258)

Replace the existing button+close block (lines 258–266):
```tsx
<div className="flex items-center gap-3">
  <button
    onClick={handlePlayAll}
    onMouseDown={(e) => e.preventDefault()}
    disabled={tracks.filter(t => t.file_path).length === 0}
    className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:opacity-[var(--hover-button-opacity)] transition-opacity disabled:opacity-[var(--disabled-opacity)]"
  >
    <Play className="w-5 h-5" fill="currentColor" />
    <span>{t('common.playAll')}</span>
  </button>

  {features.canCreatePlaylists && (
    <button
      onClick={() => setAlbumForPlaylist(true)}
      data-testid="album-page-add-to-playlist"
      className="flex items-center gap-2 px-4 py-3 rounded-full border border-border hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
      title={t('playlist.addAlbumToPlaylist', 'Add Album to Playlist')}
    >
      <ListPlus className="w-5 h-5" />
    </button>
  )}
</div>
```

Note: `features` is already destructured from `usePlatform()` in this file.

### Step 4: Add the album-level dialog

Near the bottom of the return, alongside the existing `AddToPlaylistDialog` for tracks, add:
```tsx
{album && albumForPlaylist && (
  <AddToPlaylistDialog
    open={albumForPlaylist}
    onClose={() => setAlbumForPlaylist(false)}
    mode="entity"
    entityType="album"
    entityId={album.id}
    entityName={album.title}
  />
)}
```

### Step 5: TypeScript check + commit

```bash
cargo xtask check typescript
git add applications/shared/src/pages/AlbumPage.tsx
git commit -m "feat(album-page): add 'Add Album to Playlist' button in header"
```

---

## Task 8: Full pre-commit check

```bash
cargo xtask check precommit
```

Expected: all checks pass (fmt, clippy, TS, lint). Fix any issues found.

```bash
git add -A
git commit -m "chore: fix any lint/format issues post-feature"
```

---

## Task 9: Write E2E playlist test config

**Files:**
- Create: `applications/desktop/e2e-tests/wdio.playlists.conf.js`

```js
/**
 * WebdriverIO config for Playlist E2E Tests
 *
 * Seeds: 1 artist, 1 album (5 tracks), 1 pre-existing empty playlist "Favorites"
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.playlists.conf.js
 */

import { config as baseConfig } from './wdio.conf.js';
import { mkdirSync, rmSync, readFileSync, readdirSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { fileURLToPath } from 'url';
import Database from 'better-sqlite3';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

let testDbPath = null;
let testDbDir = null;

function createSilentWavBuffer(durationSeconds = 1) {
  const sampleRate = 44100;
  const channels = 1;
  const bitsPerSample = 16;
  const numSamples = Math.floor(sampleRate * durationSeconds * channels);
  const dataSize = numSamples * (bitsPerSample / 8);
  const fileSize = 36 + dataSize;
  const buffer = Buffer.alloc(44 + dataSize, 0);
  let offset = 0;
  buffer.write('RIFF', offset); offset += 4;
  buffer.writeUInt32LE(fileSize, offset); offset += 4;
  buffer.write('WAVE', offset); offset += 4;
  buffer.write('fmt ', offset); offset += 4;
  buffer.writeUInt32LE(16, offset); offset += 4;
  buffer.writeUInt16LE(1, offset); offset += 2;
  buffer.writeUInt16LE(channels, offset); offset += 2;
  buffer.writeUInt32LE(sampleRate, offset); offset += 4;
  buffer.writeUInt32LE(sampleRate * channels * (bitsPerSample / 8), offset); offset += 4;
  buffer.writeUInt16LE(channels * (bitsPerSample / 8), offset); offset += 2;
  buffer.writeUInt16LE(bitsPerSample, offset); offset += 2;
  buffer.write('data', offset); offset += 4;
  buffer.writeUInt32LE(dataSize, offset);
  return buffer;
}

function setupTestDatabase() {
  console.log('[Playlists E2E Setup] Creating test environment...');
  const timestamp = Date.now();
  testDbDir = join(tmpdir(), `soul-player-playlists-e2e-${timestamp}`);
  const audioDir = join(testDbDir, 'audio');
  mkdirSync(audioDir, { recursive: true });

  const wavPath = join(audioDir, 'test-track.wav');
  writeFileSync(wavPath, createSilentWavBuffer(2));

  testDbPath = join(testDbDir, 'test.db');
  const migrationsDir = join(__dirname, '../../../libraries/soul-storage/migrations');
  const db = new Database(testDbPath);

  try {
    const migrationFiles = readdirSync(migrationsDir).filter(f => f.endsWith('.sql')).sort();
    for (const file of migrationFiles) {
      db.exec(readFileSync(join(migrationsDir, file), 'utf-8'));
    }

    const now = Math.floor(Date.now() / 1000);
    db.prepare('INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)').run('1', 'Test User', now);
    db.prepare('INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)').run(1, 'Local', 'local');

    // Artist + album
    db.prepare('INSERT INTO artists (id, name) VALUES (?, ?)').run(2001, 'E2E Playlist Artist');
    db.prepare('INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)').run(2001, 'Playlist Test Album', 2001, 2022);

    const titles = ['Track One', 'Track Two', 'Track Three', 'Track Four', 'Track Five'];
    titles.forEach((title, i) => {
      const trackId = 2001 + i;
      db.prepare('INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format) VALUES (?, ?, ?, ?, ?, ?, ?, ?)').run(trackId, title, 2001, 2001, i + 1, 1, 2.0, 'wav');
      db.prepare('INSERT INTO track_sources (track_id, source_id, status, local_file_path) VALUES (?, ?, ?, ?)').run(trackId, 1, 'available', wavPath);
    });

    // Pre-existing empty playlist
    db.prepare('INSERT INTO playlists (id, name, owner_id, is_public, is_favorite, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)').run('3001', 'Favorites', '1', 0, 0, now, now);

    console.log('[Playlists E2E Setup] ✓ Seeded 5 tracks, 1 playlist');
  } finally {
    db.close();
  }
  return testDbPath;
}

function cleanupTestDatabase() {
  if (testDbDir) {
    try { rmSync(testDbDir, { recursive: true, force: true }); } catch {}
  }
}

export const config = {
  ...baseConfig,
  specs: ['./tests/specs/playlists.e2e.js'],
  mochaOpts: { ...baseConfig.mochaOpts, timeout: 90000 },
  onPrepare: async function (config, capabilities) {
    testDbPath = setupTestDatabase();
    process.env.DATABASE_PATH = testDbPath;
    if (baseConfig.onPrepare) await baseConfig.onPrepare(config, capabilities);
  },
  before: async function () {
    await browser.pause(4000);
  },
  onComplete: async function (exitCode, config, capabilities, results) {
    cleanupTestDatabase();
    if (baseConfig.onComplete) await baseConfig.onComplete(exitCode, config, capabilities, results);
  },
  afterTest: async function (test, context, { error }) {
    if (error) {
      const ts = new Date().toISOString().replace(/[:.]/g, '-');
      try { await browser.saveScreenshot(`./screenshots/playlists-${ts}.png`); } catch {}
    }
  },
};
```

### Commit config

```bash
git add applications/desktop/e2e-tests/wdio.playlists.conf.js
git commit -m "test(e2e): add playlist E2E config with seeded DB"
```

---

## Task 10: Write E2E playlist test spec

**Files:**
- Create: `applications/desktop/e2e-tests/tests/specs/playlists.e2e.js`

```js
/**
 * Playlist E2E Tests
 *
 * Covers: create playlist, add track, navigate detail, play, add album batch.
 *
 * Run: cd applications/desktop/e2e-tests && npm test -- --config wdio.playlists.conf.js
 *
 * Prerequisites:
 * - Built app: cargo build --release -p soul-player-desktop
 * - tauri-driver: cargo install tauri-driver
 */

// ---- Helpers ----

async function waitForEl(selector, desc, timeout = 15000) {
  const el = await $(selector);
  try { await el.waitForExist({ timeout }); } catch {
    throw new Error(`Element not found: ${desc}\nSelector: ${selector}`);
  }
  return el;
}

async function navigateTo(navId) {
  const btn = await waitForEl(`[data-testid="nav-${navId}"]`, `nav-${navId}`);
  await btn.waitForClickable({ timeout: 5000 });
  await btn.click();
  await browser.pause(1500);
}

async function hoverElement(el) {
  await el.moveTo();
  await browser.pause(300);
}

async function waitForNowPlaying(timeout = 12000) {
  return waitForEl('[data-testid="now-playing-title"]', 'now-playing-title', timeout);
}

// ---- Tests ----

describe('Playlist: Create', () => {
  it('creates a new playlist from PlaylistsPage', async () => {
    await navigateTo('playlists');

    const createBtn = await waitForEl('[data-testid="playlist-create-button"]', 'Create Playlist button');
    await createBtn.click();
    await browser.pause(1000);

    // Should have navigated to a new playlist detail page
    const url = await browser.getUrl();
    expect(url).toContain('/playlists/');
  });
});

describe('Playlist: Add track via TrackMenu', () => {
  before(async () => {
    // Navigate to album detail page
    await navigateTo('albums');
    const albumCard = await waitForEl('[data-testid="media-card-album-2001"]', 'album card');
    await albumCard.click();
    await browser.pause(1500);
  });

  it('opens Add to Playlist dialog from track menu', async () => {
    const trackList = await waitForEl('[data-testid="track-list"]', 'track list');
    const firstRow = await trackList.$('[data-testid="track-row"]');
    await hoverElement(firstRow);

    const menuBtn = await firstRow.$('[aria-label="Track options"]');
    await menuBtn.click();
    await browser.pause(500);

    // Click "Add to Playlist" in dropdown
    const menuItems = await $$('.radix-dropdown-item, [role="menuitem"]');
    let addToPlaylistItem = null;
    for (const item of menuItems) {
      const text = await item.getText();
      if (text.includes('Playlist')) { addToPlaylistItem = item; break; }
    }
    expect(addToPlaylistItem).toBeTruthy();
    await addToPlaylistItem.click();
    await browser.pause(500);
  });

  it('shows the Add to Playlist dialog', async () => {
    const dialog = await waitForEl('[data-testid="add-to-playlist-dialog"]', 'Add to Playlist dialog');
    await expect(dialog).toBeDisplayed();
  });

  it('shows the Favorites playlist in the list', async () => {
    const items = await $$('[data-testid="playlist-dialog-item"]');
    expect(items.length).toBeGreaterThanOrEqual(1);
    const firstText = await items[0].getText();
    expect(firstText).toContain('Favorites');
  });

  it('selects Favorites and saves', async () => {
    const items = await $$('[data-testid="playlist-dialog-item"]');
    await items[0].click();
    await browser.pause(300);

    const doneBtn = await $('button=Done');
    await doneBtn.waitForClickable({ timeout: 3000 });
    await doneBtn.click();
    await browser.pause(1000);

    // Dialog should close
    const dialog = await $('[data-testid="add-to-playlist-dialog"]');
    const exists = await dialog.isExisting();
    expect(exists).toBe(false);
  });
});

describe('Playlist: Navigate to detail', () => {
  it('navigates to playlist detail and shows track', async () => {
    await navigateTo('playlists');
    await browser.pause(500);

    // Find the Favorites playlist card
    const playlistCards = await $$('[data-testid^="media-card-playlist-"]');
    let favoritesCard = null;
    for (const card of playlistCards) {
      const text = await card.getText();
      if (text.includes('Favorites')) { favoritesCard = card; break; }
    }
    expect(favoritesCard).toBeTruthy();
    await favoritesCard.click();
    await browser.pause(1500);

    // Should show at least 1 track
    const trackList = await waitForEl('[data-testid="track-list"]', 'playlist track list');
    const rows = await trackList.$$('[data-testid="track-row"]');
    expect(rows.length).toBeGreaterThanOrEqual(1);
  });
});

describe('Playlist: Play from card', () => {
  before(async () => {
    await navigateTo('playlists');
    await browser.pause(500);
  });

  it('plays playlist from card hover play button', async () => {
    const cards = await $$('[data-testid^="media-card-playlist-"]');
    expect(cards.length).toBeGreaterThan(0);

    // Hover card and click play
    await hoverElement(cards[0]);
    const playBtn = await cards[0].$('[data-testid="media-card-play-button"]');
    await playBtn.waitForClickable({ timeout: 5000 });
    await playBtn.click();

    // Wait for now-playing-title to appear
    const nowPlaying = await waitForNowPlaying(12000);
    await expect(nowPlaying).toBeDisplayed();
  });
});

describe('Playlist: Add album via card button', () => {
  before(async () => {
    await navigateTo('albums');
    await browser.pause(500);
  });

  it('opens Add to Playlist dialog from album card button', async () => {
    const albumCard = await waitForEl('[data-testid="media-card-album-2001"]', 'album card');
    await hoverElement(albumCard);

    const addBtn = await albumCard.$('[data-testid="media-card-add-to-playlist-button"]');
    await addBtn.waitForClickable({ timeout: 5000 });
    await addBtn.click();
    await browser.pause(500);

    const dialog = await waitForEl('[data-testid="add-to-playlist-dialog"]', 'Add to Playlist dialog');
    await expect(dialog).toBeDisplayed();
  });

  it('selects a playlist and saves all album tracks', async () => {
    const items = await $$('[data-testid="playlist-dialog-item"]');
    expect(items.length).toBeGreaterThan(0);
    await items[0].click();
    await browser.pause(300);

    const doneBtn = await $('button=Done');
    await doneBtn.waitForClickable({ timeout: 3000 });
    await doneBtn.click();
    await browser.pause(2000);

    // Dialog should close
    const dialog = await $('[data-testid="add-to-playlist-dialog"]');
    expect(await dialog.isExisting()).toBe(false);
  });
});
```

### Commit spec

```bash
git add applications/desktop/e2e-tests/tests/specs/playlists.e2e.js
git commit -m "test(e2e): add playlist E2E spec (create, add track/album, play, navigate)"
```

---

## Task 11: Final pre-commit + verification

```bash
cargo xtask check precommit
```

All checks must pass. Fix any issues.

If you want to run the E2E tests locally (requires built release binary + tauri-driver):
```bash
# Build first
cargo xtask build desktop --release

# Run playlist E2E
cd applications/desktop/e2e-tests
npm test -- --config wdio.playlists.conf.js
```

---

## Summary of new `data-testid` values

| Testid | Component | Purpose |
|--------|-----------|---------|
| `add-to-playlist-dialog` | `AddToPlaylistDialog` | Dialog root |
| `playlist-dialog-item` | `AddToPlaylistDialog` | Each playlist row |
| `media-card-add-to-playlist-button` | `MediaCard` | ListPlus button on artwork hover |
| `album-page-add-to-playlist` | `AlbumPage` | Header action button |
| `playlist-create-button` | `PlaylistsPage` | "Create Playlist" button |
