# Cache Invalidation Quick Reference

Quick copy-paste patterns for common cache invalidation scenarios in Soul Player.

## Import Statements

```typescript
import { useQueryClient } from '@tanstack/react-query'
import {
  invalidateAfterFileScan,
  invalidateAfterAlbumArtworkChange,
  invalidateAfterArtistArtworkChange,
  invalidateAfterPlaylistArtworkChange,
  invalidateAfterAlbumMetadataUpdate,
  invalidateAfterArtistMetadataUpdate,
} from '../hooks/queries/invalidationHelpers'
```

## Common Patterns

### 1. Artwork Change

```typescript
import { useSetArtwork } from '../hooks/queries/useArtworkMutations'

function MyComponent() {
  const setArtworkMutation = useSetArtwork()

  const handleArtworkChange = async (file: File) => {
    await setArtworkMutation.mutateAsync({
      entityType: 'album',
      entityId: '123',
      artworkBase64: await fileToBase64(file),
      mimeType: file.type
    })
    // Cache automatically invalidated!
  }
}
```

### 2. File Scan Completion

```typescript
// In App.tsx (call once at root)
import { useScanCompletionInvalidation } from '@soul-player/shared'

export function App() {
  useScanCompletionInvalidation()
  // ... rest of app
}
```

### 3. Manual Invalidation After Custom Operation

```typescript
function MyComponent() {
  const queryClient = useQueryClient()

  const handleCustomOperation = async () => {
    // ... do something ...

    // Option A: Use helper
    invalidateAfterAlbumArtworkChange(queryClient, albumId)

    // Option B: Direct invalidation
    queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  }
}
```

### 4. Track Deletion

```typescript
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'

function TrackMenu({ track }) {
  const deleteTrackMutation = useDeleteTrack()

  const handleDelete = async () => {
    await deleteTrackMutation.mutateAsync(track.id)
    // Automatically invalidates:
    // - trackKeys.all()
    // - albumKeys.all()
    // - artistKeys.all()
    // - playlistKeys.all()
  }
}
```

### 5. Playlist Mutations

```typescript
import { useAddTrackToPlaylist } from '../hooks/queries/usePlaylistMutations'

function AddToPlaylistButton({ track, playlistId }) {
  const addTrackMutation = useAddTrackToPlaylist()

  const handleAdd = async () => {
    await addTrackMutation.mutateAsync({ playlistId, trackId: track.id })
    // Automatically uses optimistic updates + invalidation
  }
}
```

## Query Keys Reference

```typescript
// Albums
albumKeys.all()                    // ['albums']
albumKeys.list()                   // ['albums', 'list']
albumKeys.detail(id)               // ['albums', 'detail', id]
albumKeys.tracks(id)               // ['albums', 'detail', id, 'tracks']

// Artists
artistKeys.all()                   // ['artists']
artistKeys.list()                  // ['artists', 'list']
artistKeys.detail(id)              // ['artists', 'detail', id]
artistKeys.tracks(id)              // ['artists', 'detail', id, 'tracks']

// Tracks
trackKeys.all()                    // ['tracks']
trackKeys.list()                   // ['tracks', 'list']

// Playlists
playlistKeys.all()                 // ['playlists']
playlistKeys.list()                // ['playlists', 'list']
playlistKeys.detail(id)            // ['playlists', 'detail', id]
playlistKeys.tracks(id)            // ['playlists', 'detail', id, 'tracks']

// Artwork (after integration)
artworkKeys.album(id)              // ['artwork', 'album', id]
artworkKeys.artist(id)             // ['artwork', 'artist', id]
```

## Invalidation Decision Tree

```
Did data change?
├─ YES: Which entity?
│  ├─ Artwork only → Use artwork helper
│  ├─ Metadata only → Use metadata helper
│  ├─ Deletion → Use specific entity invalidation
│  └─ Scan/Import → Use scan helper
└─ NO: Don't invalidate
```

## Testing Patterns

### Unit Test

```typescript
it('invalidates cache after mutation', () => {
  const queryClient = new QueryClient()
  queryClient.setQueryData(albumKeys.detail(1), { id: 1 })

  invalidateAfterAlbumArtworkChange(queryClient, 1)

  const state = queryClient.getQueryState(albumKeys.detail(1))
  expect(state?.isInvalidated).toBe(true)
})
```

### Integration Test

```typescript
it('mutation invalidates related queries', async () => {
  const { result } = renderHook(() => useSetArtwork(), { wrapper })

  result.current.mutate({ entityType: 'album', entityId: '1', ... })

  await waitFor(() => expect(result.current.isSuccess).toBe(true))

  const state = queryClient.getQueryState(albumKeys.detail(1))
  expect(state?.isInvalidated).toBe(true)
})
```

## Common Mistakes to Avoid

### ❌ Don't: Over-invalidate

```typescript
// BAD - refetches everything
queryClient.invalidateQueries()
```

### ✅ Do: Target specific queries

```typescript
// GOOD - only album 123
queryClient.invalidateQueries({ queryKey: albumKeys.detail(123) })
```

### ❌ Don't: Forget related entities

```typescript
// BAD - album title changed but tracks still cache old title
queryClient.invalidateQueries({ queryKey: albumKeys.detail(id) })
```

### ✅ Do: Use helpers that handle cascades

```typescript
// GOOD - invalidates album AND tracks
invalidateAfterAlbumMetadataUpdate(queryClient, id)
```

### ❌ Don't: Manual cache updates without invalidation

```typescript
// BAD - cache out of sync with server
queryClient.setQueryData(albumKeys.list(), [...oldAlbums, newAlbum])
```

### ✅ Do: Invalidate and refetch

```typescript
// GOOD - server is source of truth
queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
```

## Performance Tips

1. **Invalidate specifically**: Use `albumKeys.detail(id)` instead of `albumKeys.all()`
2. **Batch invalidations**: Call helper functions that handle related entities
3. **Use optimistic updates**: For playlist add/remove (already implemented)
4. **Avoid invalidating on every mutation**: Only when data actually changes

## Debugging

### Check Cache State in DevTools

```typescript
// React Query DevTools shows:
// - Query keys
// - Fetch status (idle, fetching, paused)
// - Data freshness (stale, fresh)
// - Cache hit/miss
```

### Log Invalidations

```typescript
queryClient.getQueryCache().subscribe((event) => {
  if (event.type === 'updated' && event.query.state.isInvalidated) {
    console.log('Invalidated:', event.query.queryKey)
  }
})
```

### Verify Backend Was Called

```typescript
const mutation = useSetArtwork()
mutation.mutate(params, {
  onSuccess: () => console.log('Backend succeeded'),
  onError: (err) => console.error('Backend failed:', err)
})
```

## Related Docs

- **Implementation Guide**: `docs/CACHE_STRATEGY_IMPLEMENTATION.md`
- **Research Findings**: See subagent research deliverables
- **TanStack Query Docs**: https://tanstack.com/query/latest

---

**Last Updated**: 2026-02-11
