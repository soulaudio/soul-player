# Cache Invalidation Strategy for Soul Player

**Version**: 1.0
**Date**: 2026-02-11
**Status**: Design Document

---

## Executive Summary

This document defines a comprehensive cache invalidation strategy for Soul Player's library management system. The strategy ensures data consistency across React Query caches, artwork caches, and UI state when library data changes through file imports, metadata edits, playlist mutations, and artwork changes.

---

## 1. Current State Analysis

### 1.1 Existing Query Keys

Well-structured hierarchical keys following TanStack Query v5 best practices:

```typescript
// Domain-based organization
albumKeys.all()                    // ['albums']
albumKeys.lists()                  // ['albums', 'list']
albumKeys.list(filters)            // ['albums', 'list', {...filters}]
albumKeys.details()                // ['albums', 'detail']
albumKeys.detail(id)               // ['albums', 'detail', id]
albumKeys.tracks(id)               // ['albums', 'detail', id, 'tracks']
albumKeys.random(limit)            // ['albums', 'list', 'random', limit]
albumKeys.recentlyAdded(limit)     // ['albums', 'list', 'recently-added', limit]

// Similar patterns for:
artistKeys.*
trackKeys.*
playlistKeys.*
genreKeys.*
libraryKeys.*
contextKeys.*
```

### 1.2 Existing Mutations

**Implemented**:
- `useAddTrackToPlaylist` - Optimistic updates, invalidates playlist tracks/detail
- `useRemoveTrackFromPlaylist` - Optimistic updates, invalidates playlist tracks/detail
- `useCreatePlaylist` - Invalidates `playlistKeys.all()`
- `useDeletePlaylist` - Optimistic list removal, invalidates `playlistKeys.all()`
- `useDeleteTrack` - Invalidates tracks, albums, artists, playlists (broad invalidation)

**Missing**:
- Album/artist metadata updates (title, year, etc.)
- Artwork change mutations (currently uses manual `clearArtworkCache()`)
- File import/scan completion invalidation
- Genre updates

### 1.3 Artwork Caching

**Current Implementation**:
- Separate `Map<string, string>` cache in `ArtworkImage.tsx`
- Manual invalidation via `clearArtworkCache(type, id)`
- Cache listener pattern for component reactivity
- Not integrated with React Query

**Issues**:
- Dual cache management (React Query + artwork cache)
- Manual coordination required between caches
- No automatic invalidation on mutations
- Cache keys not timestamp-based (stale images persist)

### 1.4 Tauri Events

**Existing Events** (not currently used for invalidation):
- `scan-started` - Library scan begins
- `scan-progress` - Scan progress update
- `scan-complete` - Scan finished (includes sourceId)

---

## 2. Design Principles

### 2.1 Core Principles

1. **Granular Invalidation**: Invalidate only what changed (specific album vs. all albums)
2. **Cascade Awareness**: Understand data relationships (deleting album → invalidate artist)
3. **Optimistic Updates**: Immediate UI feedback where safe
4. **Artwork Coordination**: Artwork and data invalidation happen together
5. **Event-Driven**: Use Tauri events to trigger invalidations automatically
6. **Type Safety**: Strongly typed invalidation functions
7. **Testability**: Invalidation logic is unit-testable

### 2.2 Invalidation Timing

| Mutation Type | Strategy | Rationale |
|---------------|----------|-----------|
| Playlist track add/remove | Optimistic + Invalidate | High frequency, needs instant feedback |
| Playlist create/delete | Optimistic + Invalidate | Instant feedback for list updates |
| Track delete | Invalidate only | Complex cascades, avoid stale optimistic data |
| Artwork change | Invalidate + Cache bust | Need fresh data + new image URL |
| File import/scan | Event-driven invalidation | Background operation, async completion |
| Metadata edit | Invalidate immediately | Rare operation, synchronous |

---

## 3. Query Key Structure and Conventions

### 3.1 Key Hierarchy Rules

**Established Pattern** (already implemented):
```typescript
// Level 1: Domain scope
[domain]                           // e.g., ['albums']

// Level 2: Operation type
[domain, 'list' | 'detail']        // e.g., ['albums', 'list']

// Level 3: Specificity
[domain, 'detail', id]             // e.g., ['albums', 'detail', 42]
[domain, 'list', filterType]       // e.g., ['albums', 'list', 'random']

// Level 4: Sub-resources
[domain, 'detail', id, subtype]    // e.g., ['albums', 'detail', 42, 'tracks']
```

**Benefits**:
- Partial invalidation: `invalidateQueries({ queryKey: albumKeys.all() })` invalidates all album queries
- Specific invalidation: `invalidateQueries({ queryKey: albumKeys.detail(42) })` only invalidates that album
- Sub-resource targeting: `invalidateQueries({ queryKey: albumKeys.tracks(42) })` invalidates only tracks

### 3.2 New Query Keys Needed

```typescript
// Add to existing queryKeys.ts

/**
 * Artwork query keys (NEW)
 * Artwork is now part of React Query cache
 */
export const artworkKeys = {
  all: () => ['artwork'] as const,
  album: (id: number) => [...artworkKeys.all(), 'album', id] as const,
  artist: (id: number) => [...artworkKeys.all(), 'artist', id] as const,
  playlist: (id: string) => [...artworkKeys.all(), 'playlist', id] as const,
}

// Integration with existing entity keys
// Add artwork sub-keys to existing domains:
albumKeys.artwork = (id: number) => [...albumKeys.detail(id), 'artwork'] as const
artistKeys.artwork = (id: number) => [...artistKeys.detail(id), 'artwork'] as const
playlistKeys.artwork = (id: string) => [...playlistKeys.detail(id), 'artwork'] as const
```

---

## 4. Mutation Patterns with Invalidation

### 4.1 Pattern Template

```typescript
export function useEntityMutation() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params) => {
      return backend.mutateEntity(params)
    },

    // Optional: Optimistic update for instant feedback
    onMutate: async (params) => {
      // 1. Cancel outgoing refetches
      await queryClient.cancelQueries({ queryKey: entityKeys.affected() })

      // 2. Snapshot current state
      const previous = queryClient.getQueryData(entityKeys.affected())

      // 3. Optimistically update cache
      queryClient.setQueryData(entityKeys.affected(), optimisticData)

      // 4. Return rollback context
      return { previous }
    },

    // Optional: Rollback on error
    onError: (_error, _variables, context) => {
      if (context?.previous) {
        queryClient.setQueryData(entityKeys.affected(), context.previous)
      }
    },

    // Always: Invalidate to refetch fresh data
    onSuccess: (data, variables) => {
      // Invalidate specific entity
      queryClient.invalidateQueries({ queryKey: entityKeys.detail(variables.id) })

      // Invalidate related entities (cascades)
      queryClient.invalidateQueries({ queryKey: relatedKeys.all() })

      // Clear artwork cache if applicable
      if (affectsArtwork) {
        clearArtworkCache('entity', variables.id)
        queryClient.invalidateQueries({ queryKey: artworkKeys.entity(variables.id) })
      }
    },
  })
}
```

### 4.2 Specific Mutations

#### 4.2.1 Artwork Change Mutation (NEW)

```typescript
// File: applications/shared/src/hooks/queries/useArtworkMutations.ts

export interface SetArtworkVariables {
  entityType: 'album' | 'artist' | 'playlist'
  entityId: string | number
  artworkBase64: string
  mimeType: string
  writeToFiles?: boolean
  useSoulStorage?: boolean
}

export function useSetArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (variables: SetArtworkVariables) => {
      await backend.setArtwork(variables)
    },

    onSuccess: (_data, variables) => {
      const { entityType, entityId } = variables
      const numericId = typeof entityId === 'string' ? parseInt(entityId, 10) : entityId

      // Invalidate entity detail (includes cover_art_path field)
      if (entityType === 'album') {
        queryClient.invalidateQueries({ queryKey: albumKeys.detail(numericId) })
        queryClient.invalidateQueries({ queryKey: albumKeys.list() }) // List view shows thumbnails
      } else if (entityType === 'artist') {
        queryClient.invalidateQueries({ queryKey: artistKeys.detail(numericId) })
        queryClient.invalidateQueries({ queryKey: artistKeys.list() })
      } else if (entityType === 'playlist') {
        queryClient.invalidateQueries({ queryKey: playlistKeys.detail(String(entityId)) })
        queryClient.invalidateQueries({ queryKey: playlistKeys.list() })
      }

      // Clear artwork cache (frontend cache)
      clearArtworkCache(entityType, entityId)

      // Invalidate artwork query (if we add artwork to React Query)
      queryClient.invalidateQueries({
        queryKey: entityType === 'album' ? artworkKeys.album(numericId) :
                   entityType === 'artist' ? artworkKeys.artist(numericId) :
                   artworkKeys.playlist(String(entityId))
      })
    },
  })
}

export function useRemoveArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      entityType,
      entityId
    }: {
      entityType: 'album' | 'artist' | 'playlist'
      entityId: string | number
    }) => {
      await backend.removeArtwork(entityType, String(entityId))
    },

    // Same onSuccess invalidation as useSetArtwork
    onSuccess: (_data, { entityType, entityId }) => {
      // ... (same logic as useSetArtwork)
    },
  })
}
```

#### 4.2.2 Album Metadata Update (NEW)

```typescript
// File: applications/shared/src/hooks/queries/useAlbumMutations.ts

export interface UpdateAlbumVariables {
  albumId: number
  title?: string
  artistId?: number
  year?: number
}

export function useUpdateAlbum() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (variables: UpdateAlbumVariables) => {
      return backend.updateAlbum(variables)
    },

    onSuccess: (_data, variables) => {
      const { albumId } = variables

      // Invalidate specific album
      queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
      queryClient.invalidateQueries({ queryKey: albumKeys.tracks(albumId) })

      // Invalidate album lists (titles/years changed)
      queryClient.invalidateQueries({ queryKey: albumKeys.lists() })

      // If artist changed, invalidate artist's albums
      if (variables.artistId) {
        queryClient.invalidateQueries({ queryKey: artistKeys.albums(variables.artistId) })
      }

      // Invalidate related tracks (they cache album_title)
      queryClient.invalidateQueries({ queryKey: trackKeys.all() })
    },
  })
}
```

#### 4.2.3 Artist Metadata Update (NEW)

```typescript
// File: applications/shared/src/hooks/queries/useArtistMutations.ts

export interface UpdateArtistVariables {
  artistId: number
  name?: string
  sortName?: string
}

export function useUpdateArtist() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (variables: UpdateArtistVariables) => {
      return backend.updateArtist(variables)
    },

    onSuccess: (_data, variables) => {
      const { artistId } = variables

      // Invalidate specific artist
      queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })

      // Invalidate artist lists (name changed)
      queryClient.invalidateQueries({ queryKey: artistKeys.lists() })

      // Invalidate artist's albums (they cache artist_name)
      queryClient.invalidateQueries({ queryKey: artistKeys.albums(artistId) })

      // Invalidate album/track lists (they cache artist_name)
      queryClient.invalidateQueries({ queryKey: albumKeys.all() })
      queryClient.invalidateQueries({ queryKey: trackKeys.all() })
    },
  })
}
```

#### 4.2.4 Improved Track Deletion

```typescript
// File: applications/shared/src/hooks/queries/useTrackMutations.ts
// UPDATED VERSION

export function useDeleteTrack() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (trackId: number) => {
      // Backend returns affected entity IDs for granular invalidation
      return backend.deleteTrack(trackId)
    },

    onSuccess: (result: { albumId?: number; artistId?: number; playlistIds?: string[] }) => {
      // Invalidate all track queries
      queryClient.invalidateQueries({ queryKey: trackKeys.all() })

      // Granular invalidation based on affected entities
      if (result.albumId) {
        queryClient.invalidateQueries({ queryKey: albumKeys.detail(result.albumId) })
        queryClient.invalidateQueries({ queryKey: albumKeys.tracks(result.albumId) })
        queryClient.invalidateQueries({ queryKey: albumKeys.lists() }) // Track count changed
      }

      if (result.artistId) {
        queryClient.invalidateQueries({ queryKey: artistKeys.detail(result.artistId) })
        queryClient.invalidateQueries({ queryKey: artistKeys.tracks(result.artistId) })
        queryClient.invalidateQueries({ queryKey: artistKeys.lists() }) // Track count changed
      }

      if (result.playlistIds && result.playlistIds.length > 0) {
        result.playlistIds.forEach((playlistId) => {
          queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
          queryClient.invalidateQueries({ queryKey: playlistKeys.tracks(playlistId) })
        })
        queryClient.invalidateQueries({ queryKey: playlistKeys.lists() })
      }
    },
  })
}
```

---

## 5. Cache Tags and Dependencies

### 5.1 Dependency Graph

```
File Import/Scan Complete
├── Invalidates: trackKeys.all()
├── Invalidates: albumKeys.all()
├── Invalidates: artistKeys.all()
├── Invalidates: genreKeys.all()
└── Invalidates: libraryKeys.health()

Album Artwork Change
├── Invalidates: albumKeys.detail(id)
├── Invalidates: albumKeys.list() (thumbnails in grid)
├── Clears: artworkCache['album:id']
└── Invalidates: artworkKeys.album(id)

Album Metadata Update
├── Invalidates: albumKeys.detail(id)
├── Invalidates: albumKeys.tracks(id)
├── Invalidates: albumKeys.lists()
├── Invalidates: artistKeys.albums(artistId) [if artist changed]
└── Invalidates: trackKeys.all() [tracks cache album_title]

Artist Artwork Change
├── Invalidates: artistKeys.detail(id)
├── Invalidates: artistKeys.list()
├── Clears: artworkCache['artist:id']
└── Invalidates: artworkKeys.artist(id)

Artist Metadata Update
├── Invalidates: artistKeys.detail(id)
├── Invalidates: artistKeys.lists()
├── Invalidates: artistKeys.albums(id)
├── Invalidates: albumKeys.all() [albums cache artist_name]
└── Invalidates: trackKeys.all() [tracks cache artist_name]

Track Deletion
├── Invalidates: trackKeys.all()
├── Invalidates: albumKeys.detail(albumId)
├── Invalidates: albumKeys.tracks(albumId)
├── Invalidates: albumKeys.lists() [track counts changed]
├── Invalidates: artistKeys.detail(artistId)
├── Invalidates: artistKeys.tracks(artistId)
├── Invalidates: artistKeys.lists() [track counts changed]
└── Invalidates: playlistKeys.* [for all containing playlists]

Playlist Mutations (existing - already implemented)
├── Add/Remove Track: Optimistic + invalidate playlist detail/tracks
├── Create Playlist: Invalidate playlistKeys.all()
└── Delete Playlist: Optimistic removal + invalidate playlistKeys.all()
```

### 5.2 Tag-Based Invalidation Helper (NEW)

```typescript
// File: applications/shared/src/hooks/queries/invalidationHelpers.ts

import { QueryClient } from '@tanstack/react-query'
import {
  albumKeys,
  artistKeys,
  trackKeys,
  playlistKeys,
  genreKeys,
  libraryKeys,
  artworkKeys
} from './queryKeys'
import { clearArtworkCache } from '../../components/ArtworkImage'

/**
 * Centralized invalidation functions for common operations
 * Ensures consistent cascade behavior across the app
 */

export function invalidateAfterAlbumArtworkChange(
  queryClient: QueryClient,
  albumId: number
): void {
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.list() })
  queryClient.invalidateQueries({ queryKey: artworkKeys.album(albumId) })
  clearArtworkCache('album', albumId)
}

export function invalidateAfterArtistArtworkChange(
  queryClient: QueryClient,
  artistId: number
): void {
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.list() })
  queryClient.invalidateQueries({ queryKey: artworkKeys.artist(artistId) })
  clearArtworkCache('artist', artistId)
}

export function invalidateAfterPlaylistArtworkChange(
  queryClient: QueryClient,
  playlistId: string
): void {
  queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
  queryClient.invalidateQueries({ queryKey: playlistKeys.list() })
  queryClient.invalidateQueries({ queryKey: artworkKeys.playlist(playlistId) })
  clearArtworkCache('playlist', playlistId)
}

export function invalidateAfterAlbumMetadataUpdate(
  queryClient: QueryClient,
  albumId: number,
  options?: { artistChanged?: boolean; newArtistId?: number }
): void {
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.tracks(albumId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })

  if (options?.artistChanged && options?.newArtistId) {
    queryClient.invalidateQueries({ queryKey: artistKeys.albums(options.newArtistId) })
  }
}

export function invalidateAfterArtistMetadataUpdate(
  queryClient: QueryClient,
  artistId: number
): void {
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: artistKeys.albums(artistId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.all() })
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
}

export function invalidateAfterFileScanComplete(
  queryClient: QueryClient
): void {
  // Broad invalidation - file scan can add/update anything
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
  queryClient.invalidateQueries({ queryKey: albumKeys.all() })
  queryClient.invalidateQueries({ queryKey: artistKeys.all() })
  queryClient.invalidateQueries({ queryKey: genreKeys.all() })
  queryClient.invalidateQueries({ queryKey: libraryKeys.health() })

  // Note: Don't clear artwork cache - scans preserve existing artwork
}

export function invalidateAfterTrackDeletion(
  queryClient: QueryClient,
  result: { albumId?: number; artistId?: number; playlistIds?: string[] }
): void {
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })

  if (result.albumId) {
    queryClient.invalidateQueries({ queryKey: albumKeys.detail(result.albumId) })
    queryClient.invalidateQueries({ queryKey: albumKeys.tracks(result.albumId) })
    queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
  }

  if (result.artistId) {
    queryClient.invalidateQueries({ queryKey: artistKeys.detail(result.artistId) })
    queryClient.invalidateQueries({ queryKey: artistKeys.tracks(result.artistId) })
    queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  }

  if (result.playlistIds) {
    result.playlistIds.forEach((playlistId) => {
      queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
      queryClient.invalidateQueries({ queryKey: playlistKeys.tracks(playlistId) })
    })
    queryClient.invalidateQueries({ queryKey: playlistKeys.lists() })
  }
}
```

---

## 6. Artwork Cache Busting Strategy

### 6.1 Current Problem

**Issue**: Artwork URLs are cached with static keys (`album:42`). When artwork changes:
- Backend updates file/database
- Frontend invalidates cache
- Frontend refetches artwork with same key
- Browser caches image by URL → shows stale image

**Root Cause**: No cache busting mechanism in artwork URLs.

### 6.2 Solution: Timestamp-Based Cache Keys

```typescript
// File: applications/shared/src/components/ArtworkImage.tsx
// UPDATED VERSION

// Change cache key structure to include timestamp
const artworkCache = new Map<string, { dataUrl: string; timestamp: number }>()

export function clearArtworkCache(
  type: 'track' | 'album' | 'artist' | 'playlist',
  id: string | number
): void {
  const key = `${type}:${id}`
  artworkCache.delete(key)
  notifyListeners(key)
}

// In component: add timestamp to fetch
const getCacheKey = useCallback((): string => {
  if (trackId) return `track:${trackId}`
  if (albumId) return `album:${albumId}`
  if (artistId) return `artist:${artistId}`
  if (playlistId) return `playlist:${playlistId}`
  return ''
}, [trackId, albumId, artistId, playlistId])

// When fetching artwork, append timestamp to URL
const fetchArtwork = useCallback(async () => {
  const cacheKey = getCacheKey()

  // Check cache
  const cached = artworkCache.get(cacheKey)
  if (cached && Date.now() - cached.timestamp < 60000) {
    return cached.dataUrl
  }

  // Fetch fresh artwork
  const dataUrl = await backend.getArtwork(...)

  // Cache with timestamp
  artworkCache.set(cacheKey, {
    dataUrl,
    timestamp: Date.now()
  })

  return dataUrl
}, [...])
```

### 6.3 Alternative: Query String Cache Busting

For Tauri `convertFileSrc()` URLs, append timestamp:

```typescript
const artworkUrl = `${convertFileSrc(artworkPath)}?t=${Date.now()}`
```

**Pros**: Simple, browser respects cache busting
**Cons**: Bypasses browser cache (re-downloads unchanged images)

**Recommendation**: Use timestamp-based React cache (6.2) for in-app caching, with query string for external URLs.

---

## 7. Event-Driven Invalidation

### 7.1 Scan Completion Hook (NEW)

```typescript
// File: applications/shared/src/hooks/useScanCompletionInvalidation.ts

import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { invalidateAfterFileScanComplete } from './queries/invalidationHelpers'

/**
 * Desktop-only hook - listens for scan-complete events and invalidates caches
 * No-op on web platforms
 */
export function useScanCompletionInvalidation() {
  const queryClient = useQueryClient()

  useEffect(() => {
    // Only run in Tauri environment
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
      return
    }

    let unlisten: (() => void) | null = null

    async function setupListener() {
      const { listen } = await import('@tauri-apps/api/event')

      unlisten = await listen('scan-complete', () => {
        console.log('[Cache] Scan complete - invalidating library caches')
        invalidateAfterFileScanComplete(queryClient)
      })
    }

    void setupListener()

    return () => {
      unlisten?.()
    }
  }, [queryClient])
}
```

### 7.2 Integration Points

**Desktop App** (`applications/desktop/src/App.tsx`):
```typescript
import { useScanCompletionInvalidation } from '@soul-player/shared/hooks'

function App() {
  // Auto-invalidate on scan completion
  useScanCompletionInvalidation()

  return <AppRouter />
}
```

**ScanProgressIndicator** (already has `onComplete` callback):
```typescript
// Remove manual refreshLibrary() call - hook handles it automatically
<ScanProgressIndicator
  position="footer"
  // onComplete prop no longer needed - hook handles invalidation
/>
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

**Test Coverage**:

1. **Query Key Structure** (`queryKeys.test.ts`):
   - Verify key hierarchy (parent keys are prefixes of child keys)
   - Test partial invalidation (invalidating `albumKeys.all()` affects `albumKeys.detail(42)`)

2. **Invalidation Helpers** (`invalidationHelpers.test.ts`):
   - Mock `QueryClient.invalidateQueries`
   - Verify correct keys invalidated for each helper
   - Test cascade logic (e.g., album metadata → tracks invalidated)

3. **Mutation Hooks** (per-hook test files):
   - Test `onSuccess` invalidation calls
   - Test optimistic update rollback on error
   - Verify artwork cache cleared when appropriate

4. **Artwork Cache** (`ArtworkImage.test.tsx`):
   - Test cache listener notifications
   - Test timestamp-based expiration
   - Test `clearArtworkCache()` behavior

### 8.2 Integration Tests

**Scenarios**:

1. **Artwork Change Flow**:
   ```typescript
   test('artwork change invalidates all related caches', async () => {
     const { result } = renderHook(() => useSetArtwork(), { wrapper: TestWrapper })

     // Execute mutation
     await act(async () => {
       await result.current.mutateAsync({
         entityType: 'album',
         entityId: 42,
         artworkBase64: '...',
         mimeType: 'image/jpeg'
       })
     })

     // Verify invalidations
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
       queryKey: albumKeys.detail(42)
     })
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
       queryKey: albumKeys.list()
     })

     // Verify artwork cache cleared
     const cache = getArtworkCacheState()
     expect(cache.has('album:42')).toBe(false)
   })
   ```

2. **Track Deletion Cascade**:
   ```typescript
   test('track deletion invalidates album, artist, and playlist caches', async () => {
     // Mock backend response with affected entities
     mockBackend.deleteTrack.mockResolvedValue({
       albumId: 10,
       artistId: 5,
       playlistIds: ['playlist-1', 'playlist-2']
     })

     const { result } = renderHook(() => useDeleteTrack(), { wrapper: TestWrapper })

     await act(async () => {
       await result.current.mutateAsync(123)
     })

     // Verify all affected entities invalidated
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: trackKeys.all() })
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: albumKeys.detail(10) })
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: artistKeys.detail(5) })
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: playlistKeys.detail('playlist-1') })
     expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: playlistKeys.detail('playlist-2') })
   })
   ```

3. **Scan Completion Event**:
   ```typescript
   test('scan-complete event invalidates library caches', async () => {
     const { rerender } = render(<App />, { wrapper: TauriTestWrapper })

     // Trigger scan-complete event
     await emitTauriEvent('scan-complete', { sourceId: 1 })

     await waitFor(() => {
       expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: trackKeys.all() })
       expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: albumKeys.all() })
       expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: artistKeys.all() })
     })
   })
   ```

### 8.3 E2E Tests

**Critical User Flows**:

1. **File Import → UI Update**:
   - Import new album folder
   - Wait for scan completion
   - Verify album appears in Library → Albums tab without manual refresh

2. **Artwork Change → Immediate Visibility**:
   - Open album detail page
   - Edit artwork
   - Verify new artwork shows immediately (no browser refresh needed)
   - Navigate to Library page
   - Verify thumbnail updated in album grid

3. **Track Deletion → Cascade Update**:
   - View album with 10 tracks
   - Delete 1 track
   - Verify album track count updates to 9
   - Verify artist track count decrements
   - Verify playlists containing track update

---

## 9. Implementation Plan

### 9.1 Phase 1: Foundation (Week 1)

**Files to Create**:
- `applications/shared/src/hooks/queries/invalidationHelpers.ts`
- `applications/shared/src/hooks/queries/useArtworkMutations.ts`
- `applications/shared/src/hooks/queries/useAlbumMutations.ts`
- `applications/shared/src/hooks/queries/useArtistMutations.ts`
- `applications/shared/src/hooks/useScanCompletionInvalidation.ts`

**Files to Modify**:
- `applications/shared/src/hooks/queries/queryKeys.ts` - Add artwork keys
- `applications/shared/src/hooks/queries/useTrackMutations.ts` - Update `useDeleteTrack`
- `applications/shared/src/components/ArtworkImage.tsx` - Add timestamp caching

**Backend Changes Needed** (Rust):
- Add Tauri commands: `update_album`, `update_artist`
- Modify `delete_track` to return affected entity IDs
- Ensure `scan-complete` event emitted on scan finish

**Tests**:
- Unit tests for `invalidationHelpers.ts`
- Unit tests for new mutation hooks

### 9.2 Phase 2: Integration (Week 2)

**Integration**:
- Update `EditArtworkDialog.tsx` to use `useSetArtwork()` mutation
- Add `useScanCompletionInvalidation()` to `App.tsx`
- Update `LibraryPage.tsx` to remove manual `refreshLibrary()` (now automatic)

**Files to Modify**:
- `applications/shared/src/components/EditArtworkDialog.tsx`
- `applications/desktop/src/App.tsx`
- `applications/shared/src/pages/LibraryPage.tsx`
- `applications/desktop/src/components/ScanProgressIndicator.tsx` (remove `onComplete` callback)

**Tests**:
- Integration tests for artwork change flow
- Integration tests for scan completion
- E2E test for file import → UI update

### 9.3 Phase 3: Validation (Week 3)

**Manual Testing Checklist**:
- [ ] Import new album → appears in library without refresh
- [ ] Change album artwork → updates everywhere (detail page, grid, now playing)
- [ ] Change artist artwork → updates in artist grid and detail page
- [ ] Change playlist artwork → updates in playlist list and detail
- [ ] Delete track → album/artist/playlist counts update
- [ ] Edit album title → updates in lists and track views
- [ ] Edit artist name → updates in albums/tracks that reference it
- [ ] Scan large library → no UI freezes, smooth invalidation

**Performance Testing**:
- Monitor invalidation performance with 10,000+ tracks
- Verify no unnecessary refetches (check React Query Devtools)
- Ensure artwork cache hit rate > 90% during normal usage

**Documentation**:
- Update `CLAUDE.md` with cache invalidation patterns
- Add comments to query hooks explaining invalidation strategy
- Document new mutation hooks in README

---

## 10. Migration Steps

### 10.1 Code Migration Checklist

**Step 1: Add Helpers** (Non-breaking)
```bash
# Create new files - no existing code changes
touch applications/shared/src/hooks/queries/invalidationHelpers.ts
touch applications/shared/src/hooks/queries/useArtworkMutations.ts
```

**Step 2: Update Existing Mutations** (Breaking - replace old invalidation)
```typescript
// Before (in useTrackMutations.ts):
onSuccess: () => {
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
  queryClient.invalidateQueries({ queryKey: albumKeys.all() })
  queryClient.invalidateQueries({ queryKey: artistKeys.all() })
  queryClient.invalidateQueries({ queryKey: playlistKeys.all() })
}

// After (using helper):
onSuccess: (result) => {
  invalidateAfterTrackDeletion(queryClient, result)
}
```

**Step 3: Replace Manual Cache Clearing** (Breaking - remove manual calls)
```typescript
// Before (in EditArtworkDialog.tsx):
await backend.setArtwork(...)
clearArtworkCache(entityType, entityId)
onArtworkChanged?.() // Manual callback

// After (using mutation):
const setArtworkMutation = useSetArtwork()
await setArtworkMutation.mutateAsync(...)
// No manual clearing - mutation handles it
```

**Step 4: Add Event Listeners** (Additive - safe)
```typescript
// In App.tsx - just add the hook:
function App() {
  useScanCompletionInvalidation() // NEW
  return <AppRouter />
}
```

### 10.2 Backend Migration

**Required Rust Changes**:

1. **Add Return Types to Mutations**:
```rust
// Before:
#[tauri::command]
async fn delete_track(pool: &SqlitePool, track_id: i64) -> Result<(), String> {
    storage::tracks::delete_track(pool, 1, track_id).await.map_err(|e| e.to_string())
}

// After:
#[derive(Serialize)]
struct DeleteTrackResult {
    album_id: Option<i64>,
    artist_id: Option<i64>,
    playlist_ids: Vec<String>,
}

#[tauri::command]
async fn delete_track(pool: &SqlitePool, track_id: i64) -> Result<DeleteTrackResult, String> {
    let result = storage::tracks::delete_track_with_affected(pool, 1, track_id)
        .await.map_err(|e| e.to_string())?;
    Ok(result)
}
```

2. **Add New Commands**:
```rust
#[tauri::command]
async fn update_album(
    pool: &SqlitePool,
    album_id: i64,
    title: Option<String>,
    artist_id: Option<i64>,
    year: Option<i32>,
) -> Result<(), String> {
    storage::albums::update_album(pool, 1, album_id, title, artist_id, year)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_artist(
    pool: &SqlitePool,
    artist_id: i64,
    name: Option<String>,
    sort_name: Option<String>,
) -> Result<(), String> {
    storage::artists::update_artist(pool, 1, artist_id, name, sort_name)
        .await.map_err(|e| e.to_string())
}
```

3. **Ensure Scan Events**:
```rust
// In scan completion handler:
fn on_scan_complete(app_handle: &AppHandle, source_id: i64) {
    app_handle.emit_all("scan-complete", json!({ "sourceId": source_id }))
        .expect("Failed to emit scan-complete event");
}
```

### 10.3 Rollback Plan

**If Issues Arise**:

1. **Keep Old Code Temporarily**:
```typescript
// Keep both approaches during transition
const setArtworkMutation = useSetArtwork()

const handleSaveOld = async () => {
  await backend.setArtwork(...)
  clearArtworkCache(...)
  onArtworkChanged?.()
}

const handleSaveNew = async () => {
  await setArtworkMutation.mutateAsync(...)
}

// Use feature flag to toggle
const useLegacyInvalidation = false
const handleSave = useLegacyInvalidation ? handleSaveOld : handleSaveNew
```

2. **Gradual Rollout**:
   - Week 1: Add helpers, test in isolation
   - Week 2: Migrate artwork mutations only
   - Week 3: Migrate metadata mutations
   - Week 4: Add event listeners
   - Week 5: Remove legacy code

3. **Monitoring**:
   - Add logging to invalidation helpers
   - Monitor React Query Devtools for unexpected refetches
   - Track user reports of stale data

---

## 11. Code Patterns and Examples

### 11.1 Using Mutations in Components

**Album Detail Page** (artwork editing):
```typescript
function AlbumDetailPage() {
  const { albumId } = useParams()
  const { data: album } = useAlbum(Number(albumId))
  const setArtworkMutation = useSetArtwork()
  const [editArtworkOpen, setEditArtworkOpen] = useState(false)

  const handleArtworkSave = async (artworkData: ArtworkData) => {
    try {
      await setArtworkMutation.mutateAsync({
        entityType: 'album',
        entityId: Number(albumId),
        artworkBase64: artworkData.base64,
        mimeType: artworkData.mimeType,
        writeToFiles: true,
      })
      setEditArtworkOpen(false)
      toast.success('Artwork updated')
    } catch (error) {
      toast.error('Failed to update artwork')
    }
  }

  return (
    <div>
      <ArtworkImage albumId={Number(albumId)} />
      <Button onClick={() => setEditArtworkOpen(true)}>Edit Artwork</Button>

      <EditArtworkDialog
        open={editArtworkOpen}
        onClose={() => setEditArtworkOpen(false)}
        entityType="album"
        entityId={String(albumId)}
        entityName={album?.title || ''}
        currentArtworkUrl={album?.cover_art_path}
        onSave={handleArtworkSave}
      />
    </div>
  )
}
```

**Album Metadata Editor** (hypothetical):
```typescript
function AlbumMetadataEditor({ albumId }: { albumId: number }) {
  const { data: album } = useAlbum(albumId)
  const updateAlbumMutation = useUpdateAlbum()

  const formik = useFormik({
    initialValues: {
      title: album?.title || '',
      year: album?.year || null,
    },
    onSubmit: async (values) => {
      await updateAlbumMutation.mutateAsync({
        albumId,
        title: values.title,
        year: values.year || undefined,
      })
    },
  })

  return (
    <form onSubmit={formik.handleSubmit}>
      <input name="title" value={formik.values.title} onChange={formik.handleChange} />
      <input name="year" type="number" value={formik.values.year || ''} onChange={formik.handleChange} />
      <button type="submit" disabled={updateAlbumMutation.isPending}>
        {updateAlbumMutation.isPending ? 'Saving...' : 'Save'}
      </button>
    </form>
  )
}
```

### 11.2 Using Invalidation Helpers Directly

**Custom Hook** (for complex workflows):
```typescript
function useBulkTrackImport() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (files: File[]) => {
      return backend.importTracks(files)
    },
    onSuccess: () => {
      // Bulk import = full library invalidation
      invalidateAfterFileScanComplete(queryClient)
    },
  })
}
```

**Manual Invalidation** (rare cases):
```typescript
function SettingsPage() {
  const queryClient = useQueryClient()

  const handleClearCache = () => {
    // Nuclear option - clear everything
    queryClient.clear()
    clearAllArtworkCache()
    toast.success('Cache cleared')
  }

  return <Button onClick={handleClearCache}>Clear All Caches</Button>
}
```

---

## 12. Performance Considerations

### 12.1 Invalidation Performance

**Query Invalidation Cost**:
- `invalidateQueries({ queryKey: albumKeys.all() })` → O(n) where n = number of active queries
- React Query only refetches queries that are currently mounted/observed
- Background queries are marked stale but not refetched until needed

**Best Practices**:
1. **Invalidate Narrowly**: Prefer `albumKeys.detail(42)` over `albumKeys.all()`
2. **Batch Invalidations**: Group related invalidations in one `onSuccess`
3. **Avoid Over-Invalidation**: Don't invalidate `trackKeys.all()` for artwork changes
4. **Use Optimistic Updates**: For high-frequency mutations (playlist track add/remove)

### 12.2 Artwork Cache Performance

**Current Cache Size**:
- Average artwork: 100-200 KB (JPEG, 500x500)
- 100 cached artworks: ~15-20 MB memory
- LRU eviction not implemented (could grow unbounded)

**Improvements Needed**:
```typescript
// Add LRU eviction to artwork cache
const MAX_ARTWORK_CACHE_SIZE = 200 // entries
const artworkCache = new LRUMap<string, ArtworkCacheEntry>(MAX_ARTWORK_CACHE_SIZE)
```

### 12.3 Monitoring and Debugging

**React Query Devtools**:
```typescript
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

**Custom Logging**:
```typescript
// In invalidationHelpers.ts
const DEBUG_INVALIDATION = import.meta.env.DEV

function invalidateAfterAlbumArtworkChange(
  queryClient: QueryClient,
  albumId: number
): void {
  if (DEBUG_INVALIDATION) {
    console.group('[Cache Invalidation] Album Artwork Changed')
    console.log('Album ID:', albumId)
    console.log('Invalidating:', albumKeys.detail(albumId))
    console.groupEnd()
  }

  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  // ...
}
```

---

## 13. Future Enhancements

### 13.1 Smart Invalidation

**Idea**: Track query dependencies automatically
```typescript
// Automatically track what queries depend on an album
const albumDeps = new Map<number, Set<string>>()

// When fetching album tracks, register dependency
albumDeps.get(42)?.add('tracks:list')

// On album delete, invalidate all dependent queries
const deps = albumDeps.get(42) || new Set()
deps.forEach(key => queryClient.invalidateQueries({ queryKey: [key] }))
```

### 13.2 Background Sync

**Idea**: Periodic background sync for stale data
```typescript
// Refresh stale data in background (not visible to user)
queryClient.prefetchQuery({
  queryKey: albumKeys.list(),
  staleTime: 1000 * 60 * 5, // 5 minutes
})
```

### 13.3 Offline-First Mutations

**Idea**: Queue mutations when offline, replay on reconnect
```typescript
// Using @tanstack/query-persist-client
import { PersistQueryClient } from '@tanstack/react-query-persist-client'

const persister = new PersistQueryClient({
  storage: window.localStorage,
})
```

---

## Appendix A: Full File Structure

```
applications/shared/src/hooks/
├── queries/
│   ├── queryKeys.ts                    # (UPDATED) Add artwork keys
│   ├── invalidationHelpers.ts          # (NEW) Centralized invalidation functions
│   ├── useLibraryQueries.ts            # (EXISTING) No changes
│   ├── useAlbumQueries.ts              # (EXISTING) No changes
│   ├── useArtistQueries.ts             # (EXISTING) No changes
│   ├── useTrackMutations.ts            # (UPDATED) Improve useDeleteTrack
│   ├── usePlaylistMutations.ts         # (EXISTING) No changes
│   ├── useArtworkMutations.ts          # (NEW) useSetArtwork, useRemoveArtwork
│   ├── useAlbumMutations.ts            # (NEW) useUpdateAlbum
│   └── useArtistMutations.ts           # (NEW) useUpdateArtist
├── useScanCompletionInvalidation.ts    # (NEW) Event listener for scan-complete
└── ...

applications/shared/src/components/
├── ArtworkImage.tsx                    # (UPDATED) Add timestamp caching
├── EditArtworkDialog.tsx               # (UPDATED) Use useSetArtwork mutation
└── ...

applications/desktop/src/
├── App.tsx                             # (UPDATED) Add useScanCompletionInvalidation
├── components/
│   └── ScanProgressIndicator.tsx       # (UPDATED) Remove onComplete callback
└── ...

applications/desktop/src-tauri/src/
├── commands/
│   ├── library.rs                      # (UPDATED) Add update_album, update_artist
│   └── tracks.rs                       # (UPDATED) Return affected IDs from delete_track
└── ...
```

---

## Appendix B: Backend API Changes

**New Tauri Commands Needed**:

```rust
// In applications/desktop/src-tauri/src/commands/library.rs

#[tauri::command]
pub async fn update_album(
    pool: State<'_, SqlitePool>,
    album_id: i64,
    title: Option<String>,
    artist_id: Option<i64>,
    year: Option<i32>,
) -> Result<(), String> {
    // Implementation in soul-storage crate
}

#[tauri::command]
pub async fn update_artist(
    pool: State<'_, SqlitePool>,
    artist_id: i64,
    name: Option<String>,
    sort_name: Option<String>,
) -> Result<(), String> {
    // Implementation in soul-storage crate
}

#[derive(Serialize)]
pub struct DeleteTrackResult {
    pub album_id: Option<i64>,
    pub artist_id: Option<i64>,
    pub playlist_ids: Vec<String>,
}

#[tauri::command]
pub async fn delete_track(
    pool: State<'_, SqlitePool>,
    track_id: i64,
) -> Result<DeleteTrackResult, String> {
    // Modified to return affected entities
}
```

**Backend Context Interface Extension**:

```typescript
// In applications/shared/src/contexts/BackendContext.tsx

export interface BackendInterface {
  // ... existing methods ...

  // NEW: Metadata updates
  updateAlbum: (params: {
    albumId: number
    title?: string
    artistId?: number
    year?: number
  }) => Promise<void>

  updateArtist: (params: {
    artistId: number
    name?: string
    sortName?: string
  }) => Promise<void>

  // UPDATED: Return affected IDs
  deleteTrack: (id: number) => Promise<{
    albumId?: number
    artistId?: number
    playlistIds?: string[]
  }>
}
```

---

## Appendix C: Testing Utilities

**Mock Query Client Setup**:

```typescript
// tests/utils/queryClient.ts

import { QueryClient } from '@tanstack/react-query'

export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: Infinity,
      },
      mutations: {
        retry: false,
      },
    },
    logger: {
      log: console.log,
      warn: console.warn,
      error: () => {}, // Silence errors in tests
    },
  })
}

export function createMockQueryClient(): QueryClient & {
  invalidateQueries: jest.Mock
} {
  const client = createTestQueryClient()
  const invalidateMock = jest.fn()
  client.invalidateQueries = invalidateMock
  return client as any
}
```

**Test Wrapper**:

```typescript
// tests/utils/TestWrapper.tsx

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BackendProvider } from '@soul-player/shared/contexts'
import { mockBackend } from './mockBackend'

export function TestWrapper({
  children,
  queryClient = createTestQueryClient()
}: {
  children: React.ReactNode
  queryClient?: QueryClient
}) {
  return (
    <QueryClientProvider client={queryClient}>
      <BackendProvider value={mockBackend}>
        {children}
      </BackendProvider>
    </QueryClientProvider>
  )
}
```

---

## Document Control

**Version History**:
- v1.0 (2026-02-11): Initial design document

**Approval**:
- Design Review: Pending
- Implementation Start: Pending

**Related Documents**:
- `ARCHITECTURE.md` - Overall system architecture
- `TESTING.md` - Testing strategy and guidelines
- `CLAUDE.md` - Development guidelines

**Contact**:
- For questions: See project maintainers in `CONTRIBUTING.md`
