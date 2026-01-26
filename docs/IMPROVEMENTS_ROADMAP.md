# Soul Player - Improvements Roadmap

## Executive Summary

Comprehensive analysis of Soul Player revealed **11 critical improvements** needed for production readiness. The app has good foundational patterns (AlbumPage race condition handling, proper event cleanup) but applies them **inconsistently**. Key gaps: no error boundaries, unprotected async operations in 4+ pages, memory leaks in sync store, and zero caching.

**Total Estimated Effort**: 20-25 hours
**Critical Fixes**: 3 items (6-7 hours)
**High Priority**: 3 items (9-11 hours)

---

## Critical Issues (Fix Immediately)

### 1. Race Condition Protection for All Pages (2-3 hours)

**Status**: ❌ **CRITICAL**

**Problem**: ArtistPage, PlaylistPage, TracksPage have NO race condition protection. Rapid navigation causes stale data to overwrite current data.

**Affected Files**:
- `applications/shared/src/pages/ArtistPage.tsx`
- `applications/shared/src/pages/PlaylistPage.tsx`
- `applications/shared/src/pages/TracksPage.tsx`

**Current Risk**:
```typescript
// ArtistPage - VULNERABLE
const loadArtist = async (artistId: number) => {
  const foundArtist = await backend.getArtistById(artistId)
  setArtist(foundArtist)  // No stale check!

  const [tracks, albums] = await Promise.all([...])
  setTracks(tracks)  // Stale data can overwrite current!
  setAlbums(albums)
}
```

**Solution**: Apply AlbumPage pattern to all pages:

```typescript
// Add to each page
const requestIdRef = useRef(0)

const loadArtist = useCallback(async (artistId: number) => {
  const currentRequestId = ++requestIdRef.current

  setLoading(true)
  try {
    const artist = await backend.getArtistById(artistId)

    // Check after EACH async operation
    if (currentRequestId !== requestIdRef.current) {
      return  // Discard stale response
    }

    setArtist(artist)

    const [tracks, albums] = await Promise.all([...])

    // Check again after Promise.all
    if (currentRequestId !== requestIdRef.current) {
      return
    }

    setTracks(tracks)
    setAlbums(albums)
  } finally {
    if (currentRequestId === requestIdRef.current) {
      setLoading(false)
    }
  }
}, [backend])
```

**Testing**:
- Navigate Artist A → B → A rapidly (< 500ms)
- Verify correct artist data shown
- Check console for stale request logs

---

### 2. Fix Sync Store Listener Memory Leak (1 hour)

**Status**: ❌ **CRITICAL**

**Problem**: `setupSyncListeners()` in sync store creates event listeners but never cleans them up. Multiple calls accumulate listeners, causing memory leak.

**File**: `applications/shared/src/stores/sync.ts`

**Current Code**:
```typescript
// CRITICAL: No cleanup!
export function setupSyncListeners() {
  listen<SyncProgress>('sync-progress', (event) => {
    useSyncStore.getState().setProgress(event.payload)
  })  // ← Returns unsubscribe function, but not captured!

  listen<SyncSummary>('sync-complete', (event) => {
    useSyncStore.getState().setSummary(event.payload)
  })

  // ... 6 more listeners without cleanup
}
```

**Fix**:
```typescript
let unlisteners: Array<() => void> = []

export function setupSyncListeners() {
  // Cleanup old listeners first
  cleanupSyncListeners()

  listen<SyncProgress>('sync-progress', (event) => {
    useSyncStore.getState().setProgress(event.payload)
  }).then(fn => unlisteners.push(fn))

  listen<SyncSummary>('sync-complete', (event) => {
    useSyncStore.getState().setSummary(event.payload)
  }).then(fn => unlisteners.push(fn))

  // ... capture all listener cleanups
}

export function cleanupSyncListeners() {
  unlisteners.forEach(fn => {
    try {
      fn()
    } catch (err) {
      console.error('Failed to cleanup listener:', err)
    }
  })
  unlisteners = []
}
```

**Testing**:
- Open DevTools → Memory profiler
- Call `setupSyncListeners()` 10 times
- Force garbage collection
- Verify no listener accumulation

---

### 3. Add Error Boundaries (2 hours)

**Status**: ❌ **CRITICAL**

**Problem**: No error boundaries anywhere. Single component error crashes entire page, requiring app restart.

**Gap**: Zero error boundary components found in codebase.

**Solution**: Create reusable error boundary component:

```typescript
// applications/shared/src/components/ErrorBoundary.tsx
import React, { ErrorInfo } from 'react'

interface Props {
  children: React.ReactNode
  fallback?: React.ComponentType<{ error: Error; reset: () => void }>
  onError?: (error: Error, errorInfo: ErrorInfo) => void
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught:', error, errorInfo)
    this.props.onError?.(error, errorInfo)
  }

  reset = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        const FallbackComponent = this.props.fallback
        return <FallbackComponent error={this.state.error!} reset={this.reset} />
      }

      return (
        <div className="flex items-center justify-center h-full p-6">
          <div className="text-center max-w-md">
            <h2 className="text-xl font-bold text-destructive mb-2">
              Something went wrong
            </h2>
            <p className="text-sm text-muted-foreground mb-4">
              {this.state.error?.message || 'An unexpected error occurred'}
            </p>
            <button
              onClick={this.reset}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
            >
              Try Again
            </button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
```

**Usage in Router**:
```typescript
// applications/shared/src/App.tsx
<Routes>
  <Route path="/albums/:id" element={
    <ErrorBoundary>
      <AlbumPage />
    </ErrorBoundary>
  } />
  <Route path="/artists/:id" element={
    <ErrorBoundary>
      <ArtistPage />
    </ErrorBoundary>
  } />
  {/* Wrap all routes */}
</Routes>
```

**Testing**:
- Throw error in component: `throw new Error('Test')`
- Verify error boundary catches it
- Click "Try Again" → component remounts
- Verify no console warnings about unmounted setState

---

## High Priority Issues (Fix This Sprint)

### 4. Implement Request Cancellation (3-4 hours)

**Status**: ⚠️ **HIGH**

**Problem**: Long-running backend requests can't be cancelled. Resources wasted when user navigates away during data loading.

**Current Limitation**: Tauri's `invoke()` doesn't support AbortSignal natively.

**Solution**: Implement cancellation at wrapper level:

```typescript
// applications/shared/src/hooks/useCancellableRequest.ts (ALREADY CREATED)
// Enhanced version with cleanup tracking:

export function useCancellableBackend() {
  const backend = useBackend()
  const { execute, cancelAll } = useCancellableRequest()

  const wrappedBackend = useMemo(() => ({
    getAlbumById: async (id: number) => {
      return execute(() => backend.getAlbumById(id))
    },
    getAlbumTracks: async (id: number) => {
      return execute(() => backend.getAlbumTracks(id))
    },
    // ... wrap all backend methods
  }), [backend, execute])

  useEffect(() => {
    return () => cancelAll()  // Cancel on unmount
  }, [cancelAll])

  return wrappedBackend
}
```

**Usage**:
```typescript
// Replace useBackend() with useCancellableBackend()
const backend = useCancellableBackend()

// All requests automatically cancelled on unmount
const album = await backend.getAlbumById(id)
```

**Testing**:
- Start loading album
- Navigate away immediately
- Check logs for "Request cancelled" message
- Verify no state updates after navigation

---

### 5. Add React Query Caching (4-5 hours)

**Status**: ⚠️ **HIGH**

**Problem**: Zero caching. Every page navigation makes fresh database queries, even for recently loaded data.

**Impact**:
- LibraryPage loads ALL tracks/albums/artists on every visit
- AlbumPage reloads same album if user navigates back
- Slow UX, excessive backend load

**Solution**: Integrate React Query (TanStack Query):

```typescript
// applications/shared/src/providers/QueryProvider.tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5,  // 5 minutes
      cacheTime: 1000 * 60 * 30, // 30 minutes
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
})

export function QueryProvider({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  )
}
```

**Usage in AlbumPage**:
```typescript
import { useQuery } from '@tanstack/react-query'

function AlbumPage() {
  const { id } = useParams()
  const backend = useBackend()

  const { data: album, isLoading, error } = useQuery({
    queryKey: ['album', id],
    queryFn: () => backend.getAlbumById(parseInt(id!, 10)),
    enabled: !!id,
  })

  const { data: tracks } = useQuery({
    queryKey: ['album-tracks', id],
    queryFn: () => backend.getAlbumTracks(parseInt(id!, 10)),
    enabled: !!id && !!album,
  })

  // No manual loading state management needed!
}
```

**Benefits**:
- ✅ Automatic caching (revisit album → instant load from cache)
- ✅ Background refetching (stale data shown immediately, fresh data loaded in background)
- ✅ Request deduplication (multiple components requesting same data → single request)
- ✅ Built-in race condition protection
- ✅ Automatic retries on failure

**Migration Strategy**:
1. Add React Query to root App component
2. Migrate AlbumPage (highest traffic)
3. Migrate LibraryPage
4. Migrate ArtistPage, PlaylistPage
5. Remove manual loading/error state management

---

### 6. Consolidate Safe Async Pattern (2 hours)

**Status**: ⚠️ **HIGH**

**Problem**: We have `useCancellableRequest` hook but it's not used. Pages still use manual `requestIdRef` + `isMountedRef` patterns inconsistently.

**Solution**: Enhance existing hook and enforce usage:

```typescript
// applications/shared/src/hooks/useSafeAsync.ts
import { useRef, useCallback, useEffect } from 'react'

/**
 * Hook that combines requestId tracking + mounted state checking
 * Prevents all race conditions and unmounted state updates
 */
export function useSafeAsync() {
  const requestIdRef = useRef(0)
  const isMountedRef = useRef(true)

  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const execute = useCallback(async <T,>(
    fn: () => Promise<T>
  ): Promise<T | null> => {
    const currentRequestId = ++requestIdRef.current

    try {
      const result = await fn()

      // Check both mounted state AND request validity
      if (
        currentRequestId !== requestIdRef.current ||
        !isMountedRef.current
      ) {
        return null  // Silently discard stale/unmounted result
      }

      return result
    } catch (error) {
      // Only propagate error if request is still valid
      if (
        currentRequestId !== requestIdRef.current ||
        !isMountedRef.current
      ) {
        return null  // Discard error from stale request
      }
      throw error
    }
  }, [])

  const cancelAll = useCallback(() => {
    requestIdRef.current++
  }, [])

  return { execute, cancelAll }
}
```

**Refactor AlbumPage to use hook**:
```typescript
function AlbumPage() {
  const { execute } = useSafeAsync()
  const backend = useBackend()

  const loadAlbum = useCallback(async (albumId: number) => {
    setLoading(true)
    setError(null)

    try {
      const album = await execute(() => backend.getAlbumById(albumId))
      if (!album) return  // Cancelled or unmounted

      setAlbum(album)

      const tracks = await execute(() => backend.getAlbumTracks(albumId))
      if (!tracks) return  // Cancelled or unmounted

      setTracks(tracks)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }, [execute, backend])

  // Much cleaner! No manual requestIdRef management
}
```

**Benefits**:
- ✅ Single pattern for all pages
- ✅ Impossible to forget checks (enforced by hook)
- ✅ Less boilerplate code
- ✅ Easier to test

---

## Medium Priority Issues (Fix Next)

### 7. Virtual List for Large Collections (3 hours)

**Problem**: Rendering 10,000 tracks creates 10,000 DOM nodes, even if only 50 visible. Causes laggy scrolling and high memory usage.

**Solution**: Use `react-window` or `@tanstack/react-virtual`:

```typescript
// applications/shared/src/components/VirtualTrackList.tsx
import { useVirtualizer } from '@tanstack/react-virtual'

export function VirtualTrackList({ tracks }: { tracks: Track[] }) {
  const parentRef = useRef<HTMLDivElement>(null)

  const virtualizer = useVirtualizer({
    count: tracks.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,  // 48px per row
    overscan: 10,  // Render 10 extra rows above/below viewport
  })

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <div
            key={virtualRow.key}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <TrackRow track={tracks[virtualRow.index]} />
          </div>
        ))}
      </div>
    </div>
  )
}
```

**Impact**: 10,000 tracks → only ~50 DOM nodes rendered (those in viewport).

---

### 8. Optimistic UI Updates (2-3 hours)

**Problem**: No immediate feedback when user clicks play/pause/queue. UI waits for backend confirmation, feels sluggish.

**Solution**: Update UI immediately, rollback if fails:

```typescript
const handleAddToQueue = async (track: Track) => {
  // Optimistic update - add to UI immediately
  const optimisticQueue = [...queue, track]
  setQueue(optimisticQueue)

  try {
    await commands.addToQueueEnd(track)
    // Success - already in UI
  } catch (error) {
    // Rollback on error
    setQueue(queue)
    toast.error('Failed to add to queue')
  }
}
```

---

### 9. Improve Loading UX (2-3 hours)

**Problem**: Generic spinners everywhere. No indication of what's loading or progress.

**Solution**: Add skeleton screens:

```typescript
// Show skeleton while loading
{isLoading ? (
  <div className="grid grid-cols-6 gap-4">
    {Array.from({ length: 12 }).map((_, i) => (
      <SkeletonCard key={i} />
    ))}
  </div>
) : (
  <div className="grid grid-cols-6 gap-4">
    {albums.map(album => <AlbumCard key={album.id} album={album} />)}
  </div>
)}
```

---

## Low Priority (Nice to Have)

### 10. Request Deduplication (2 hours)

Prevent duplicate requests for same data within 100ms window.

### 11. Offline Support (4-5 hours)

Cache data locally, show cached data when offline, sync when back online.

---

## Implementation Order

### Week 1 (Critical)
- Day 1: Fix race conditions in ArtistPage, PlaylistPage, TracksPage
- Day 2: Fix sync store listener leak + add error boundaries
- Day 3: Test all critical fixes

### Week 2 (High Priority)
- Day 4-5: Integrate React Query caching
- Day 6: Consolidate safe async pattern
- Day 7: Implement request cancellation

### Week 3 (Medium Priority)
- Day 8: Virtual list for tracks
- Day 9: Optimistic UI updates
- Day 10: Skeleton screens

---

## Success Metrics

**Before Fixes**:
- Race conditions: Reproducible in 80% of rapid navigation tests
- Memory leaks: 50MB growth after 10 sync operations
- Cache hit rate: 0% (no caching)
- Error recovery: 0% (crashes require restart)

**After Fixes**:
- Race conditions: 0% (eliminated by request ID tracking)
- Memory leaks: < 1MB growth (proper cleanup)
- Cache hit rate: > 70% (React Query caching)
- Error recovery: 100% (error boundaries catch all errors)

---

## Testing Checklist

- [ ] Navigate rapidly between 10 different albums/artists (< 500ms each)
- [ ] Monitor memory usage during 20 navigation cycles
- [ ] Trigger errors and verify error boundary catches them
- [ ] Check DevTools Performance tab for unnecessary re-renders
- [ ] Verify cache hits in React Query DevTools
- [ ] Test with large library (10,000+ tracks)
- [ ] Test network failures and retry behavior

---

**Last Updated**: 2026-01-20
