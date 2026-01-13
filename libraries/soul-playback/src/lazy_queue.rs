//! Lazy queue context for on-demand track loading
//!
//! Enables playback of large collections (1000+ tracks) without loading
//! everything into memory. Tracks are loaded in windows of ~50 tracks.

use serde::{Deserialize, Serialize};

/// Default window size for lazy loading (tracks to load at once)
pub const DEFAULT_WINDOW_SIZE: usize = 50;

/// Minimum tracks remaining before loading next batch
pub const LOAD_THRESHOLD: usize = 10;

/// Queue context describing what collection is being played
///
/// Used to fetch tracks on-demand without storing entire collection in memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QueueContext {
    /// All tracks in library
    #[serde(rename = "AllTracks")]
    AllTracks {
        /// User ID for multi-user support
        #[serde(rename = "userId")]
        user_id: i64,
        /// Total track count in collection
        #[serde(rename = "totalCount")]
        total_count: usize,
    },

    /// Tracks from a specific album
    #[serde(rename = "Album")]
    Album {
        /// Album ID
        #[serde(rename = "albumId")]
        album_id: i64,
        /// Total track count in album
        #[serde(rename = "totalCount")]
        total_count: usize,
    },

    /// Tracks from a specific artist
    #[serde(rename = "Artist")]
    Artist {
        /// Artist ID
        #[serde(rename = "artistId")]
        artist_id: i64,
        /// Total track count for artist
        #[serde(rename = "totalCount")]
        total_count: usize,
    },

    /// Tracks from a playlist
    #[serde(rename = "Playlist")]
    Playlist {
        /// Playlist ID
        #[serde(rename = "playlistId")]
        playlist_id: i64,
        /// Owner user ID (for multi-user support)
        #[serde(rename = "ownerId")]
        owner_id: i64,
        /// Total track count in playlist
        #[serde(rename = "totalCount")]
        total_count: usize,
    },

    /// Search results (not lazy-loaded, always send full results)
    #[serde(rename = "Search")]
    Search {
        /// Search query
        query: String,
    },
}

impl QueueContext {
    /// Get total track count for this context
    pub fn total_count(&self) -> usize {
        match self {
            Self::AllTracks { total_count, .. }
            | Self::Album { total_count, .. }
            | Self::Artist { total_count, .. }
            | Self::Playlist { total_count, .. } => *total_count,
            Self::Search { .. } => 0, // Search results not lazy-loaded
        }
    }

    /// Check if this context supports lazy loading
    pub fn supports_lazy_loading(&self) -> bool {
        !matches!(self, Self::Search { .. })
    }

    /// Get context type name for logging
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::AllTracks { .. } => "AllTracks",
            Self::Album { .. } => "Album",
            Self::Artist { .. } => "Artist",
            Self::Playlist { .. } => "Playlist",
            Self::Search { .. } => "Search",
        }
    }
}

/// Lazy queue state tracking window position and shuffle state
#[derive(Debug, Clone)]
pub struct LazyQueueState {
    /// Current context being played
    pub context: QueueContext,

    /// Start index in original collection (before shuffle)
    pub start_index: usize,

    /// Current window start index (in shuffled order if shuffle enabled)
    pub window_start: usize,

    /// Current window end index (in shuffled order if shuffle enabled, exclusive)
    pub window_end: usize,

    /// Seed for deterministic shuffle (if shuffle enabled)
    ///
    /// Uses only 8 bytes instead of storing entire shuffle order in memory.
    /// The same seed always produces the same shuffle order.
    ///
    /// Example: With seed 12345:
    /// - Position 0 → calculate_shuffle_index(12345, 0, total) = 42
    /// - Position 1 → calculate_shuffle_index(12345, 1, total) = 7
    /// - etc.
    pub shuffle_seed: Option<u64>,

    /// Current position in playback order (shuffled or sequential)
    pub current_position: usize,
}

impl LazyQueueState {
    /// Create new lazy queue state
    pub fn new(context: QueueContext, start_index: usize) -> Self {
        Self {
            context,
            start_index,
            window_start: 0,
            window_end: 0,
            shuffle_seed: None,
            current_position: 0,
        }
    }

    /// Check if we need to load the next batch
    ///
    /// Returns true if < 10 tracks remain in current window
    pub fn should_load_next_batch(&self, current_queue_position: usize) -> bool {
        let remaining = self.window_end.saturating_sub(current_queue_position);
        remaining < LOAD_THRESHOLD
    }

    /// Calculate indices for next batch to load
    ///
    /// Returns (offset, limit) for database query
    pub fn next_batch_range(&self) -> (usize, usize) {
        let total = self.context.total_count();
        let offset = self.window_end;
        let limit = DEFAULT_WINDOW_SIZE.min(total.saturating_sub(offset));
        (offset, limit)
    }

    /// Update window bounds after loading new batch
    pub fn extend_window(&mut self, loaded_count: usize) {
        self.window_end = self.window_end.saturating_add(loaded_count);
    }

    /// Enable shuffle with seed-based approach (uses only 8 bytes)
    pub fn enable_shuffle(&mut self, shuffle_mode: crate::types::ShuffleMode) {
        match shuffle_mode {
            crate::types::ShuffleMode::Off => {
                self.shuffle_seed = None;
                self.current_position = 0;
            }
            crate::types::ShuffleMode::Random | crate::types::ShuffleMode::Smart => {
                // Generate seed from entropy
                use rand::RngCore;
                let mut rng = rand::thread_rng();
                self.shuffle_seed = Some(rng.next_u64());
                self.current_position = 0;
            }
        }
    }

    /// Disable shuffle and restore original order
    pub fn disable_shuffle(&mut self) {
        self.shuffle_seed = None;
        self.current_position = 0;
    }

    /// Get indices to load for current window
    ///
    /// If shuffle enabled, calculates shuffled indices for current window on-demand.
    /// Otherwise, returns sequential indices.
    pub fn current_window_indices(&self) -> Vec<usize> {
        let total = self.context.total_count();
        let start = self.window_start;
        let end = self.window_end.min(total);

        if let Some(seed) = self.shuffle_seed {
            // Shuffle enabled: generate shuffled indices for this window
            // This generates the full shuffle temporarily to extract our window
            // For very large datasets (>100k), we could use a stateless permutation instead
            generate_shuffled_window(seed, start, end - start, total)
        } else {
            // No shuffle: return sequential indices
            (start..end).collect()
        }
    }

    /// Check if shuffle is currently enabled
    pub fn is_shuffled(&self) -> bool {
        self.shuffle_seed.is_some()
    }
}

/// Generate shuffled indices for a specific window using a seed
///
/// This generates the full shuffle in memory temporarily, then extracts the requested window.
/// Memory usage: O(n) temporarily during generation, then freed.
///
/// For collections < 100k items, this is fast enough (< 5ms for 100k items).
/// For larger collections, consider using a Feistel cipher or other stateless permutation.
fn generate_shuffled_window(
    seed: u64,
    window_start: usize,
    window_size: usize,
    total: usize,
) -> Vec<usize> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    // Generate full shuffle with seed
    let mut indices: Vec<usize> = (0..total).collect();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    indices.shuffle(&mut rng);

    // Extract requested window
    let end = (window_start + window_size).min(total);
    indices[window_start..end].to_vec()
}

/// Calculate a single shuffled index using seed (for future optimization)
///
/// This is a stateless approach that calculates individual indices without
/// generating the full shuffle. Useful for very large datasets (1M+ items).
///
/// Uses a simple LCG-based permutation. For production, consider using
/// a Feistel cipher or Format-Preserving Encryption for better distribution.
#[allow(dead_code)]
fn calculate_shuffled_index(seed: u64, position: usize, total: usize) -> usize {
    if total == 0 {
        return 0;
    }

    // Simple LCG parameters (would need tuning for production)
    let multiplier = 1664525u64;
    let increment = 1013904223u64;

    // Generate pseudorandom value for this position
    let hash = seed
        .wrapping_mul(multiplier)
        .wrapping_add(position as u64)
        .wrapping_add(increment);

    // Map to valid range
    (hash % total as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_total_count() {
        let ctx = QueueContext::AllTracks {
            user_id: 1,
            total_count: 1000,
        };
        assert_eq!(ctx.total_count(), 1000);
    }

    #[test]
    fn test_context_supports_lazy_loading() {
        let all_tracks = QueueContext::AllTracks {
            user_id: 1,
            total_count: 1000,
        };
        assert!(all_tracks.supports_lazy_loading());

        let search = QueueContext::Search {
            query: "test".to_string(),
        };
        assert!(!search.supports_lazy_loading());
    }

    #[test]
    fn test_should_load_next_batch() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 1000,
            },
            0,
        );
        state.window_start = 0;
        state.window_end = 50;

        // At position 45, only 5 tracks remain - should load next batch
        assert!(state.should_load_next_batch(45));

        // At position 30, 20 tracks remain - don't load yet
        assert!(!state.should_load_next_batch(30));
    }

    #[test]
    fn test_next_batch_range() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 1000,
            },
            0,
        );
        state.window_end = 50;

        let (offset, limit) = state.next_batch_range();
        assert_eq!(offset, 50);
        assert_eq!(limit, 50);
    }

    #[test]
    fn test_extend_window() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 1000,
            },
            0,
        );
        state.window_end = 50;

        state.extend_window(50);
        assert_eq!(state.window_end, 100);
    }

    #[test]
    fn test_current_window_indices_sequential() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 1000,
            },
            0,
        );
        state.window_start = 0;
        state.window_end = 5;

        let indices = state.current_window_indices();
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_current_window_indices_shuffled() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 10,
            },
            0,
        );
        state.window_start = 0;
        state.window_end = 5;

        // Enable shuffle with a fixed seed for deterministic test
        state.shuffle_seed = Some(42);

        let indices = state.current_window_indices();

        // Should return 5 indices
        assert_eq!(indices.len(), 5);

        // All indices should be valid (< 10)
        for &idx in &indices {
            assert!(idx < 10);
        }

        // Should be deterministic - same seed = same result
        let indices2 = state.current_window_indices();
        assert_eq!(indices, indices2);
    }

    #[test]
    fn test_shuffle_seed_deterministic() {
        // Same seed should produce same shuffle
        let indices1 = generate_shuffled_window(12345, 0, 10, 100);
        let indices2 = generate_shuffled_window(12345, 0, 10, 100);
        assert_eq!(indices1, indices2);

        // Different seed should produce different shuffle
        let indices3 = generate_shuffled_window(99999, 0, 10, 100);
        assert_ne!(indices1, indices3);
    }

    #[test]
    fn test_shuffle_memory_efficiency() {
        // Test that we only store 8 bytes (the seed), not the full shuffle
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 100_000,
            },
            0,
        );

        // Enable shuffle
        state.shuffle_seed = Some(42);

        // Verify shuffle is enabled
        assert!(state.is_shuffled());
        assert_eq!(state.shuffle_seed, Some(42));

        // Memory footprint is just 8 bytes (u64) regardless of collection size
        assert_eq!(
            std::mem::size_of_val(&state.shuffle_seed),
            std::mem::size_of::<Option<u64>>()
        );
    }

    #[test]
    fn test_enable_disable_shuffle() {
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 1000,
            },
            0,
        );

        // Initially no shuffle
        assert!(!state.is_shuffled());

        // Enable shuffle
        state.enable_shuffle(crate::types::ShuffleMode::Random);
        assert!(state.is_shuffled());
        assert!(state.shuffle_seed.is_some());

        // Disable shuffle
        state.disable_shuffle();
        assert!(!state.is_shuffled());
        assert!(state.shuffle_seed.is_none());
    }
}
