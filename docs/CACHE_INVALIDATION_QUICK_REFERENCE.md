# Cache Invalidation Quick Reference

**Quick guide for implementing cache invalidation in Soul Player**

See `CACHE_INVALIDATION_STRATEGY.md` for full details.

---

## Common Invalidation Patterns

### Import the helpers

```typescript
import { useQueryClient } from '@tanstack/react-query'
import {
  invalidateAfterAlbumArtworkChange,
  invalidateAfterArtistArtworkChange,
  invalidateAfterPlaylistArtworkChange,
  invalidateAfterAlbumMetadataUpdate,
  invalidateAfterArtistMetadataUpdate,
  invalidateAfterFileScanComplete,
  invalidateAfterTrackDeletion,
} from '@soul-player/shared/hooks/queries/invalidationHelpers'
```

### Album Artwork Changed

```typescript
const queryClient = useQueryClient()
invalidateAfterAlbumArtworkChange(queryClient, albumId)
```

**Invalidates**:
- Album detail
- Album list (thumbnails)
- Artwork cache

### Artist Artwork Changed

```typescript
invalidateAfterArtistArtworkChange(queryClient, artistId)
```

**Invalidates**:
- Artist detail
- Artist list
- Artwork cache

### Playlist Artwork Changed

```typescript
invalidateAfterPlaylistArtworkChange(queryClient, playlistId)
```

**Invalidates**:
- Playlist detail
- Playlist list
- Artwork cache

### Album Metadata Updated

```typescript
invalidateAfterAlbumMetadataUpdate(queryClient, albumId, {
  artistChanged: true,
  newArtistId: 42
})
```

**Invalidates**:
- Album detail + tracks
- Album lists
- Track lists (cache album_title)
- Artist albums (if artist changed)

### Artist Metadata Updated

```typescript
invalidateAfterArtistMetadataUpdate(queryClient, artistId)
```

**Invalidates**:
- Artist detail
- Artist lists
- Artist albums
- Album lists (cache artist_name)
- Track lists (cache artist_name)

### Track Deleted

```typescript
const result = await backend.deleteTrack(trackId)
invalidateAfterTrackDeletion(queryClient, result)
```

**Invalidates**:
- All tracks
- Affected album
- Affected artist
- Affected playlists

### File Scan Completed

```typescript
invalidateAfterFileScanComplete(queryClient)
```

**Invalidates**:
- All tracks
- All albums
- All artists
- All genres
- Library health

---

## Using Mutations in Components

### Artwork Change

```typescript
import { useSetArtwork } from '@soul-player/shared/hooks/queries/useArtworkMutations'

function AlbumPage() {
  const setArtworkMutation = useSetArtwork()

  const handleSave = async (artworkData) => {
    await setArtworkMutation.mutateAsync({
      entityType: 'album',
      entityId: albumId,
      artworkBase64: artworkData.base64,
      mimeType: artworkData.mimeType,
      writeToFiles: true,
    })
  }
}
```

### Metadata Update

```typescript
import { useUpdateAlbum } from '@soul-player/shared/hooks/queries/useAlbumMutations'

function AlbumEditor() {
  const updateAlbumMutation = useUpdateAlbum()

  const handleSave = async (values) => {
    await updateAlbumMutation.mutateAsync({
      albumId,
      title: values.title,
      year: values.year,
    })
  }
}
```

### Track Deletion

```typescript
import { useDeleteTrack } from '@soul-player/shared/hooks/queries/useTrackMutations'

function TrackList() {
  const deleteTrackMutation = useDeleteTrack()

  const handleDelete = async (trackId) => {
    await deleteTrackMutation.mutateAsync(trackId)
    // Invalidation happens automatically
  }
}
```

---

## Event-Driven Invalidation

### Scan Completion (Desktop Only)

```typescript
// In App.tsx
import { useScanCompletionInvalidation } from '@soul-player/shared/hooks'

function App() {
  useScanCompletionInvalidation() // Auto-invalidates on scan-complete event
  return <AppRouter />
}
```

---

## Query Keys Reference

### Albums

```typescript
import { albumKeys } from '@soul-player/shared/hooks/queries/queryKeys'

albumKeys.all()                    // ['albums']
albumKeys.lists()                  // ['albums', 'list']
albumKeys.list(filters)            // ['albums', 'list', {...filters}]
albumKeys.detail(id)               // ['albums', 'detail', id]
albumKeys.tracks(id)               // ['albums', 'detail', id, 'tracks']
albumKeys.artwork(id)              // ['albums', 'detail', id, 'artwork']
albumKeys.random(limit)            // ['albums', 'list', 'random', limit]
albumKeys.recentlyAdded(limit)     // ['albums', 'list', 'recently-added', limit]
```

### Artists

```typescript
import { artistKeys } from '@soul-player/shared/hooks/queries/queryKeys'

artistKeys.all()                   // ['artists']
artistKeys.lists()                 // ['artists', 'list']
artistKeys.detail(id)              // ['artists', 'detail', id]
artistKeys.tracks(id)              // ['artists', 'detail', id, 'tracks']
artistKeys.albums(id)              // ['artists', 'detail', id, 'albums']
artistKeys.artwork(id)             // ['artists', 'detail', id, 'artwork']
```

### Playlists

```typescript
import { playlistKeys } from '@soul-player/shared/hooks/queries/queryKeys'

playlistKeys.all()                 // ['playlists']
playlistKeys.lists()               // ['playlists', 'list']
playlistKeys.detail(id)            // ['playlists', 'detail', id]
playlistKeys.tracks(id)            // ['playlists', 'detail', id, 'tracks']
playlistKeys.artwork(id)           // ['playlists', 'detail', id, 'artwork']
```

### Tracks

```typescript
import { trackKeys } from '@soul-player/shared/hooks/queries/queryKeys'

trackKeys.all()                    // ['tracks']
trackKeys.lists()                  // ['tracks', 'list']
trackKeys.list(filters)            // ['tracks', 'list', {...filters}]
```

---

## Invalidation Decision Tree

```
Did the mutation change...

├─ Artwork?
│  ├─ Album → invalidateAfterAlbumArtworkChange()
│  ├─ Artist → invalidateAfterArtistArtworkChange()
│  └─ Playlist → invalidateAfterPlaylistArtworkChange()
│
├─ Metadata?
│  ├─ Album title/year → invalidateAfterAlbumMetadataUpdate()
│  └─ Artist name → invalidateAfterArtistMetadataUpdate()
│
├─ Track count?
│  ├─ Track deleted → invalidateAfterTrackDeletion()
│  ├─ Playlist track added → useAddTrackToPlaylist() (handles it)
│  └─ Playlist track removed → useRemoveTrackFromPlaylist() (handles it)
│
└─ Bulk import/scan?
   └─ File scan completed → invalidateAfterFileScanComplete()
      OR useScanCompletionInvalidation() hook (automatic)
```

---

## Artwork Cache Busting

### Clear Specific Artwork

```typescript
import { clearArtworkCache } from '@soul-player/shared/components/ArtworkImage'

clearArtworkCache('album', albumId)
clearArtworkCache('artist', artistId)
clearArtworkCache('playlist', playlistId)
```

### Clear All Artwork

```typescript
import { clearAllArtworkCache } from '@soul-player/shared/components/ArtworkImage'

clearAllArtworkCache()
```

**Note**: Artwork cache is automatically cleared when using invalidation helpers. Manual clearing is rarely needed.

---

## Testing Invalidation

### Unit Test Example

```typescript
import { createMockQueryClient } from '../../../tests/utils/queryClient'
import { invalidateAfterAlbumArtworkChange } from './invalidationHelpers'
import { albumKeys, artworkKeys } from './queryKeys'

test('invalidateAfterAlbumArtworkChange invalidates correct keys', () => {
  const queryClient = createMockQueryClient()

  invalidateAfterAlbumArtworkChange(queryClient, 42)

  expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
    queryKey: albumKeys.detail(42)
  })
  expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
    queryKey: albumKeys.list()
  })
  expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
    queryKey: artworkKeys.album(42)
  })
})
```

### Integration Test Example

```typescript
import { renderHook, act, waitFor } from '@testing-library/react'
import { useSetArtwork } from './useArtworkMutations'
import { TestWrapper } from '../../../tests/utils/TestWrapper'

test('useSetArtwork invalidates caches on success', async () => {
  const queryClient = createMockQueryClient()
  const { result } = renderHook(() => useSetArtwork(), {
    wrapper: ({ children }) => (
      <TestWrapper queryClient={queryClient}>{children}</TestWrapper>
    )
  })

  await act(async () => {
    await result.current.mutateAsync({
      entityType: 'album',
      entityId: 42,
      artworkBase64: 'base64data',
      mimeType: 'image/jpeg'
    })
  })

  await waitFor(() => {
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: albumKeys.detail(42)
    })
  })
})
```

---

## Debugging Tips

### Enable Query Devtools

```typescript
// In App.tsx
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'

function App() {
  return (
    <>
      <AppRouter />
      {import.meta.env.DEV && (
        <ReactQueryDevtools initialIsOpen={false} />
      )}
    </>
  )
}
```

### Log Invalidations

```typescript
// In invalidationHelpers.ts
const DEBUG = import.meta.env.DEV

function invalidateAfterAlbumArtworkChange(
  queryClient: QueryClient,
  albumId: number
): void {
  if (DEBUG) {
    console.group('[Cache] Album Artwork Changed')
    console.log('Album ID:', albumId)
    console.log('Invalidating:', {
      detail: albumKeys.detail(albumId),
      list: albumKeys.list(),
      artwork: artworkKeys.album(albumId)
    })
    console.groupEnd()
  }
  // ... invalidation logic
}
```

### Check Active Queries

```typescript
import { useQueryClient } from '@tanstack/react-query'

function DebugPanel() {
  const queryClient = useQueryClient()
  const queryCache = queryClient.getQueryCache()

  const activeQueries = queryCache.getAll().filter(q => q.isActive())

  return (
    <div>
      <h3>Active Queries: {activeQueries.length}</h3>
      {activeQueries.map(q => (
        <div key={q.queryHash}>
          {JSON.stringify(q.queryKey)} - {q.state.status}
        </div>
      ))}
    </div>
  )
}
```

---

## Common Mistakes to Avoid

### ❌ Don't invalidate too broadly

```typescript
// BAD - invalidates everything
queryClient.invalidateQueries({ queryKey: ['albums'] })

// GOOD - invalidate specific album
queryClient.invalidateQueries({ queryKey: albumKeys.detail(42) })
```

### ❌ Don't forget to clear artwork cache

```typescript
// BAD - React Query cache invalidated but artwork cache not cleared
queryClient.invalidateQueries({ queryKey: albumKeys.detail(42) })

// GOOD - use helper that clears both
invalidateAfterAlbumArtworkChange(queryClient, 42)
```

### ❌ Don't manually refetch (let invalidation handle it)

```typescript
// BAD - manual refetch
await queryClient.invalidateQueries(...)
await queryClient.refetchQueries(...)

// GOOD - invalidate only (refetch happens automatically for active queries)
await queryClient.invalidateQueries(...)
```

### ❌ Don't use mutations without invalidation

```typescript
// BAD - data changed but cache not updated
await backend.setArtwork(...)
// UI shows stale data!

// GOOD - use mutation hook
const mutation = useSetArtwork()
await mutation.mutateAsync(...)
// Cache automatically invalidated
```

---

## Performance Tips

1. **Invalidate narrowly**: Use specific keys when possible
2. **Batch operations**: Group related invalidations in one `onSuccess`
3. **Use optimistic updates**: For high-frequency mutations (playlist edits)
4. **Monitor devtools**: Watch for unnecessary refetches
5. **Set appropriate staleTime**: Don't refetch data that rarely changes

---

## Related Files

- `CACHE_INVALIDATION_STRATEGY.md` - Full design document
- `applications/shared/src/hooks/queries/invalidationHelpers.ts` - Implementation
- `applications/shared/src/hooks/queries/queryKeys.ts` - Query key structure
- `applications/shared/src/hooks/queries/useArtworkMutations.ts` - Artwork mutations
- `applications/shared/src/hooks/queries/useAlbumMutations.ts` - Album mutations
- `applications/shared/src/hooks/queries/useArtistMutations.ts` - Artist mutations

---

**Last Updated**: 2026-02-11
