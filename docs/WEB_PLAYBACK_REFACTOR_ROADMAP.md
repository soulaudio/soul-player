# Web Playback Refactor Roadmap

**Goal**: Extract web playback logic into a reusable library, reduce discrepancies between desktop and web, and prepare for a future web-player application.

**Status**: 🟡 In Progress

---

## Overview

### Current State
- Web playback code is tightly coupled to `applications/marketing`
- Manual event emission causes UI desync issues
- WASM adapter mixed with demo-specific logic
- Cannot reuse for a production web player

### Target State
- Web playback in a shared library (`libraries/soul-playback-web`)
- Automatic event emission (no manual calls in providers)
- Clear separation: Data layer (Backend) vs Playback layer (Commands)
- Reusable for both marketing demo and future web-player app

---

## Phase 1: Extract Web Playback Library 🔴 CRITICAL

**Objective**: Move WASM adapter and web audio code into a reusable library.

### Tasks

#### 1.1 Create Library Package Structure
- [ ] Create `libraries/soul-playback-web/` directory
- [ ] Add `package.json` with dependencies
- [ ] Add `tsconfig.json` with proper compilation settings
- [ ] Add `README.md` with usage documentation

**Files to create:**
```
libraries/soul-playback-web/
├── package.json
├── tsconfig.json
├── README.md
└── src/
    └── index.ts (barrel exports)
```

#### 1.2 Move Core Files
- [ ] Move `wasm-playback-adapter.ts` from marketing to library
- [ ] Move `audio-player.ts` from marketing to library
- [ ] Move `types.ts` from marketing to library
- [ ] Move `converters.ts` from marketing to library (if reusable)
- [ ] Move WASM bindings (`soul_playback.d.ts`, etc.)

**Source paths:**
```
applications/marketing/src/lib/demo/wasm-playback-adapter.ts
applications/marketing/src/lib/demo/audio-player.ts
applications/marketing/src/lib/demo/types.ts
applications/marketing/src/lib/demo/converters.ts
applications/marketing/src/wasm/soul-playback/*
```

**Target paths:**
```
libraries/soul-playback-web/src/wasm-adapter.ts
libraries/soul-playback-web/src/audio-player.ts
libraries/soul-playback-web/src/types.ts
libraries/soul-playback-web/src/converters.ts
libraries/soul-playback-web/src/wasm/*
```

#### 1.3 Update Imports
- [ ] Update `applications/marketing` imports to use new package
- [ ] Add `@soul-player/playback-web` to marketing dependencies
- [ ] Update `applications/shared` if needed
- [ ] Add yarn workspace configuration

**Import changes:**
```typescript
// Before
import { WasmPlaybackAdapter } from '@/lib/demo/wasm-playback-adapter'

// After
import { WasmPlaybackAdapter } from '@soul-player/playback-web'
```

---

## Phase 2: Automatic Event Emission 🟡 HIGH PRIORITY

**Objective**: Move event emission logic into the adapter itself, eliminating manual `emit()` calls in providers.

### Tasks

#### 2.1 Enhance WASM Adapter
- [ ] Add automatic `queueChange` emission on track changes
- [ ] Add automatic `queueChange` emission on next/previous
- [ ] Add automatic `queueChange` emission on skipToQueueIndex
- [ ] Add automatic `queueChange` emission on shuffle changes
- [ ] Remove redundant manual emits from providers

#### 2.2 Add State Synchronization
- [ ] Implement periodic state sync between WASM and UI (every 5s)
- [ ] Add queue consistency checks
- [ ] Log warnings when desync detected
- [ ] Auto-correct desync issues

#### 2.3 Improve Error Handling
- [ ] Add validation before all WASM operations
- [ ] Check queue length before skipToQueueIndex
- [ ] Check hasNext() before calling next()
- [ ] Add try-catch around all WASM calls
- [ ] Emit user-friendly error messages

**Files to modify:**
```
libraries/soul-playback-web/src/wasm-adapter.ts
applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx
```

---

## Phase 3: Abstract Web Playback Provider 🟢 MEDIUM PRIORITY

**Objective**: Create a reusable `WebPlaybackProvider` that works with any data source.

### Tasks

#### 3.1 Create Abstract Provider
- [ ] Create `applications/shared/src/providers/WebPlaybackProvider.tsx`
- [ ] Accept generic `DataStorage` interface
- [ ] Initialize WASM adapter
- [ ] Wire up all event listeners automatically
- [ ] Handle cleanup on unmount

#### 3.2 Refactor Demo Provider
- [ ] Update `DemoPlayerCommandsProvider` to use `WebPlaybackProvider`
- [ ] Pass `DemoStorage` as data source
- [ ] Remove manual event wiring
- [ ] Simplify command implementations

#### 3.3 Add Provider Documentation
- [ ] Document `WebPlaybackProvider` API
- [ ] Add usage examples
- [ ] Document data source interface requirements
- [ ] Add migration guide for future web-player

**Files to create/modify:**
```
applications/shared/src/providers/WebPlaybackProvider.tsx
applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx
docs/WEB_PLAYBACK_PROVIDER.md
```

---

## Phase 4: Testing & Validation 🟢 MEDIUM PRIORITY

**Objective**: Ensure refactored code works correctly and reduce regressions.

### Tasks

#### 4.1 Manual Testing
- [ ] Test play/pause on marketing demo
- [ ] Test queue navigation (click items)
- [ ] Test next/previous track
- [ ] Test shuffle enable/disable
- [ ] Test repeat mode changes
- [ ] Test seek functionality
- [ ] Test error cases (empty queue, invalid index)

#### 4.2 Automated Tests
- [ ] Add unit tests for WasmPlaybackAdapter
- [ ] Add unit tests for WebAudioPlayer
- [ ] Add integration tests for event emission
- [ ] Add tests for queue synchronization
- [ ] Add error handling tests

#### 4.3 Performance Testing
- [ ] Measure WASM ↔ JS boundary overhead
- [ ] Test with large queues (1000+ tracks)
- [ ] Check memory leaks during long sessions
- [ ] Verify audio latency

**Files to create:**
```
libraries/soul-playback-web/src/__tests__/wasm-adapter.test.ts
libraries/soul-playback-web/src/__tests__/audio-player.test.ts
applications/marketing/src/providers/__tests__/DemoPlayerCommandsProvider.test.tsx
```

---

## Phase 5: Documentation & Cleanup 🔵 LOW PRIORITY

**Objective**: Document the new architecture and clean up old code.

### Tasks

#### 5.1 Update Documentation
- [ ] Update `ARCHITECTURE.md` with web playback section
- [ ] Update `CLAUDE.md` with new patterns
- [ ] Create `WEB_PLAYBACK_GUIDE.md`
- [ ] Add JSDoc comments to all public APIs
- [ ] Create architecture diagrams

#### 5.2 Code Cleanup
- [ ] Remove old demo adapter code
- [ ] Remove unused bridge files
- [ ] Consolidate duplicate type definitions
- [ ] Remove debug console.logs (keep structured logging)
- [ ] Run prettier/eslint on all modified files

#### 5.3 WASM Build Optimization
- [ ] Review WASM bundle size
- [ ] Add code splitting if needed
- [ ] Optimize for faster loading
- [ ] Add WASM caching strategy

**Files to create/modify:**
```
docs/ARCHITECTURE.md
docs/WEB_PLAYBACK_GUIDE.md
CLAUDE.md
libraries/soul-playback-web/README.md
```

---

## Phase 6: Prepare for Web Player App 🔵 FUTURE

**Objective**: Set up foundation for a production web player application.

### Tasks

#### 6.1 Create Web Player Skeleton
- [ ] Create `applications/web-player/` directory
- [ ] Set up Vite/React app
- [ ] Add API provider interface
- [ ] Add authentication scaffolding
- [ ] Configure routing

#### 6.2 API Backend Provider
- [ ] Create `ApiBackendProvider.tsx`
- [ ] Implement `BackendInterface` using REST/GraphQL
- [ ] Add authentication headers
- [ ] Handle API errors gracefully
- [ ] Add loading states

#### 6.3 Web Player Provider
- [ ] Create `WebPlayerCommandsProvider.tsx`
- [ ] Use `WebPlaybackProvider` from shared
- [ ] Connect to API backend
- [ ] Handle streaming URLs
- [ ] Add buffering indicators

**Files to create:**
```
applications/web-player/
├── package.json
├── vite.config.ts
└── src/
    ├── main.tsx
    └── providers/
        ├── ApiBackendProvider.tsx
        └── WebPlayerCommandsProvider.tsx
```

---

## Success Metrics

### Phase 1 Success
- ✅ Marketing demo still works
- ✅ No import errors
- ✅ WASM builds correctly
- ✅ All TypeScript checks pass

### Phase 2 Success
- ✅ No more "Queue is empty" errors
- ✅ Queue UI updates automatically on track changes
- ✅ Shuffle/repeat work without manual events
- ✅ No manual `emit()` calls in DemoPlayerCommandsProvider

### Phase 3 Success
- ✅ WebPlaybackProvider is reusable
- ✅ Demo provider is simplified (<200 lines)
- ✅ Clear separation of concerns
- ✅ Ready for web-player implementation

### Overall Success
- ✅ Marketing demo has <5% discrepancy with desktop
- ✅ Web playback code is in a reusable library
- ✅ Easy to create web-player app in future
- ✅ No regressions in existing functionality

---

## Risk Mitigation

### Risk: Breaking Marketing Demo
**Mitigation**:
- Test after each phase
- Keep git branches for rollback
- Run TypeScript checks continuously
- Manual testing checklist

### Risk: Performance Degradation
**Mitigation**:
- Profile before/after refactor
- Monitor WASM bundle size
- Test with large playlists
- Measure event emission overhead

### Risk: WASM Compatibility Issues
**Mitigation**:
- Keep WASM bindings versioned
- Test on multiple browsers
- Fallback to mock player if WASM fails
- Clear error messages for users

---

## Timeline Estimate

| Phase | Effort | Duration |
|-------|--------|----------|
| Phase 1: Extract Library | High | 2-4 hours |
| Phase 2: Auto Events | Medium | 1-2 hours |
| Phase 3: Abstract Provider | Medium | 2-3 hours |
| Phase 4: Testing | Medium | 1-2 hours |
| Phase 5: Documentation | Low | 1-2 hours |
| Phase 6: Web Player Prep | Low | Future |

**Total Estimated Time**: 7-13 hours (AI-assisted)

---

## Dependencies

### Required Before Starting
- ✅ Yarn workspaces configured
- ✅ TypeScript build working
- ✅ WASM compilation working

### Blocking Issues
- None currently

---

## Notes

- Keep desktop app untouched (only affects web playback)
- Maintain backwards compatibility with existing demo
- Focus on robustness over features
- Document all breaking changes

---

**Last Updated**: 2026-01-23
**Status**: Ready to execute
