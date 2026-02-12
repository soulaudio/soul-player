# Cache Invalidation Implementation Checklist

Step-by-step checklist for implementing cache invalidation in Soul Player.

---

## Phase 1: Foundation Setup

### 1.1 Create Helper Files

- [ ] Create `applications/shared/src/hooks/queries/invalidationHelpers.ts`
  - [ ] Add `invalidateAfterAlbumArtworkChange()`
  - [ ] Add `invalidateAfterArtistArtworkChange()`
  - [ ] Add `invalidateAfterPlaylistArtworkChange()`
  - [ ] Add `invalidateAfterAlbumMetadataUpdate()`
  - [ ] Add `invalidateAfterArtistMetadataUpdate()`
  - [ ] Add `invalidateAfterFileScanComplete()`
  - [ ] Add `invalidateAfterTrackDeletion()`
  - [ ] Add JSDoc comments to each function
  - [ ] Export all functions

### 1.2 Update Query Keys

- [ ] Open `applications/shared/src/hooks/queries/queryKeys.ts`
  - [ ] Add `artworkKeys` object:
    - [ ] `artworkKeys.all()`
    - [ ] `artworkKeys.album(id)`
    - [ ] `artworkKeys.artist(id)`
    - [ ] `artworkKeys.playlist(id)`
  - [ ] Add `albumKeys.artwork(id)` method
  - [ ] Add `artistKeys.artwork(id)` method
  - [ ] Add `playlistKeys.artwork(id)` method
  - [ ] Verify all keys follow hierarchical pattern

### 1.3 Create Unit Tests

- [ ] Create `applications/shared/src/hooks/queries/__tests__/invalidationHelpers.test.ts`
  - [ ] Test each invalidation helper function
  - [ ] Mock `QueryClient.invalidateQueries`
  - [ ] Verify correct query keys invalidated
  - [ ] Test cascade behavior
  - [ ] Verify artwork cache clearing

- [ ] Create `applications/shared/src/hooks/queries/__tests__/queryKeys.test.ts`
  - [ ] Test key hierarchy (parent keys are prefixes)
  - [ ] Test all domain key factories
  - [ ] Verify type safety

---

## Phase 2: Mutation Hooks

### 2.1 Create Artwork Mutations

- [ ] Create `applications/shared/src/hooks/queries/useArtworkMutations.ts`

  **useSetArtwork**:
  - [ ] Define `SetArtworkVariables` interface
  - [ ] Implement mutation with `backend.setArtwork()`
  - [ ] Add `onSuccess` invalidation:
    - [ ] Call appropriate helper based on `entityType`
    - [ ] Test with different entity types
  - [ ] Add JSDoc with usage example
  - [ ] Export hook

  **useRemoveArtwork**:
  - [ ] Define variables interface
  - [ ] Implement mutation with `backend.removeArtwork()`
  - [ ] Add `onSuccess` invalidation
  - [ ] Add JSDoc
  - [ ] Export hook

- [ ] Create unit tests for both hooks
  - [ ] Test successful artwork change
  - [ ] Test error handling
  - [ ] Verify invalidation calls

### 2.2 Create Album Mutations

- [ ] Create `applications/shared/src/hooks/queries/useAlbumMutations.ts`

  **useUpdateAlbum**:
  - [ ] Define `UpdateAlbumVariables` interface
  - [ ] Implement mutation (NOTE: Requires new backend command)
  - [ ] Add `onSuccess` invalidation:
    - [ ] Call `invalidateAfterAlbumMetadataUpdate()`
    - [ ] Pass artist change info if applicable
  - [ ] Add JSDoc
  - [ ] Export hook

- [ ] Create unit tests
  - [ ] Test metadata update
  - [ ] Test artist change handling
  - [ ] Verify cascade invalidation

### 2.3 Create Artist Mutations

- [ ] Create `applications/shared/src/hooks/queries/useArtistMutations.ts`

  **useUpdateArtist**:
  - [ ] Define `UpdateArtistVariables` interface
  - [ ] Implement mutation (NOTE: Requires new backend command)
  - [ ] Add `onSuccess` invalidation:
    - [ ] Call `invalidateAfterArtistMetadataUpdate()`
  - [ ] Add JSDoc
  - [ ] Export hook

- [ ] Create unit tests
  - [ ] Test name update
  - [ ] Test sort name update
  - [ ] Verify broad invalidation (albums, tracks)

### 2.4 Update Track Mutations

- [ ] Open `applications/shared/src/hooks/queries/useTrackMutations.ts`

  **Modify useDeleteTrack**:
  - [ ] Update to use new backend return type (affected IDs)
  - [ ] Replace manual invalidations with `invalidateAfterTrackDeletion()`
  - [ ] Update JSDoc
  - [ ] Test with mock backend

- [ ] Update unit tests
  - [ ] Mock new return type
  - [ ] Verify granular invalidation
  - [ ] Test with multiple affected playlists

---

## Phase 3: Backend Changes

### 3.1 Add New Tauri Commands (Rust)

- [ ] Open `applications/desktop/src-tauri/src/commands/library.rs`

  **update_album**:
  - [ ] Define command signature
  - [ ] Call `soul-storage` update function
  - [ ] Handle errors properly
  - [ ] Add to Tauri builder
  - [ ] Test via Tauri devtools

  **update_artist**:
  - [ ] Define command signature
  - [ ] Call `soul-storage` update function
  - [ ] Handle errors
  - [ ] Add to Tauri builder
  - [ ] Test

- [ ] Open `applications/desktop/src-tauri/src/commands/tracks.rs`

  **Modify delete_track**:
  - [ ] Create `DeleteTrackResult` struct
  - [ ] Query affected album/artist/playlists BEFORE deletion
  - [ ] Perform deletion
  - [ ] Return affected IDs
  - [ ] Update integration tests

### 3.2 Add Storage Layer Functions

- [ ] Open `libraries/soul-storage/src/albums.rs`
  - [ ] Add `update_album()` function
  - [ ] Use compile-time `query!` macro
  - [ ] Include `user_id` in WHERE clause
  - [ ] Add unit tests

- [ ] Open `libraries/soul-storage/src/artists.rs`
  - [ ] Add `update_artist()` function
  - [ ] Use compile-time `query!` macro
  - [ ] Include `user_id`
  - [ ] Add unit tests

- [ ] Open `libraries/soul-storage/src/tracks.rs`
  - [ ] Add `delete_track_with_affected()` function
  - [ ] Query related entities before delete
  - [ ] Return `DeleteTrackResult`
  - [ ] Add unit tests

### 3.3 Update Backend Interface

- [ ] Open `applications/shared/src/contexts/BackendContext.tsx`
  - [ ] Add `updateAlbum()` method signature
  - [ ] Add `updateArtist()` method signature
  - [ ] Update `deleteTrack()` return type
  - [ ] Update JSDoc

### 3.4 Implement in Tauri Provider

- [ ] Open `applications/desktop/src/providers/TauriBackendProvider.tsx`
  - [ ] Implement `updateAlbum()` with `invoke('update_album', ...)`
  - [ ] Implement `updateArtist()` with `invoke('update_artist', ...)`
  - [ ] Update `deleteTrack()` to handle new return type
  - [ ] Test each implementation

### 3.5 Implement in Mock Providers

- [ ] Update `applications/shared/src/providers/MockBackendProvider.tsx`
  - [ ] Add mock implementations (return success)
  - [ ] Add to mock backend interface
  - [ ] Update demo data as needed (via DemoStorage)

---

## Phase 4: Artwork Cache Improvements

### 4.1 Update ArtworkImage Component

- [ ] Open `applications/shared/src/components/ArtworkImage.tsx`

  **Timestamp-based caching**:
  - [ ] Change cache structure to include timestamp:
    ```typescript
    const artworkCache = new Map<string, {
      dataUrl: string
      timestamp: number
    }>()
    ```
  - [ ] Add expiration check (e.g., 60 seconds)
  - [ ] Clear expired entries on access
  - [ ] Update `clearArtworkCache()` to work with new structure
  - [ ] Update `clearAllArtworkCache()`

  **Cache listener improvements**:
  - [ ] Verify listener cleanup in useEffect
  - [ ] Test listener notifications
  - [ ] Add debug logging (dev only)

- [ ] Update component tests
  - [ ] Test timestamp expiration
  - [ ] Test cache invalidation notifications
  - [ ] Test fallback icon display

### 4.2 Update EditArtworkDialog

- [ ] Open `applications/shared/src/components/EditArtworkDialog.tsx`

  **Replace manual invalidation with mutation**:
  - [ ] Import `useSetArtwork` and `useRemoveArtwork`
  - [ ] Replace `handleSave()` to use mutation
  - [ ] Replace `handleRemove()` to use mutation
  - [ ] Remove manual `clearArtworkCache()` calls (mutation handles it)
  - [ ] Remove `onArtworkChanged` callback (no longer needed)
  - [ ] Update loading/error states
  - [ ] Test dialog flow end-to-end

- [ ] Update all usages of `EditArtworkDialog`:
  - [ ] Remove `onArtworkChanged` prop
  - [ ] Verify UI updates automatically after save

---

## Phase 5: Event-Driven Invalidation

### 5.1 Create Scan Completion Hook

- [ ] Create `applications/shared/src/hooks/useScanCompletionInvalidation.ts`
  - [ ] Check for Tauri environment
  - [ ] Listen for `scan-complete` event
  - [ ] Call `invalidateAfterFileScanComplete()` on event
  - [ ] Clean up listener on unmount
  - [ ] Add debug logging
  - [ ] Export hook

- [ ] Create unit tests
  - [ ] Mock Tauri event system
  - [ ] Verify listener setup
  - [ ] Verify cleanup
  - [ ] Test no-op on web platforms

### 5.2 Integrate in Desktop App

- [ ] Open `applications/desktop/src/App.tsx`
  - [ ] Import `useScanCompletionInvalidation`
  - [ ] Add hook call at top of component
  - [ ] Test with actual library scan

- [ ] Open `applications/desktop/src/components/ScanProgressIndicator.tsx`
  - [ ] Remove `onComplete` callback prop (no longer needed)
  - [ ] Remove manual refresh logic
  - [ ] Update component tests

### 5.3 Verify Scan Event Emission

- [ ] Open `applications/desktop/src-tauri/src/scanner/mod.rs` (or equivalent)
  - [ ] Verify `scan-complete` event is emitted
  - [ ] Ensure event includes `sourceId` payload
  - [ ] Test event emission in integration tests

---

## Phase 6: Integration & Testing

### 6.1 Integration Tests

- [ ] Create `applications/shared/src/hooks/queries/__tests__/integration/artworkChange.test.tsx`
  - [ ] Test full artwork change flow
  - [ ] Verify React Query cache invalidation
  - [ ] Verify artwork cache clearing
  - [ ] Test UI component updates

- [ ] Create `applications/shared/src/hooks/queries/__tests__/integration/trackDeletion.test.tsx`
  - [ ] Test cascade invalidation
  - [ ] Verify all affected entities update
  - [ ] Test playlist removal

- [ ] Create `applications/shared/src/hooks/queries/__tests__/integration/scanCompletion.test.tsx`
  - [ ] Mock Tauri event
  - [ ] Verify broad invalidation
  - [ ] Test UI updates

### 6.2 E2E Tests

- [ ] Create `applications/desktop/e2e-tests/cacheInvalidation.spec.ts`

  **File Import Test**:
  - [ ] Import new album folder
  - [ ] Wait for scan completion
  - [ ] Navigate to Library → Albums
  - [ ] Verify new album appears (no refresh)

  **Artwork Change Test**:
  - [ ] Open album detail page
  - [ ] Edit artwork
  - [ ] Verify detail page updates
  - [ ] Navigate to library grid
  - [ ] Verify thumbnail updates

  **Track Deletion Test**:
  - [ ] View album with multiple tracks
  - [ ] Delete one track
  - [ ] Verify track count decrements
  - [ ] Navigate to artist page
  - [ ] Verify artist track count updates

### 6.3 Manual Testing

- [ ] Test artwork change flow:
  - [ ] Album artwork
  - [ ] Artist artwork
  - [ ] Playlist artwork
  - [ ] Verify updates in all views

- [ ] Test metadata editing:
  - [ ] Edit album title → verify lists update
  - [ ] Edit album year → verify filtered views
  - [ ] Edit artist name → verify albums/tracks update

- [ ] Test track deletion:
  - [ ] Delete track → verify album/artist counts
  - [ ] Delete track in playlist → verify playlist updates
  - [ ] Delete last track in album → verify album handling

- [ ] Test scan completion:
  - [ ] Add new files to watched folder
  - [ ] Trigger scan
  - [ ] Verify UI updates without refresh
  - [ ] Check performance with large scans

---

## Phase 7: Performance Validation

### 7.1 React Query Devtools

- [ ] Enable devtools in development
- [ ] Monitor query invalidations during mutations
- [ ] Check for over-invalidation (unnecessary refetches)
- [ ] Verify background queries stay stale until accessed
- [ ] Measure network traffic per operation

### 7.2 Artwork Cache Monitoring

- [ ] Add cache size logging
- [ ] Monitor cache hit rate
- [ ] Verify LRU eviction (if implemented)
- [ ] Test with 100+ cached artworks
- [ ] Measure memory usage

### 7.3 Performance Benchmarks

- [ ] Benchmark granular vs broad invalidation:
  - [ ] Delete single track → measure refetch time
  - [ ] Compare old approach vs new
  - [ ] Target: 10x improvement

- [ ] Benchmark scan completion:
  - [ ] Scan 1000 files
  - [ ] Measure invalidation time
  - [ ] Verify UI doesn't freeze
  - [ ] Target: <200ms total invalidation

- [ ] Benchmark artwork cache:
  - [ ] Load 50 album artworks
  - [ ] Measure cache hit rate
  - [ ] Target: >90% hit rate

---

## Phase 8: Documentation

### 8.1 Code Documentation

- [ ] Add JSDoc comments to all new functions
- [ ] Add usage examples to mutation hooks
- [ ] Document invalidation patterns in helpers
- [ ] Add inline comments for complex logic

### 8.2 Update Project Docs

- [ ] Update `CLAUDE.md`:
  - [ ] Add cache invalidation section
  - [ ] Reference new mutation hooks
  - [ ] Add examples

- [ ] Update `ARCHITECTURE.md`:
  - [ ] Add invalidation strategy section
  - [ ] Link to detailed docs

- [ ] Update `TESTING.md`:
  - [ ] Add cache invalidation testing patterns
  - [ ] Add examples

### 8.3 Create Developer Guide

- [ ] Write migration guide for existing code
- [ ] Add troubleshooting section
- [ ] Document common patterns
- [ ] Add "What changed" summary

---

## Phase 9: Migration & Cleanup

### 9.1 Migrate Existing Code

- [ ] Search for manual `queryClient.invalidateQueries()` calls
- [ ] Replace with appropriate helper functions
- [ ] Search for manual `clearArtworkCache()` calls
- [ ] Replace with mutation hooks
- [ ] Update all `EditArtworkDialog` usages

### 9.2 Remove Legacy Code

- [ ] Remove old `refreshLibrary()` functions
- [ ] Remove manual invalidation in components
- [ ] Remove `onArtworkChanged` callbacks
- [ ] Clean up unused imports

### 9.3 Update Dependencies

- [ ] Verify `@tanstack/react-query` version (v5.x)
- [ ] Update TypeScript types if needed
- [ ] Run `yarn install` to ensure consistency

---

## Phase 10: Final Validation

### 10.1 Code Review Checklist

- [ ] All TypeScript types correct
- [ ] No `any` types used
- [ ] All errors handled properly
- [ ] Logging appropriate (not excessive)
- [ ] No console.log (use debug utils)
- [ ] Tests have good coverage (>80%)

### 10.2 User Acceptance Testing

- [ ] Import large library (1000+ albums)
  - [ ] Verify performance acceptable
  - [ ] Check for UI freezes

- [ ] Rapid mutations (spam playlist edits)
  - [ ] Verify optimistic updates work
  - [ ] Check for race conditions

- [ ] Network interruption during mutation
  - [ ] Verify error handling
  - [ ] Check rollback behavior

### 10.3 Release Preparation

- [ ] Run full test suite: `yarn test`
- [ ] Run linter: `yarn lint`
- [ ] Run type check: `yarn tsc --noEmit`
- [ ] Run Rust tests: `cargo test --all`
- [ ] Build desktop app: `yarn build:desktop`
- [ ] Test built app (not dev mode)

---

## Completion Criteria

### Must-Have (Phase 1-6)

- [x] All helper functions implemented
- [x] All mutation hooks created
- [x] Backend commands added
- [x] Event-driven invalidation working
- [x] Unit tests passing (>80% coverage)
- [x] Integration tests passing
- [x] E2E tests passing
- [x] Documentation complete

### Nice-to-Have (Phase 7-10)

- [ ] Performance benchmarks documented
- [ ] Developer guide published
- [ ] Migration complete (all legacy code removed)
- [ ] User acceptance testing passed

### Success Metrics

- [ ] Cache invalidation < 50ms for specific entities
- [ ] Cache invalidation < 200ms for scan completion
- [ ] Artwork cache hit rate > 90%
- [ ] No user-reported stale data issues
- [ ] Network traffic reduced by 80% vs broad invalidation
- [ ] UI remains responsive during all mutations

---

## Rollback Plan

If critical issues arise during rollout:

1. **Keep old code paths**:
   - [ ] Add feature flag: `USE_NEW_INVALIDATION`
   - [ ] Toggle between old/new approaches
   - [ ] Monitor for issues

2. **Gradual rollout**:
   - [ ] Week 1: Artwork mutations only
   - [ ] Week 2: Metadata mutations
   - [ ] Week 3: Event-driven invalidation
   - [ ] Week 4: Remove legacy code

3. **Emergency rollback**:
   - [ ] Revert to previous commit
   - [ ] Disable event listeners
   - [ ] Re-enable manual refresh buttons

---

## Resources

- **Design Doc**: `docs/CACHE_INVALIDATION_STRATEGY.md`
- **Quick Reference**: `docs/CACHE_INVALIDATION_QUICK_REFERENCE.md`
- **Flow Diagrams**: `docs/diagrams/cache-invalidation-flows.md`
- **TanStack Query Docs**: https://tanstack.com/query/v5/docs
- **Effective Query Keys**: https://tkdodo.eu/blog/effective-react-query-keys

---

**Estimated Timeline**:
- Phase 1-3: Week 1 (Foundation)
- Phase 4-5: Week 2 (Integration)
- Phase 6-7: Week 3 (Testing & Performance)
- Phase 8-10: Week 4 (Documentation & Cleanup)

**Total**: 4 weeks for full implementation

---

**Last Updated**: 2026-02-11
