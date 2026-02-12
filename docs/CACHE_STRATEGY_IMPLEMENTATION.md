# Cache Invalidation Strategy - Implementation Guide

## Executive Summary

This document provides a complete implementation strategy for cache invalidation in Soul Player, addressing:
- File imports not appearing without refresh
- Artwork changes not updating across the app
- Stale data after library operations
- Inconsistent cache behavior

**Based on Research**: TanStack Query best practices, current codebase analysis, and proven patterns from the React Query community.

---

## Current State Analysis

### What Works Well ✅
- **Query key structure**: Hierarchical, follows best practices
- **Playlist mutations**: Optimistic updates implemented correctly
- **Track deletion**: Proper cascade invalidation
- **Query organization**: `*Options` pattern enables code reuse

### Critical Gaps ❌
1. **No artwork mutation hooks** → Changes don't invalidate React Query cache
2. **No album/artist metadata mutations** → Can't update via UI
3. **No scan completion invalidation** → Fresh imports require manual refresh
4. **Dual cache systems** → Artwork cache isolated from React Query
5. **No settings mutations** → Settings changes not reflected

---

## Implementation Plan

### Phase 1: Foundation (Week 1)

#### 1.1 Create Invalidation Helpers

**File**: `applications/shared/src/hooks/queries/invalidationHelpers.ts`

```typescript
import { QueryClient } from '@tanstack/react-query'
import { albumKeys, artistKeys, playlistKeys, trackKeys, genreKeys, libraryKeys } from './queryKeys'

/**
 * Invalidates all caches after file scan completes
 * Call this when library import/scan finishes
 */
export function invalidateAfterFileScan(queryClient: QueryClient): void {
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
  queryClient.invalidateQueries({ queryKey: albumKeys.all() })
  queryClient.invalidateQueries({ queryKey: artistKeys.all() })
  queryClient.invalidateQueries({ queryKey: genreKeys.all() })
  queryClient.invalidateQueries({ queryKey: libraryKeys.health() })
}

/**
 * Invalidates caches after album artwork changes
 * Invalidates: album detail, album list (for thumbnails), artwork cache
 */
export function invalidateAfterAlbumArtworkChange(
  queryClient: QueryClient,
  albumId: number
): void {
  // Specific album
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })

  // Album list (thumbnails)
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })

  // Artwork queries (if integrated with React Query)
  queryClient.invalidateQueries({ queryKey: ['artwork', 'album', albumId] })
}

/**
 * Invalidates caches after artist artwork changes
 */
export function invalidateAfterArtistArtworkChange(
  queryClient: QueryClient,
  artistId: number
): void {
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: ['artwork', 'artist', artistId] })
}

/**
 * Invalidates caches after playlist artwork changes
 */
export function invalidateAfterPlaylistArtworkChange(
  queryClient: QueryClient,
  playlistId: string
): void {
  queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
  queryClient.invalidateQueries({ queryKey: playlistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: ['artwork', 'playlist', playlistId] })
}

/**
 * Invalidates caches after album metadata update (title, year, etc.)
 * Also invalidates tracks since they cache album_title
 */
export function invalidateAfterAlbumMetadataUpdate(
  queryClient: QueryClient,
  albumId: number
): void {
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
  queryClient.invalidateQueries({ queryKey: albumKeys.tracks(albumId) })

  // Tracks cache album_title, so must invalidate
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
}

/**
 * Invalidates caches after artist metadata update
 */
export function invalidateAfterArtistMetadataUpdate(
  queryClient: QueryClient,
  artistId: number
): void {
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: artistKeys.tracks(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.albums(artistId) })

  // Tracks cache artist_name
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
}
```

#### 1.2 Update Query Keys

**File**: `applications/shared/src/hooks/queries/queryKeys.ts`

Add artwork keys:

```typescript
export const artworkKeys = {
  all: () => ['artwork'] as const,
  album: (id: number) => [...artworkKeys.all(), 'album', id] as const,
  artist: (id: number) => [...artworkKeys.all(), 'artist', id] as const,
  playlist: (id: string) => [...artworkKeys.all(), 'playlist', id] as const,
  track: (id: string | number) => [...artworkKeys.all(), 'track', id] as const,
}
```

#### 1.3 Create Artwork Mutation Hooks

**File**: `applications/shared/src/hooks/queries/useArtworkMutations.ts`

```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { invalidateAfterAlbumArtworkChange, invalidateAfterArtistArtworkChange, invalidateAfterPlaylistArtworkChange } from './invalidationHelpers'
import { clearArtworkCache } from '../../components/ArtworkImage'

export function useSetArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params: {
      entityType: 'album' | 'artist' | 'playlist'
      entityId: string
      artworkBase64: string
      mimeType: string
      writeToFiles?: boolean
      useSoulStorage?: boolean
    }) => {
      await backend.setArtwork(params)
      return params
    },
    onSuccess: (_data, params) => {
      const { entityType, entityId } = params

      // Clear component-level artwork cache
      clearArtworkCache(entityType, entityId)

      // Invalidate React Query caches
      const id = entityType === 'playlist' ? entityId : Number(entityId)

      if (entityType === 'album') {
        invalidateAfterAlbumArtworkChange(queryClient, id as number)
      } else if (entityType === 'artist') {
        invalidateAfterArtistArtworkChange(queryClient, id as number)
      } else if (entityType === 'playlist') {
        invalidateAfterPlaylistArtworkChange(queryClient, entityId)
      }
    }
  })
}

export function useRemoveArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params: {
      entityType: 'album' | 'artist' | 'playlist'
      entityId: string
    }) => {
      await backend.removeArtwork(params.entityType, params.entityId)
      return params
    },
    onSuccess: (_data, params) => {
      const { entityType, entityId } = params

      // Clear component-level artwork cache
      clearArtworkCache(entityType, entityId)

      // Invalidate React Query caches (same as setArtwork)
      const id = entityType === 'playlist' ? entityId : Number(entityId)

      if (entityType === 'album') {
        invalidateAfterAlbumArtworkChange(queryClient, id as number)
      } else if (entityType === 'artist') {
        invalidateAfterArtistArtworkChange(queryClient, id as number)
      } else if (entityType === 'playlist') {
        invalidateAfterPlaylistArtworkChange(queryClient, entityId)
      }
    }
  })
}
```

#### 1.4 Update EditArtworkDialog

**File**: `applications/shared/src/components/EditArtworkDialog.tsx`

Replace direct backend calls with mutation hooks:

```typescript
import { useSetArtwork, useRemoveArtwork } from '../hooks/queries/useArtworkMutations'

export function EditArtworkDialog({ /* props */ }) {
  const setArtworkMutation = useSetArtwork()
  const removeArtworkMutation = useRemoveArtwork()

  const handleSaveArtwork = async () => {
    await setArtworkMutation.mutateAsync({
      entityType,
      entityId: String(entityId),
      artworkBase64: croppedImageDataUrl,
      mimeType,
      writeToFiles,
      useSoulStorage
    })
    // Cache automatically invalidated, no manual clearArtworkCache needed
    onArtworkChanged?.()
  }

  const handleRemoveArtwork = async () => {
    await removeArtworkMutation.mutateAsync({
      entityType,
      entityId: String(entityId)
    })
    // Cache automatically invalidated
    onArtworkChanged?.()
  }
}
```

---

### Phase 2: Scan Completion Hook (Week 2)

#### 2.1 Create Scan Completion Hook

**File**: `applications/shared/src/hooks/useScanCompletionInvalidation.ts`

```typescript
import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { invalidateAfterFileScan } from './queries/invalidationHelpers'

/**
 * Hook that listens for scan completion events and invalidates caches
 * Usage: Call once in App.tsx root component
 */
export function useScanCompletionInvalidation() {
  const queryClient = useQueryClient()

  useEffect(() => {
    // Listen for Tauri events (desktop)
    const unlistenPromise = window.__TAURI__?.event.listen('scan-complete', () => {
      console.log('[Cache] Library scan completed, invalidating caches...')
      invalidateAfterFileScan(queryClient)
    })

    return () => {
      unlistenPromise?.then(unlisten => unlisten())
    }
  }, [queryClient])
}
```

#### 2.2 Integrate in App.tsx

**File**: `applications/desktop/src/App.tsx`

```typescript
import { useScanCompletionInvalidation } from '@soul-player/shared'

export function App() {
  useScanCompletionInvalidation() // Add this line

  // ... rest of app
}
```

#### 2.3 Emit Event from Rust (Backend)

**File**: `applications/desktop/src-tauri/src/scanner.rs`

```rust
use tauri::Manager;

pub async fn scan_library(app_handle: tauri::AppHandle) -> Result<ScanResult> {
    // ... scanning logic ...

    // After scan completes
    app_handle.emit_all("scan-complete", ()).expect("Failed to emit scan-complete event");

    Ok(result)
}
```

---

### Phase 3: Album & Artist Mutations (Week 2-3)

#### 3.1 Create Album Mutation Hooks

**File**: `applications/shared/src/hooks/queries/useAlbumMutations.ts`

```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { invalidateAfterAlbumMetadataUpdate } from './invalidationHelpers'

export function useUpdateAlbum() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params: {
      albumId: number
      title?: string
      year?: number
      artistId?: number
    }) => {
      await backend.updateAlbum(params)
      return params
    },
    onSuccess: (_data, { albumId }) => {
      invalidateAfterAlbumMetadataUpdate(queryClient, albumId)
    }
  })
}
```

#### 3.2 Add Backend Method

**File**: `applications/shared/src/contexts/BackendContext.tsx`

```typescript
export interface BackendInterface {
  // ... existing methods ...

  updateAlbum(params: {
    albumId: number
    title?: string
    year?: number
    artistId?: number
  }): Promise<void>

  updateArtist(params: {
    artistId: number
    name?: string
  }): Promise<void>
}
```

#### 3.3 Implement Rust Command

**File**: `applications/desktop/src-tauri/src/commands/albums.rs`

```rust
#[tauri::command]
pub async fn update_album(
    album_id: i64,
    title: Option<String>,
    year: Option<i32>,
    artist_id: Option<i64>,
    storage: tauri::State<'_, StorageState>,
) -> Result<(), String> {
    let pool = &storage.pool;

    // Call storage layer
    soul_storage::albums::update_album(pool, album_id, title, year, artist_id)
        .await
        .map_err(|e| e.to_string())
}
```

**File**: `libraries/soul-storage/src/albums.rs`

```rust
pub async fn update_album(
    pool: &SqlitePool,
    album_id: i64,
    title: Option<String>,
    year: Option<i32>,
    artist_id: Option<i64>,
) -> Result<()> {
    let mut query = "UPDATE albums SET".to_string();
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();

    if let Some(t) = title {
        updates.push(" title = ?");
        values.push(Box::new(t));
    }
    if let Some(y) = year {
        updates.push(" year = ?");
        values.push(Box::new(y));
    }
    if let Some(a) = artist_id {
        updates.push(" artist_id = ?");
        values.push(Box::new(a));
    }

    if updates.is_empty() {
        return Ok(()); // Nothing to update
    }

    query.push_str(&updates.join(","));
    query.push_str(" WHERE id = ?");
    values.push(Box::new(album_id));

    sqlx::query(&query)
        .bind_all(values)
        .execute(pool)
        .await?;

    Ok(())
}
```

---

### Phase 4: Testing (Week 3)

#### 4.1 Unit Tests for Helpers

**File**: `applications/shared/src/hooks/queries/invalidationHelpers.test.ts`

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { QueryClient } from '@tanstack/react-query'
import { invalidateAfterAlbumArtworkChange, invalidateAfterFileScan } from './invalidationHelpers'
import { albumKeys, trackKeys } from './queryKeys'

describe('invalidationHelpers', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient()
  })

  it('invalidates album detail and lists after artwork change', () => {
    // Pre-populate cache
    queryClient.setQueryData(albumKeys.detail(1), { id: 1, title: 'Test' })
    queryClient.setQueryData(albumKeys.list(), [{ id: 1, title: 'Test' }])

    // Call helper
    invalidateAfterAlbumArtworkChange(queryClient, 1)

    // Check invalidation
    const detailState = queryClient.getQueryState(albumKeys.detail(1))
    const listState = queryClient.getQueryState(albumKeys.list())

    expect(detailState?.isInvalidated).toBe(true)
    expect(listState?.isInvalidated).toBe(true)
  })

  it('invalidates all library caches after scan', () => {
    // Pre-populate
    queryClient.setQueryData(trackKeys.list(), [])
    queryClient.setQueryData(albumKeys.list(), [])

    // Call helper
    invalidateAfterFileScan(queryClient)

    // Verify
    expect(queryClient.getQueryState(trackKeys.list())?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(albumKeys.list())?.isInvalidated).toBe(true)
  })
})
```

#### 4.2 Integration Test for Mutation

**File**: `applications/shared/src/hooks/queries/useArtworkMutations.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useSetArtwork } from './useArtworkMutations'
import { BackendProvider } from '../../contexts/BackendContext'
import { albumKeys } from './queryKeys'

describe('useSetArtwork', () => {
  it('invalidates album cache after artwork change', async () => {
    const mockBackend = {
      setArtwork: vi.fn().mockResolvedValue(undefined)
    }

    const queryClient = new QueryClient()
    const wrapper = ({ children }) => (
      <QueryClientProvider client={queryClient}>
        <BackendProvider value={mockBackend}>
          {children}
        </BackendProvider>
      </QueryClientProvider>
    )

    // Pre-populate album cache
    queryClient.setQueryData(albumKeys.detail(1), { id: 1, title: 'Album' })

    const { result } = renderHook(() => useSetArtwork(), { wrapper })

    // Trigger mutation
    result.current.mutate({
      entityType: 'album',
      entityId: '1',
      artworkBase64: 'base64data',
      mimeType: 'image/jpeg'
    })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    // Verify backend was called
    expect(mockBackend.setArtwork).toHaveBeenCalledWith({
      entityType: 'album',
      entityId: '1',
      artworkBase64: 'base64data',
      mimeType: 'image/jpeg'
    })

    // Verify cache was invalidated
    const albumState = queryClient.getQueryState(albumKeys.detail(1))
    expect(albumState?.isInvalidated).toBe(true)
  })
})
```

#### 4.3 E2E Test

**File**: `applications/desktop/tests/e2e/cache-invalidation.spec.ts`

```typescript
import { test, expect } from '@playwright/test'

test('artwork change updates all views', async ({ page }) => {
  // Navigate to album page
  await page.goto('/albums/1')

  // Get initial artwork src
  const initialSrc = await page.locator('[data-testid="album-artwork"]').getAttribute('src')

  // Open edit dialog
  await page.click('[data-testid="edit-artwork-button"]')

  // Upload new artwork
  await page.setInputFiles('[data-testid="artwork-file-input"]', 'tests/fixtures/new-artwork.jpg')

  // Save
  await page.click('[data-testid="save-artwork-button"]')

  // Wait for mutation to complete
  await page.waitForTimeout(500)

  // Verify artwork updated on album page
  const newSrc = await page.locator('[data-testid="album-artwork"]').getAttribute('src')
  expect(newSrc).not.toBe(initialSrc)

  // Navigate to albums list
  await page.goto('/albums')

  // Verify thumbnail also updated (cache invalidation worked)
  const thumbnailSrc = await page.locator(`[data-testid="album-card-1"] img`).getAttribute('src')
  expect(thumbnailSrc).toContain('new-artwork')
})

test('file scan updates library without refresh', async ({ page }) => {
  await page.goto('/library')

  // Get initial album count
  const initialCount = await page.locator('[data-testid="album-count"]').textContent()

  // Trigger scan (via settings or scan button)
  await page.click('[data-testid="scan-library-button"]')

  // Wait for scan to complete (watch for event or UI indicator)
  await page.waitForSelector('[data-testid="scan-complete-indicator"]')

  // Verify count updated automatically (no refresh needed)
  const newCount = await page.locator('[data-testid="album-count"]').textContent()
  expect(Number(newCount)).toBeGreaterThan(Number(initialCount))
})
```

---

## Success Metrics

### Performance
- ✅ Artwork cache invalidation < 50ms
- ✅ Scan completion invalidation < 200ms
- ✅ No unnecessary refetches (verify in DevTools)

### Quality
- ✅ All unit tests passing
- ✅ All integration tests passing
- ✅ E2E tests covering critical flows

### User Experience
- ✅ No manual refresh needed after artwork change
- ✅ No manual refresh needed after file import
- ✅ Changes visible immediately across all views

---

## Rollback Plan

If issues arise:

1. **Revert Phase 1 only**: Remove mutation hooks, keep old EditArtworkDialog
2. **Revert scan hook**: Remove `useScanCompletionInvalidation` call
3. **Full rollback**: Revert entire feature branch

All changes are backward compatible - old code continues to work.

---

## Future Enhancements

### Phase 5: Settings Mutations (Optional)
- Create `useUpdateSetting()` hook
- Invalidate settings queries on change

### Phase 6: Real-time Sync (Optional)
- WebSocket integration for multi-instance sync
- Use query invalidation pattern (WebSocket → invalidate → refetch)

### Phase 7: Artwork React Query Integration (Optional)
- Replace component-level `artworkCache` Map
- Migrate to TanStack Query for unified cache management

---

## Related Research

- [TanStack Query Invalidation Guide](https://tanstack.com/query/v5/docs/framework/react/guides/query-invalidation)
- [Effective React Query Keys](https://tkdodo.eu/blog/effective-react-query-keys)
- [Optimistic Updates](https://tanstack.com/query/latest/docs/framework/react/guides/optimistic-updates)
- Soul Player codebase analysis (see research deliverables)

---

**Last Updated**: 2026-02-11
**Status**: Ready for implementation
**Est. Timeline**: 3 weeks
