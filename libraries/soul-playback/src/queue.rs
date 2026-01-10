//! Two-tier queue system
//!
//! Implements Spotify-style queue with:
//! - Explicit queue: User-added tracks that play next
//! - Source queue: Tracks from playlist/album

use crate::types::QueueTrack;

/// Two-tier queue for playback
///
/// Structure:
/// ```text
/// Currently Playing: Track A
/// ─────────────────────────────
/// Explicit Queue (play next):
///   - Track B (user added)
///   - Track C (user added)
/// ─────────────────────────────
/// Source Queue (from playlist/album):
///   - Track D
///   - Track E
///   - Track F
/// ```
#[derive(Debug, Clone)]
pub struct Queue {
    /// Tracks explicitly added by user (play next)
    explicit: Vec<QueueTrack>,

    /// Tracks from source (playlist/album)
    source: Vec<QueueTrack>,

    /// Current position in source queue (for non-destructive navigation)
    source_index: usize,

    /// Original order before shuffle (for restoring)
    original_source: Vec<QueueTrack>,

    /// Whether source queue is currently shuffled
    is_shuffled: bool,
}

impl Queue {
    /// Create new empty queue
    pub fn new() -> Self {
        Self {
            explicit: Vec::new(),
            source: Vec::new(),
            source_index: 0,
            original_source: Vec::new(),
            is_shuffled: false,
        }
    }

    /// Add track to explicit queue (next to play)
    ///
    /// Track will play after current track, before source queue
    pub fn add_next(&mut self, track: QueueTrack) {
        self.explicit.push(track);
    }

    /// Add track to end of explicit queue
    pub fn add_to_end(&mut self, track: QueueTrack) {
        self.explicit.push(track);
    }

    /// Add tracks from source (playlist/album)
    ///
    /// Replaces current source queue
    pub fn set_source(&mut self, tracks: Vec<QueueTrack>) {
        self.source.clone_from(&tracks);
        self.original_source = tracks;
        self.source_index = 0;
        self.is_shuffled = false;
    }

    /// Append tracks to source queue
    pub fn append_to_source(&mut self, tracks: Vec<QueueTrack>) {
        self.source.extend(tracks.clone());
        self.original_source.extend(tracks);
    }

    /// Remove track from queue by index
    ///
    /// Returns the removed track if successful
    pub fn remove(&mut self, index: usize) -> Option<QueueTrack> {
        let total = self.len();
        if index >= total {
            return None;
        }

        if index < self.explicit.len() {
            // Remove from explicit queue
            Some(self.explicit.remove(index))
        } else {
            // Remove from source queue
            let source_index = index - self.explicit.len();
            let track = self.source.remove(source_index);

            // Also remove from original (to maintain consistency)
            if let Some(pos) = self.original_source.iter().position(|t| t.id == track.id) {
                self.original_source.remove(pos);
            }

            Some(track)
        }
    }

    /// Reorder track in queue
    ///
    /// Moves track from `from_index` to `to_index`
    pub fn reorder(&mut self, from_index: usize, to_index: usize) -> Result<(), String> {
        let total = self.len();
        if from_index >= total || to_index >= total {
            return Err("Index out of bounds".to_string());
        }

        if from_index == to_index {
            return Ok(());
        }

        // For simplicity, only allow reordering within same tier
        let explicit_len = self.explicit.len();

        if from_index < explicit_len && to_index < explicit_len {
            // Both in explicit queue
            let track = self.explicit.remove(from_index);
            self.explicit.insert(to_index, track);
            Ok(())
        } else if from_index >= explicit_len && to_index >= explicit_len {
            // Both in source queue
            let from_source = from_index - explicit_len;
            let to_source = to_index - explicit_len;
            let track = self.source.remove(from_source);
            self.source.insert(to_source, track);
            Ok(())
        } else {
            Err("Cannot move tracks between explicit and source queues".to_string())
        }
    }

    /// Clear entire queue
    pub fn clear(&mut self) {
        self.explicit.clear();
        self.source.clear();
        self.source_index = 0;
        self.original_source.clear();
        self.is_shuffled = false;
    }

    /// Clear only explicit queue
    #[allow(dead_code)]
    pub fn clear_explicit(&mut self) {
        self.explicit.clear();
    }

    /// Clear only source queue
    #[allow(dead_code)]
    pub fn clear_source(&mut self) {
        self.source.clear();
        self.source_index = 0;
        self.original_source.clear();
        self.is_shuffled = false;
    }

    /// Get next track to play
    ///
    /// Prioritizes explicit queue, then source queue.
    /// Uses index-based navigation for source queue (non-destructive).
    pub fn pop_next(&mut self) -> Option<QueueTrack> {
        // Explicit queue still uses remove (destructive by design)
        if !self.explicit.is_empty() {
            return Some(self.explicit.remove(0));
        }

        // Source queue uses index navigation (non-destructive)
        if self.source_index < self.source.len() {
            let track = self.source[self.source_index].clone();
            self.source_index += 1;
            Some(track)
        } else {
            None
        }
    }

    /// Peek at next track without removing
    #[allow(dead_code)]
    pub fn peek_next(&self) -> Option<&QueueTrack> {
        if self.explicit.is_empty() {
            self.source.first()
        } else {
            self.explicit.first()
        }
    }

    /// Get all tracks in queue order from current position
    ///
    /// Returns explicit queue followed by remaining source queue tracks.
    /// Source queue starts from current position (source_index), not from beginning.
    pub fn get_all(&self) -> Vec<&QueueTrack> {
        // Explicit queue always shown fully
        let explicit_iter = self.explicit.iter();

        // Source queue shows only tracks from current position onward
        let remaining_source = if self.source_index < self.source.len() {
            &self.source[self.source_index..]
        } else {
            &self.source[0..0] // Empty slice if we've played through all
        };

        explicit_iter.chain(remaining_source.iter()).collect()
    }

    /// Get track at index
    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> Option<&QueueTrack> {
        let explicit_len = self.explicit.len();
        if index < explicit_len {
            self.explicit.get(index)
        } else {
            self.source.get(index - explicit_len)
        }
    }

    /// Total number of tracks in queue
    pub fn len(&self) -> usize {
        self.explicit.len() + self.source.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.explicit.is_empty() && self.source.is_empty()
    }

    /// Check if source queue is shuffled
    #[allow(dead_code)]
    pub fn is_shuffled(&self) -> bool {
        self.is_shuffled
    }

    /// Get reference to source queue (for shuffling)
    pub(crate) fn source_mut(&mut self) -> &mut Vec<QueueTrack> {
        &mut self.source
    }

    /// Mark source queue as shuffled
    pub(crate) fn set_shuffled(&mut self, shuffled: bool) {
        self.is_shuffled = shuffled;
    }

    /// Restore original order of source queue
    ///
    /// Used when turning shuffle off
    pub fn restore_original_order(&mut self) {
        if self.is_shuffled {
            self.source = self.original_source.clone();
            self.source_index = 0;
            self.is_shuffled = false;
        }
    }

    /// Reload source queue from original (for Repeat All mode)
    ///
    /// Resets playback position to beginning while preserving shuffle state
    pub fn reload_source(&mut self, shuffle_mode: crate::types::ShuffleMode) {
        self.source = self.original_source.clone();

        // Re-shuffle if shuffle is enabled
        if shuffle_mode != crate::types::ShuffleMode::Off {
            crate::shuffle::shuffle_queue(&mut self.source, shuffle_mode);
        }

        self.source_index = 0;
    }

    /// Check if source queue has more tracks
    pub fn has_next_in_source(&self) -> bool {
        self.source_index < self.source.len()
    }

    /// Get current position in source queue
    pub fn get_source_position(&self) -> usize {
        self.source_index
    }

    /// Get total source queue size
    pub fn get_source_total(&self) -> usize {
        self.source.len()
    }

    /// Remove consecutive duplicate tracks from source queue
    ///
    /// Prevents the same track from playing twice in a row (UX improvement)
    pub fn remove_consecutive_duplicates(&mut self) {
        if self.source.len() <= 1 {
            return;
        }

        let mut i = 0;
        while i < self.source.len() - 1 {
            if self.source[i].id == self.source[i + 1].id {
                self.source.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Get mutable reference to original source (for updating on shuffle)
    pub(crate) fn update_original_source(&mut self) {
        if !self.is_shuffled {
            self.original_source = self.source.clone();
        }
    }

    /// Skip to track at index in queue
    ///
    /// Returns all tracks that were skipped over (for adding to history).
    /// Handles both explicit and source queue navigation properly:
    /// - If target is in explicit queue: removes tracks before it
    /// - If target is in source queue: clears explicit queue and updates source_index
    ///
    /// This preserves the source queue structure so tracks can be navigated back to
    /// using the previous button.
    pub fn skip_to_index(&mut self, index: usize) -> Option<Vec<QueueTrack>> {
        let explicit_len = self.explicit.len();

        if index >= self.len() {
            return None;
        }

        let mut skipped = Vec::new();

        if index < explicit_len {
            // Target is in explicit queue
            // Remove all explicit tracks before the target
            for _ in 0..index {
                if let Some(track) = self.explicit.first() {
                    skipped.push(track.clone());
                    self.explicit.remove(0);
                }
            }
        } else {
            // Target is in source queue
            // First, add all explicit tracks to skipped list
            skipped.extend(self.explicit.drain(..));

            // Calculate target position in source queue
            let target_in_source = index - explicit_len;

            // Add source tracks from current position up to (but not including) target
            let start = self.source_index;
            let end = self.source_index + target_in_source;

            if end <= self.source.len() {
                // Collect skipped tracks from source queue
                for i in start..end {
                    skipped.push(self.source[i].clone());
                }

                // Update source_index to point to target
                self.source_index = end;
            } else {
                // Index out of bounds for source queue
                return None;
            }
        }

        Some(skipped)
    }

    /// Check if we can go back in source queue (for previous button)
    pub fn can_go_back(&self) -> bool {
        self.source_index > 0
    }

    /// Go back one track in source queue (for previous button)
    ///
    /// Returns the track at the previous position without modifying the queue structure.
    /// This allows true index-based navigation without reordering.
    pub fn go_back(&mut self) -> Option<QueueTrack> {
        if self.source_index > 0 {
            self.source_index -= 1;
            Some(self.source[self.source_index].clone())
        } else {
            None
        }
    }

    /// Get current source index position
    pub fn current_source_index(&self) -> usize {
        self.source_index
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrackSource;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_track(id: &str, title: &str) -> QueueTrack {
        QueueTrack {
            id: id.to_string(),
            path: PathBuf::from(format!("/music/{}.mp3", id)),
            title: title.to_string(),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        }
    }

    #[test]
    fn create_empty_queue() {
        let queue = Queue::new();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn add_to_explicit_queue() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));

        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());
    }

    #[test]
    fn set_source_queue() {
        let mut queue = Queue::new();
        let tracks = vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ];

        queue.set_source(tracks);
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn explicit_queue_has_priority() {
        let mut queue = Queue::new();

        // Add to source queue
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);

        // Add to explicit queue
        queue.add_next(create_test_track("e1", "Explicit 1"));

        // Explicit should be next
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "e1");

        // Then source
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "s1");
    }

    #[test]
    fn peek_next_doesnt_remove() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));

        let peeked = queue.peek_next().unwrap();
        assert_eq!(peeked.id, "1");

        // Still there
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn remove_from_queue() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));

        let removed = queue.remove(1).unwrap();
        assert_eq!(removed.id, "2");
        assert_eq!(queue.len(), 2);

        // Verify order maintained
        assert_eq!(queue.get(0).unwrap().id, "1");
        assert_eq!(queue.get(1).unwrap().id, "3");
    }

    #[test]
    fn reorder_within_explicit() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));

        queue.reorder(0, 2).unwrap(); // Move first to last

        assert_eq!(queue.get(0).unwrap().id, "2");
        assert_eq!(queue.get(1).unwrap().id, "3");
        assert_eq!(queue.get(2).unwrap().id, "1");
    }

    #[test]
    fn cannot_reorder_across_tiers() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("e1", "Explicit 1"));
        queue.set_source(vec![create_test_track("s1", "Source 1")]);

        let result = queue.reorder(0, 1); // Try to move explicit to source
        assert!(result.is_err());
    }

    #[test]
    fn clear_queue() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.set_source(vec![create_test_track("2", "Track 2")]);

        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn clear_explicit_only() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.set_source(vec![create_test_track("2", "Track 2")]);

        queue.clear_explicit();
        assert_eq!(queue.len(), 1); // Source remains
    }

    #[test]
    fn get_all_returns_ordered() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("e1", "Explicit 1"));
        queue.add_next(create_test_track("e2", "Explicit 2"));
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);

        let all = queue.get_all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0].id, "e1");
        assert_eq!(all[1].id, "e2");
        assert_eq!(all[2].id, "s1");
        assert_eq!(all[3].id, "s2");
    }

    #[test]
    fn restore_original_order() {
        let mut queue = Queue::new();
        let tracks = vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ];

        queue.set_source(tracks);

        // Manually shuffle (shuffle algorithm will be tested separately)
        queue.source_mut().reverse();
        queue.set_shuffled(true);

        assert_eq!(queue.get(0).unwrap().id, "3"); // Reversed

        // Restore original
        queue.restore_original_order();
        assert_eq!(queue.get(0).unwrap().id, "1"); // Back to original
        assert!(!queue.is_shuffled());
    }
}
