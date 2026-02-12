# Cache Invalidation Flow Diagrams

Visual representation of cache invalidation patterns in Soul Player.

---

## 1. Album Artwork Change Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ User Action: Edit Album Artwork                                │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ useSetArtwork().mutateAsync({                                  │
│   entityType: 'album',                                         │
│   entityId: 42,                                                │
│   artworkBase64: '...',                                        │
│   mimeType: 'image/jpeg'                                       │
│ })                                                             │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Backend: Save artwork to disk/database                         │
│ - Write to Soul Player storage                                 │
│ - Optionally write to album folder                             │
│ - Optionally embed in track metadata                           │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ onSuccess
┌────────────────────────────────────────────────────────────────┐
│ Invalidation Chain                                             │
├────────────────────────────────────────────────────────────────┤
│ 1. queryClient.invalidateQueries(albumKeys.detail(42))         │
│    ├─ Refetches album data with new cover_art_path            │
│    └─ Updates AlbumDetailPage                                  │
│                                                                 │
│ 2. queryClient.invalidateQueries(albumKeys.list())             │
│    ├─ Refetches album grid                                     │
│    └─ Updates thumbnails in LibraryPage                        │
│                                                                 │
│ 3. queryClient.invalidateQueries(artworkKeys.album(42))        │
│    └─ Marks artwork query stale                                │
│                                                                 │
│ 4. clearArtworkCache('album', 42)                              │
│    ├─ Removes cached data URL                                  │
│    └─ Forces re-fetch on next render                           │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ UI Updates                                                      │
├────────────────────────────────────────────────────────────────┤
│ ✓ Album detail page shows new artwork                          │
│ ✓ Album grid thumbnail updates                                 │
│ ✓ Now Playing bar updates (if this album is playing)           │
│ ✓ No browser refresh needed                                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Track Deletion Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ User Action: Delete Track from Album                           │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ useDeleteTrack().mutateAsync(trackId: 123)                     │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Backend: Delete track and return affected entities             │
│ - Delete from database                                         │
│ - Optionally delete file                                       │
│ - Return: {                                                    │
│     albumId: 42,                                               │
│     artistId: 10,                                              │
│     playlistIds: ['playlist-1', 'playlist-2']                  │
│   }                                                            │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ onSuccess(result)
┌────────────────────────────────────────────────────────────────┐
│ Cascade Invalidation                                           │
├────────────────────────────────────────────────────────────────┤
│ 1. Tracks                                                       │
│    ├─ invalidateQueries(trackKeys.all())                       │
│    ├─ Refetches all track lists                                │
│    └─ Updates: LibraryPage, search results, etc.               │
│                                                                 │
│ 2. Album (albumId: 42)                                         │
│    ├─ invalidateQueries(albumKeys.detail(42))                  │
│    ├─ invalidateQueries(albumKeys.tracks(42))                  │
│    ├─ invalidateQueries(albumKeys.lists())                     │
│    └─ Updates: track count, album grid                         │
│                                                                 │
│ 3. Artist (artistId: 10)                                       │
│    ├─ invalidateQueries(artistKeys.detail(10))                 │
│    ├─ invalidateQueries(artistKeys.tracks(10))                 │
│    ├─ invalidateQueries(artistKeys.lists())                    │
│    └─ Updates: track count, artist grid                        │
│                                                                 │
│ 4. Playlists (['playlist-1', 'playlist-2'])                    │
│    ├─ For each playlist:                                       │
│    │   ├─ invalidateQueries(playlistKeys.detail(id))           │
│    │   └─ invalidateQueries(playlistKeys.tracks(id))           │
│    └─ invalidateQueries(playlistKeys.lists())                  │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ UI Updates                                                      │
├────────────────────────────────────────────────────────────────┤
│ ✓ Track removed from album view                                │
│ ✓ Album track count: 10 → 9                                    │
│ ✓ Artist track count: 50 → 49                                  │
│ ✓ Playlists no longer show deleted track                       │
│ ✓ Library track list updated                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. File Scan Complete Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ System Event: Library Scan Completes                           │
│ - Scanned folder: ~/Music                                      │
│ - Found: 50 new files, 10 updated, 5 removed                   │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Backend: Emit Tauri Event                                      │
│ emit_all("scan-complete", { sourceId: 1 })                     │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Frontend: useScanCompletionInvalidation() Hook                 │
│ - Listens for "scan-complete" event                            │
│ - Automatically triggers invalidation                          │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Broad Invalidation (Library-Wide)                              │
├────────────────────────────────────────────────────────────────┤
│ invalidateAfterFileScanComplete(queryClient)                   │
│                                                                 │
│ 1. invalidateQueries(trackKeys.all())                          │
│    └─ All track lists refetch                                  │
│                                                                 │
│ 2. invalidateQueries(albumKeys.all())                          │
│    └─ All album lists/details refetch                          │
│                                                                 │
│ 3. invalidateQueries(artistKeys.all())                         │
│    └─ All artist lists/details refetch                         │
│                                                                 │
│ 4. invalidateQueries(genreKeys.all())                          │
│    └─ Genre lists refetch                                      │
│                                                                 │
│ 5. invalidateQueries(libraryKeys.health())                     │
│    └─ Health stats recalculate                                 │
│                                                                 │
│ Note: Artwork cache NOT cleared (preserves existing artwork)   │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ UI Updates                                                      │
├────────────────────────────────────────────────────────────────┤
│ ✓ New albums appear in grid                                    │
│ ✓ Updated metadata reflects                                    │
│ ✓ Removed albums disappear                                     │
│ ✓ Track counts update                                          │
│ ✓ Genre list refreshes                                         │
│ ✓ No user action required                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Album Metadata Update Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ User Action: Edit Album Title & Year                           │
│ - Title: "Abbey Road" → "Abbey Road (Remastered)"              │
│ - Year: 1969 → 2009                                            │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ useUpdateAlbum().mutateAsync({                                 │
│   albumId: 42,                                                 │
│   title: "Abbey Road (Remastered)",                            │
│   year: 2009                                                   │
│ })                                                             │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ Backend: Update database                                       │
│ UPDATE albums SET title=?, year=? WHERE id=42                  │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ onSuccess
┌────────────────────────────────────────────────────────────────┐
│ Targeted Invalidation                                          │
├────────────────────────────────────────────────────────────────┤
│ 1. Album-specific queries                                      │
│    ├─ invalidateQueries(albumKeys.detail(42))                  │
│    │  └─ Refetch album with new title/year                     │
│    └─ invalidateQueries(albumKeys.tracks(42))                  │
│       └─ Track list might show album metadata                  │
│                                                                 │
│ 2. Album lists (title/year changed)                            │
│    ├─ invalidateQueries(albumKeys.lists())                     │
│    └─ All filtered lists refetch (recently added, etc.)        │
│                                                                 │
│ 3. Track lists (tracks cache album_title)                      │
│    └─ invalidateQueries(trackKeys.all())                       │
│       └─ Track views show updated album title                  │
│                                                                 │
│ 4. If artist changed:                                          │
│    └─ invalidateQueries(artistKeys.albums(newArtistId))        │
│       └─ New artist's album list includes this album           │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ UI Updates                                                      │
├────────────────────────────────────────────────────────────────┤
│ ✓ Album detail shows new title/year                            │
│ ✓ Album grid sorts correctly by new title                      │
│ ✓ Track list shows "Abbey Road (Remastered)"                   │
│ ✓ Filtered views (by year) update                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Query Key Hierarchy Visualization

```
albums (root)
│
├─ list
│  │
│  ├─ [filters: undefined]       ← All albums
│  ├─ random, 20                 ← Random albums
│  ├─ recently-added, 50         ← Recently added
│  ├─ least-played, 30           ← Least played
│  └─ time-capsule, 25           ← Time capsule
│
└─ detail
   │
   └─ 42 (album ID)
      │
      ├─ (album data)            ← Album metadata
      ├─ tracks                  ← Album tracks
      └─ artwork                 ← Album artwork

Invalidation Examples:
├─ invalidateQueries(['albums'])              → Invalidates ALL album queries
├─ invalidateQueries(['albums', 'list'])      → Invalidates all lists, not details
├─ invalidateQueries(['albums', 'detail'])    → Invalidates all details, not lists
├─ invalidateQueries(['albums', 'detail', 42]) → Invalidates only album 42 and sub-keys
└─ invalidateQueries(['albums', 'detail', 42, 'tracks']) → Only album 42's tracks
```

---

## 6. Mutation + Optimistic Update Pattern

```
┌─────────────────────────────────────────────────────────────────┐
│ User Action: Add Track to Playlist                             │
│ (High-frequency operation → needs optimistic update)            │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────┐
│ useAddTrackToPlaylist().mutate({                               │
│   playlistId: 'my-playlist',                                   │
│   trackId: 123                                                 │
│ })                                                             │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ onMutate (BEFORE backend call)
┌────────────────────────────────────────────────────────────────┐
│ Optimistic Update                                              │
├────────────────────────────────────────────────────────────────┤
│ 1. Cancel outgoing refetches                                   │
│    cancelQueries(playlistKeys.tracks('my-playlist'))           │
│                                                                 │
│ 2. Snapshot current state                                      │
│    const previous = getQueryData(playlistKeys.tracks(...))     │
│                                                                 │
│ 3. Update cache optimistically                                 │
│    setQueryData(playlistKeys.tracks(...), old => {             │
│      return [...old, newTrack]                                 │
│    })                                                          │
│                                                                 │
│ 4. Return rollback context                                     │
│    return { previous }                                         │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ UI updates IMMEDIATELY
┌────────────────────────────────────────────────────────────────┐
│ User sees track added instantly (before backend confirms)      │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼ Backend call
┌────────────────────────┬───────────────────────────────────────┐
│                        │                                        │
│      Success           │           Error                        │
│                        │                                        │
▼                        ▼                                        │
┌─────────────────────┐  ┌────────────────────────────────────┐  │
│ onSuccess           │  │ onError (rollback)                 │  │
│ - Invalidate caches │  │ - Restore previous state           │  │
│ - Refetch to sync   │  │   setQueryData(..., context.prev)  │  │
└─────────────────────┘  └────────────────────────────────────┘  │
                         │                                        │
                         ▼                                        │
┌────────────────────────────────────────────────────────────────┐
│ onSettled (always runs)                                        │
│ - Invalidate to ensure sync with server                       │
│   invalidateQueries(playlistKeys.tracks(...))                  │
│   invalidateQueries(playlistKeys.detail(...))                  │
└────────────────────────────────────────────────────────────────┘
```

---

## 7. Artwork Cache + React Query Integration

```
┌─────────────────────────────────────────────────────────────────┐
│ Component: <ArtworkImage albumId={42} />                       │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
                  Check caches
                         │
         ┌───────────────┴───────────────┐
         │                               │
         ▼                               ▼
┌──────────────────────┐        ┌──────────────────────┐
│ Frontend Cache       │        │ React Query Cache    │
│ (artworkCache Map)   │        │ (queryClient)        │
├──────────────────────┤        ├──────────────────────┤
│ Key: 'album:42'      │        │ Key: ['artwork',     │
│ Value: {             │        │       'album', 42]   │
│   dataUrl: '...',    │        │ Value: {             │
│   timestamp: 123456  │        │   dataUrl: '...'     │
│ }                    │        │ }                    │
└──────────────────────┘        └──────────────────────┘
         │                               │
         └───────────────┬───────────────┘
                         │
                    Both caches
                    have data?
                         │
         ┌───────────────┼───────────────┐
         │ YES           │ NO            │
         ▼               ▼               │
    Return cached    Fetch from          │
    data URL         backend             │
                         │               │
                         ▼               │
                  Cache in both          │
                  locations              │
                         │               │
                         └───────────────┘
                         │
                         ▼
                 Render <img>

Invalidation triggers:
├─ clearArtworkCache('album', 42)
│  └─ Clears: Frontend cache (Map)
│
├─ invalidateQueries(artworkKeys.album(42))
│  └─ Marks stale: React Query cache
│
└─ Both triggered by:
   └─ invalidateAfterAlbumArtworkChange(queryClient, 42)
      └─ Ensures complete cache invalidation
```

---

## 8. Event-Driven Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Tauri Backend (Rust)                                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ Scan Worker                                                     │
│ ├─ Scans folder                                                 │
│ ├─ Processes files                                              │
│ └─ On completion:                                               │
│    emit_all("scan-complete", { sourceId: 1 })                  │
│                                                                 │
└────────────────────────┬───────────────────────────────────────┘
                         │ Event Bus
                         │
┌────────────────────────┴───────────────────────────────────────┐
│ React Frontend                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ App.tsx                                                         │
│ └─ useScanCompletionInvalidation()                             │
│    │                                                            │
│    └─ useEffect(() => {                                        │
│         listen('scan-complete', () => {                        │
│           invalidateAfterFileScanComplete(queryClient)         │
│         })                                                     │
│       }, [])                                                   │
│                                                                 │
└────────────────────────┬───────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ Automatic Cache Invalidation                                   │
│ - No manual refresh needed                                     │
│ - UI updates when scan completes                               │
│ - Works in background                                          │
└─────────────────────────────────────────────────────────────────┘

Benefits:
✓ Decoupled: Backend doesn't know about React Query
✓ Automatic: No manual refresh buttons
✓ Real-time: UI updates as soon as scan completes
✓ Scalable: Add more events without changing backend
```

---

## 9. Performance Optimization: Granular vs Broad Invalidation

```
Scenario: User deletes 1 track from an album with 100 tracks

┌─────────────────────────────────────────────────────────────────┐
│ ❌ POOR: Broad Invalidation                                    │
├─────────────────────────────────────────────────────────────────┤
│ invalidateQueries(['albums'])                                  │
│ invalidateQueries(['artists'])                                 │
│ invalidateQueries(['tracks'])                                  │
│                                                                 │
│ Result:                                                         │
│ - Refetches ALL albums (1000+ albums)                          │
│ - Refetches ALL artists (500+ artists)                         │
│ - Refetches ALL tracks (10000+ tracks)                         │
│ - Network: ~500KB - 1MB of data                                │
│ - Time: 500-1000ms                                             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ ✓ GOOD: Granular Invalidation                                  │
├─────────────────────────────────────────────────────────────────┤
│ invalidateQueries(['albums', 'detail', 42])                    │
│ invalidateQueries(['albums', 'detail', 42, 'tracks'])          │
│ invalidateQueries(['artists', 'detail', 10])                   │
│ invalidateQueries(['tracks', 'list']) // Only track lists      │
│                                                                 │
│ Result:                                                         │
│ - Refetches 1 album (album 42)                                 │
│ - Refetches 1 album's tracks (100 tracks)                      │
│ - Refetches 1 artist (artist 10)                               │
│ - Refetches track list (if visible)                            │
│ - Network: ~50-100KB                                           │
│ - Time: 50-100ms                                               │
└─────────────────────────────────────────────────────────────────┘

Performance Gain: 10x faster, 90% less data transferred
```

---

## 10. Invalidation Dependency Graph

```
                    ┌──────────────────┐
                    │  File Scan       │
                    │  Completes       │
                    └────────┬─────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
    ┌─────────┐        ┌─────────┐       ┌─────────┐
    │ Tracks  │        │ Albums  │       │ Artists │
    │  ALL    │        │  ALL    │       │  ALL    │
    └─────────┘        └─────────┘       └─────────┘

                    ┌──────────────────┐
                    │  Album Artwork   │
                    │  Changed         │
                    └────────┬─────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
    ┌─────────┐        ┌─────────┐       ┌─────────┐
    │ Album   │        │ Album   │       │ Artwork │
    │ Detail  │        │  List   │       │  Cache  │
    └─────────┘        └─────────┘       └─────────┘

                    ┌──────────────────┐
                    │  Track           │
                    │  Deleted         │
                    └────────┬─────────┘
                             │
     ┌───────────────────────┼───────────────────────┐
     │                       │                       │
     ▼                       ▼                       ▼
┌─────────┐            ┌─────────┐            ┌──────────┐
│ Tracks  │            │  Album  │            │  Artist  │
│  ALL    │            │ (Tracks │            │ (Tracks  │
└─────────┘            │  Count) │            │  Count)  │
                       └────┬────┘            └────┬─────┘
                            │                      │
                            ▼                      ▼
                       ┌─────────┐            ┌─────────┐
                       │Playlists│            │ Artist  │
                       │(contain │            │  List   │
                       │  track) │            └─────────┘
                       └─────────┘

                    ┌──────────────────┐
                    │  Album Metadata  │
                    │  Updated         │
                    └────────┬─────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
    ┌─────────┐        ┌─────────┐       ┌─────────┐
    │ Album   │        │ Album   │       │ Tracks  │
    │ Detail  │        │  List   │       │  ALL    │
    └─────────┘        └─────────┘       └────┬────┘
                                               │
                                          (cache
                                        album_title)
```

---

## Legend

```
┌─────────┐
│  Box    │   = Component, Function, or State
└─────────┘

    │
    ▼         = Flow direction

────┼────     = Branch/Decision point

✓            = Successful operation
❌           = Poor practice
```

---

**Related Documents**:
- `CACHE_INVALIDATION_STRATEGY.md` - Full design document
- `CACHE_INVALIDATION_QUICK_REFERENCE.md` - Quick reference guide
