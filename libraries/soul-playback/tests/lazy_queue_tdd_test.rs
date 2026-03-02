//! TDD tests for the lazy_queue module
//!
//! Bug found: `current_window_indices()` subtracts `start` from `end` (both usize)
//! without checking that `start <= end`.  When `window_start > window_end.min(total)`
//! the subtraction wraps/panics in debug mode with:
//!   "attempt to subtract with overflow"
//!
//! Repro: create a state with window_start = 20, window_end = 5, total = 100
//! (can happen if the consumer rewinds window_start past window_end, or if
//! window_end is initialised to 0 before the first batch loads but window_start
//! was already advanced).
//!
//! Fix: guard the subtraction — use `end.saturating_sub(start)` (returns 0 when
//! start > end so an empty vec is returned, which is the correct no-op behaviour).

use soul_playback::lazy_queue::{
    LazyQueueState, QueueContext, DEFAULT_WINDOW_SIZE, LOAD_THRESHOLD,
};

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn all_tracks_ctx(total: usize) -> QueueContext {
    QueueContext::AllTracks {
        user_id: 1,
        total_count: total,
    }
}

fn album_ctx(total: usize) -> QueueContext {
    QueueContext::Album {
        album_id: 42,
        total_count: total,
    }
}

fn playlist_ctx(total: usize) -> QueueContext {
    QueueContext::Playlist {
        playlist_id: 7,
        owner_id: 1,
        total_count: total,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug: current_window_indices() panics when window_start > window_end
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn current_window_indices_does_not_panic_when_window_start_exceeds_window_end() {
    let mut state = LazyQueueState::new(all_tracks_ctx(100), 0);
    // Force an inverted window (start > end) — can arise from incorrect
    // window management in the consuming code or after a partial state load.
    state.window_start = 20;
    state.window_end = 5; // end < start

    // Must not panic with "attempt to subtract with overflow"
    let indices = state.current_window_indices();
    // When the window is empty/inverted, returning an empty slice is correct.
    assert!(
        indices.is_empty(),
        "Expected empty indices for inverted window (start={}, end={}), got {:?}",
        state.window_start,
        state.window_end,
        indices
    );
}

#[test]
fn current_window_indices_does_not_panic_when_window_start_exceeds_total() {
    let mut state = LazyQueueState::new(all_tracks_ctx(10), 0);
    state.window_start = 15; // beyond total
    state.window_end = 20;

    // end = window_end.min(total) = 10; start = 15 > end
    // Must not panic
    let indices = state.current_window_indices();
    assert!(
        indices.is_empty(),
        "Expected empty indices when window_start > total"
    );
}

#[test]
fn current_window_indices_shuffled_does_not_panic_when_window_start_exceeds_window_end() {
    let mut state = LazyQueueState::new(all_tracks_ctx(100), 0);
    state.window_start = 50;
    state.window_end = 10; // inverted
    state.shuffle_seed = Some(12345);

    // Must not panic
    let indices = state.current_window_indices();
    assert!(
        indices.is_empty(),
        "Expected empty shuffled indices for inverted window"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: empty collection
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn next_batch_range_when_collection_is_empty_returns_zero_limit() {
    let state = LazyQueueState::new(all_tracks_ctx(0), 0);
    let (offset, limit) = state.next_batch_range();
    assert_eq!(offset, 0, "offset should be 0 for empty collection");
    assert_eq!(
        limit, 0,
        "limit should be 0 for empty collection — nothing to load"
    );
}

#[test]
fn current_window_indices_on_empty_collection_is_empty() {
    let state = LazyQueueState::new(all_tracks_ctx(0), 0);
    let indices = state.current_window_indices();
    assert!(
        indices.is_empty(),
        "Empty collection must give empty window"
    );
}

#[test]
fn should_load_next_batch_on_empty_collection_returns_true() {
    // With an empty collection (window_end = 0), remaining = 0 < LOAD_THRESHOLD
    let state = LazyQueueState::new(all_tracks_ctx(0), 0);
    // Position 0, window_end 0 → remaining = 0 < 10 → should trigger load attempt
    assert!(
        state.should_load_next_batch(0),
        "Should indicate a load attempt is needed even for empty collection \
         (the caller decides if there's anything to fetch)"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: collection smaller than one batch window
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn next_batch_range_when_window_end_already_covers_all_tracks_returns_zero_limit() {
    let mut state = LazyQueueState::new(all_tracks_ctx(30), 0);
    state.window_end = 30; // already loaded all 30 tracks

    let (offset, limit) = state.next_batch_range();
    assert_eq!(offset, 30);
    assert_eq!(limit, 0, "Nothing left to load when window_end == total");
}

#[test]
fn next_batch_range_clamps_to_remaining_tracks() {
    let mut state = LazyQueueState::new(all_tracks_ctx(60), 0);
    state.window_end = 40; // 20 tracks remain, less than DEFAULT_WINDOW_SIZE (50)

    let (offset, limit) = state.next_batch_range();
    assert_eq!(offset, 40);
    assert_eq!(
        limit, 20,
        "Limit must be clamped to remaining tracks, not DEFAULT_WINDOW_SIZE"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: extend_window does not exceed total
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn extend_window_large_loaded_count_does_not_overflow() {
    let mut state = LazyQueueState::new(all_tracks_ctx(100), 0);
    state.window_end = 90;

    // Caller passes loaded_count = 50 even though only 10 remain
    state.extend_window(50);

    // window_end should saturate, not overflow
    assert_eq!(
        state.window_end, 140,
        "extend_window itself does not clamp to total — caller is responsible for clamping. \
         But it must not overflow (saturating_add is used)."
    );

    // The consumer of current_window_indices will see total clamped
    let indices = state.current_window_indices();
    assert_eq!(
        indices.len(),
        100,
        "current_window_indices must clamp to total even if window_end exceeds it"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: should_load_next_batch at exact LOAD_THRESHOLD boundary
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn should_load_next_batch_at_exact_threshold_is_false() {
    let mut state = LazyQueueState::new(all_tracks_ctx(1000), 0);
    state.window_end = 50;

    // remaining = 50 - 40 = 10 == LOAD_THRESHOLD → should NOT trigger yet
    assert!(
        !state.should_load_next_batch(40),
        "Should not load when exactly LOAD_THRESHOLD ({LOAD_THRESHOLD}) tracks remain"
    );
}

#[test]
fn should_load_next_batch_one_below_threshold_is_true() {
    let mut state = LazyQueueState::new(all_tracks_ctx(1000), 0);
    state.window_end = 50;

    // remaining = 50 - 41 = 9 < LOAD_THRESHOLD → should trigger
    assert!(
        state.should_load_next_batch(41),
        "Should load when fewer than LOAD_THRESHOLD ({LOAD_THRESHOLD}) tracks remain"
    );
}

#[test]
fn should_load_next_batch_current_position_beyond_window_end_does_not_panic() {
    let mut state = LazyQueueState::new(all_tracks_ctx(1000), 0);
    state.window_end = 50;

    // current_queue_position > window_end — saturating_sub gives 0 < 10 → triggers
    let result = state.should_load_next_batch(999);
    assert!(
        result,
        "Position beyond window_end must not panic and should trigger load"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: QueueContext accessors
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn search_context_total_count_is_zero() {
    let ctx = QueueContext::Search {
        query: "hello".to_string(),
    };
    assert_eq!(ctx.total_count(), 0);
}

#[test]
fn search_context_does_not_support_lazy_loading() {
    let ctx = QueueContext::Search {
        query: "anything".to_string(),
    };
    assert!(!ctx.supports_lazy_loading());
}

#[test]
fn album_context_supports_lazy_loading() {
    assert!(album_ctx(100).supports_lazy_loading());
}

#[test]
fn playlist_context_supports_lazy_loading() {
    assert!(playlist_ctx(500).supports_lazy_loading());
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: constants are sane
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn constants_are_valid() {
    assert!(LOAD_THRESHOLD > 0, "LOAD_THRESHOLD must be positive");
    assert!(
        DEFAULT_WINDOW_SIZE > LOAD_THRESHOLD,
        "Window size must be larger than threshold to prevent infinite reload loops"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: single-track collection
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn single_track_collection_sequential_window() {
    let mut state = LazyQueueState::new(all_tracks_ctx(1), 0);
    state.window_start = 0;
    state.window_end = 1;

    let indices = state.current_window_indices();
    assert_eq!(indices, vec![0], "Single-track window must return [0]");
}

#[test]
fn single_track_collection_next_batch_range_after_load() {
    let mut state = LazyQueueState::new(all_tracks_ctx(1), 0);
    state.window_end = 1;

    let (offset, limit) = state.next_batch_range();
    assert_eq!(
        limit, 0,
        "No more tracks to load after loading the only track"
    );
    let _ = offset; // offset = 1 is fine
}
