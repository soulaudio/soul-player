# Technical Debt Report

**Generated:** 2026-02-15
**Status:** Post-stash merge cleanup

---

## Summary

- **Production Code:** ✅ Healthy - compiles, passes clippy, critical tests passing
- **Test Suite:** ⚠️ Needs attention - 322 ignored tests, 185 API migration needed
- **Architecture:** ⚠️ Phase 5 refactoring planned but not started
- **Code Markers:** 146 TODO/FIXME comments across codebase

---

## Critical Issues (High Priority)

### 1. Test API Migration (185 occurrences)

**Impact:** Medium - Test coverage gaps
**Effort:** Medium - Mechanical refactoring

**Issue:**
- 185 test calls still use removed `set_audio_source()` API
- Need conversion to `activate_source(source, track)` API
- Requires extracting inline track creation to variables

**Files affected:**
- `playback_manager_e2e_test.rs` (49 occurrences)
- `stress_test.rs` (48 occurrences)
- `memory_and_edge_case_test.rs` (17 occurrences)
- 10+ other test files

**Example fix needed:**
```rust
// OLD (broken):
manager.add_to_queue_end(create_track("1", "Track", "Artist", 180));
manager.set_audio_source(Box::new(MockAudioSource::new(...)));

// NEW (working):
let track = create_track("1", "Track", "Artist", 180);
manager.add_to_queue_end(track.clone());
manager.activate_source(Box::new(MockAudioSource::new(...)), track);
```

**Progress:** 71/256 fixed (28% complete)

---

### 2. Ignored Tests (322 total)

**Impact:** High - Reduces confidence in changes
**Effort:** High - Requires architectural fixes

**Categories:**

1. **Timing-dependent tests (23 occurrences)**
   - Tests fail in release mode due to timing variations
   - Examples: Crossfade trigger timing, start fade jitter
   - Root cause: Tests assume synchronous behavior in async system

2. **Async state transition tests (6 occurrences)**
   - Tests expect immediate state changes after `play()`/`pause()`
   - Actual behavior: State changes deferred until audio processing cycle
   - Needs: Either mock audio processing or async test helpers

3. **Hardware-dependent tests**
   - Tests require real audio devices
   - Can't run in CI environments
   - Should be feature-gated or use mock devices

**Recommendation:**
- Add `#[cfg(feature = "timing-tests")]` for timing-sensitive tests
- Create async test helpers that properly wait for state changes
- Implement mock audio device for CI testing

---

### 3. Phase 5 Architecture Migration (Incomplete)

**Impact:** Medium - Deprecated APIs still in use
**Effort:** High - Major refactoring

**Background:**
The codebase references a "Phase 5" refactoring to use `SourceState` for source management, but it's not started.

**Deprecated APIs:**
```rust
#[deprecated(note = "Use SourceState for source management in Phase 5")]
pub fn set_next_source(&mut self, source, track) { ... }

#[deprecated(note = "Use SourceState::is_transitioning() in Phase 5")]
pub fn has_next_source(&self) -> bool { ... }
```

**TODOs in `manager.rs`:**
- 20+ references to "Phase 5" refactoring
- Affects: Crossfade, prefetching, event deduplication, progress tracking

**Risk:** Using deprecated APIs creates future breaking changes

**Recommendation:**
- Document Phase 5 goals and timeline
- Consider if this refactoring is still planned or should be deprioritized
- If keeping deprecated APIs, ensure they're fully functional

---

## Medium Priority Issues

### 4. Disabled Test File

**File:** `applications/desktop/src-tauri/tests/lazy_queue_loading_test.rs`
**Status:** Disabled with `#![cfg(all())]`
**Reason:** API changes from stash merge broke tests (6 test functions)

**Fix needed:** Update tests for PlaybackManager API changes

---

### 5. Missing Features

**From TODO markers in TypeScript:**

1. **Toast notification system** (TauriPlayerCommandsProvider.tsx)
   - Playback errors currently silent
   - Need user-facing error notifications

2. **Queue synchronization** (usePlaybackEvents.ts)
   - QueueChanged event received but queue not fetched from backend
   - Can cause UI/backend queue drift

3. **Album navigation** (HomePage.tsx)
   - Click on album does nothing
   - Should navigate to album detail or start playback

4. **Convolution IR visualization** (ConvolutionEditor.tsx)
   - Currently shows placeholder waveform
   - Should parse actual impulse response file

---

## Low Priority Issues

### 6. Code Quality Markers

**146 TODO/FIXME comments** across codebase:
- 90+ in Rust files
- 50+ in TypeScript files
- Most are feature requests or optimization notes
- None blocking production functionality

**Recommendation:** Triage and prioritize in issue tracker

---

### 7. Line Ending Warnings

```
warning: in the working copy of '.../*.rs', LF will be replaced by CRLF
```

**Issue:** Mixed line endings on Windows
**Impact:** Low - cosmetic only
**Fix:** Configure `.gitattributes` properly

---

## Positive Findings

### What's Working Well ✅

1. **Production code quality**
   - All lib/bin code compiles cleanly
   - Passes `clippy -D warnings`
   - No unsafe code issues

2. **Critical functionality tested**
   - 11/11 playback persistence E2E tests passing
   - 347/347 library unit tests passing
   - Core playback functionality verified

3. **Recent improvements**
   - Fixed N+1 query problem (50x faster)
   - Eliminated dual state updates
   - Added Zod runtime validation
   - Unified duplicate structs

4. **Documentation**
   - CLAUDE.md comprehensive and up-to-date
   - Architecture documented
   - Clear commit messages with Co-Authored-By

---

## Recommendations

### Immediate (This Week)

1. ✅ **DONE:** Fix clippy lints
2. ✅ **DONE:** Update 71 test files for activate_source API
3. ⏭️ **NEXT:** Fix remaining 185 test API calls
4. ⏭️ **NEXT:** Re-enable and fix lazy_queue_loading_test.rs

### Short Term (This Month)

1. Triage ignored tests - categorize by fix difficulty
2. Implement async test helpers for state transition tests
3. Add feature gates for timing-dependent tests
4. Document or deprioritize Phase 5 refactoring

### Long Term (This Quarter)

1. Complete test suite restoration (all tests passing or properly ignored)
2. Implement toast notification system
3. Add queue synchronization for backend/frontend parity
4. Decide on Phase 5: commit to it or remove deprecated APIs

---

## Metrics

| Metric | Count | Status |
|--------|-------|--------|
| Production compilation | ✅ | Passing |
| Clippy warnings | 0 | ✅ Clean |
| Critical E2E tests | 11/11 | ✅ Passing |
| Library unit tests | 347/347 | ✅ Passing |
| Ignored tests | 322 | ⚠️ High |
| Test API migration | 71/256 | ⚠️ 28% |
| TODO/FIXME markers | 146 | ⚠️ Moderate |
| Deprecated APIs | 3 | ⚠️ Low |

---

## Risk Assessment

**Overall Risk:** 🟡 **Medium**

- Production code is solid and functional
- Test suite gaps create regression risk
- Deprecated APIs need migration path
- No critical blockers for current development

**Recommendation:** Safe to continue development, prioritize test fixes incrementally.
