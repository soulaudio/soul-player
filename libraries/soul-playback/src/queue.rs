//! Three-tier queue system
//!
//! Implements Spotify-style queue with:
//! - Play Next queue: User-added tracks that play immediately after current (LIFO)
//! - Add to Queue: User-added tracks that play at the end (FIFO)
//! - Source queue: Tracks from playlist/album

use crate::types::QueueTrack;

/// Three-tier queue for playback
///
/// Structure:
/// ```text
/// Currently Playing: Track A
/// ─────────────────────────────
/// Play Next Queue (LIFO, highest priority):
///   - Track B (user added, plays next)
///   - Track C (user added)
/// ─────────────────────────────
/// Source Queue (from playlist/album):
///   - Track D
///   - Track E
///   - Track F
/// ─────────────────────────────
/// Add to Queue (FIFO, lowest priority):
///   - Track G (user added)
///   - Track H (user added)
/// ```
#[derive(Debug, Clone)]
pub struct Queue {
    /// Tracks explicitly added to play next (LIFO - last added plays first)
    play_next: Vec<QueueTrack>,

    /// Tracks added to queue end (FIFO - first added plays first)
    queued_later: Vec<QueueTrack>,

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
            play_next: Vec::new(),
            queued_later: Vec::new(),
            source: Vec::new(),
            source_index: 0,
            original_source: Vec::new(),
            is_shuffled: false,
        }
    }

    /// Add track to play next (highest priority, LIFO)
    ///
    /// Track will play immediately after current track.
    /// Multiple calls result in LIFO order (last added plays first).
    pub fn add_next(&mut self, track: QueueTrack) {
        tracing::debug!(
            title = %track.title,
            new_queue_len = self.play_next.len() + 1,
            "Adding track to Play Next queue"
        );
        self.play_next.insert(0, track);
    }

    /// Add track to queue end (lowest priority, FIFO)
    ///
    /// Track will play after all Play Next and Source queue tracks.
    /// Multiple calls result in FIFO order (first added plays first).
    pub fn add_to_end(&mut self, track: QueueTrack) {
        tracing::debug!(
            title = %track.title,
            new_queue_len = self.queued_later.len() + 1,
            "Adding track to Add to Queue"
        );
        self.queued_later.push(track);
    }

    /// Add tracks from source (playlist/album)
    ///
    /// Replaces current source queue.
    /// Clears Play Next queue (new context), but keeps Add to Queue (Spotify behavior).
    pub fn set_source(&mut self, tracks: Vec<QueueTrack>) {
        // Clear Play Next on new context (Spotify behavior)
        self.play_next.clear();

        // Keep Add to Queue (persists across contexts)
        // This matches Spotify's behavior

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

    /// Remove track from queue by index (in priority order)
    ///
    /// Returns the removed track if successful
    pub fn remove(&mut self, index: usize) -> Option<QueueTrack> {
        let total = self.len();
        if index >= total {
            return None;
        }

        let play_next_len = self.play_next.len();
        let remaining_source = self.source.len().saturating_sub(self.source_index);

        if index < play_next_len {
            // Remove from Play Next queue
            Some(self.play_next.remove(index))
        } else if index < play_next_len + remaining_source {
            // Remove from source queue
            let source_idx = self.source_index + (index - play_next_len);
            let track = self.source.remove(source_idx);

            // Also remove from original (to maintain consistency)
            if let Some(pos) = self.original_source.iter().position(|t| t.id == track.id) {
                self.original_source.remove(pos);
            }

            // Adjust source_index if we removed before current position
            if source_idx < self.source_index {
                self.source_index = self.source_index.saturating_sub(1);
            }

            Some(track)
        } else {
            // Remove from Add to Queue
            let queued_later_idx = index - play_next_len - remaining_source;
            Some(self.queued_later.remove(queued_later_idx))
        }
    }

    /// Reorder track in queue
    ///
    /// Moves track from `from_index` to `to_index`.
    /// Only allows reordering within the same tier for simplicity.
    pub fn reorder(&mut self, from_index: usize, to_index: usize) -> Result<(), String> {
        let total = self.len();
        if from_index >= total || to_index >= total {
            return Err("Index out of bounds".to_string());
        }

        if from_index == to_index {
            return Ok(());
        }

        let play_next_len = self.play_next.len();
        let remaining_source = self.source.len().saturating_sub(self.source_index);
        let source_end = play_next_len + remaining_source;

        if from_index < play_next_len && to_index < play_next_len {
            // Both in Play Next queue
            let track = self.play_next.remove(from_index);
            self.play_next.insert(to_index, track);
            Ok(())
        } else if from_index >= play_next_len
            && from_index < source_end
            && to_index >= play_next_len
            && to_index < source_end
        {
            // Both in source queue
            let from_source = self.source_index + (from_index - play_next_len);
            let to_source = self.source_index + (to_index - play_next_len);
            let track = self.source.remove(from_source);
            self.source.insert(to_source, track);
            Ok(())
        } else if from_index >= source_end && to_index >= source_end {
            // Both in Add to Queue
            let from_add = from_index - source_end;
            let to_add = to_index - source_end;
            let track = self.queued_later.remove(from_add);
            self.queued_later.insert(to_add, track);
            Ok(())
        } else {
            Err("Cannot move tracks between different queue tiers".to_string())
        }
    }

    /// Clear entire queue (all three tiers)
    pub fn clear(&mut self) {
        self.play_next.clear();
        self.queued_later.clear();
        self.source.clear();
        self.source_index = 0;
        self.original_source.clear();
        self.is_shuffled = false;
    }

    /// Clear only Play Next queue
    pub fn clear_play_next(&mut self) {
        self.play_next.clear();
    }

    /// Clear only Add to Queue
    pub fn clear_queued_later(&mut self) {
        self.queued_later.clear();
    }

    /// Clear only source queue
    #[allow(dead_code)]
    pub fn clear_source(&mut self) {
        self.source.clear();
        self.source_index = 0;
        self.original_source.clear();
        self.is_shuffled = false;
    }

    /// Clear explicit queues (Play Next + Add to Queue)
    /// Kept for backwards compatibility
    #[allow(dead_code)]
    pub fn clear_explicit(&mut self) {
        self.play_next.clear();
        self.queued_later.clear();
    }

    /// Get next track to play
    ///
    /// Priority order:
    /// 1. Play Next queue (highest priority, destructive)
    /// 2. Source queue (medium priority, index-based)
    /// 3. Add to Queue (lowest priority, destructive - plays after source exhausts)
    pub fn pop_next(&mut self) -> Option<QueueTrack> {
        // Tier 1: Play Next queue (destructive, LIFO)
        if !self.play_next.is_empty() {
            return Some(self.play_next.remove(0));
        }

        // Tier 2: Source queue (index-based, non-destructive)
        if self.source_index < self.source.len() {
            let track = self.source[self.source_index].clone();
            self.source_index += 1;
            return Some(track);
        }

        // Tier 3: Add to Queue (destructive, FIFO - plays at the very end)
        if !self.queued_later.is_empty() {
            return Some(self.queued_later.remove(0));
        }

        // All queues exhausted
        None
    }

    /// Get next track while skipping Play Next queue
    ///
    /// Used when starting playback - Play Next should play AFTER the first track, not instead of it.
    /// Priority order: Source → Add to Queue (skips Play Next)
    pub(crate) fn pop_next_skip_play_next(&mut self) -> Option<QueueTrack> {
        // Tier 1: Source queue (index-based, non-destructive)
        if self.source_index < self.source.len() {
            let track = self.source[self.source_index].clone();
            self.source_index += 1;
            return Some(track);
        }

        // Tier 2: Add to Queue (destructive, FIFO)
        if !self.queued_later.is_empty() {
            return Some(self.queued_later.remove(0));
        }

        // All queues exhausted
        None
    }

    /// Peek at next track without removing
    ///
    /// Returns the next track that would be played following priority order.
    #[allow(dead_code)]
    pub fn peek_next(&self) -> Option<&QueueTrack> {
        if !self.play_next.is_empty() {
            self.play_next.first()
        } else if self.source_index < self.source.len() {
            self.source.get(self.source_index)
        } else if !self.queued_later.is_empty() {
            self.queued_later.first()
        } else {
            None
        }
    }

    /// Get all tracks in queue order from current position
    ///
    /// Returns tracks in priority order:
    /// Play Next → Source (from current position) → Add to Queue
    pub fn get_all(&self) -> Vec<&QueueTrack> {
        let mut tracks = Vec::new();

        // Tier 1: Play Next queue
        tracks.extend(self.play_next.iter());

        // Tier 2: Source queue (from current position onward)
        if self.source_index < self.source.len() {
            tracks.extend(self.source[self.source_index..].iter());
        }

        // Tier 3: Add to Queue (plays at the end)
        tracks.extend(self.queued_later.iter());

        tracing::debug!(
            total_tracks = tracks.len(),
            play_next_count = self.play_next.len(),
            source_count = if self.source_index < self.source.len() {
                self.source.len() - self.source_index
            } else {
                0
            },
            queued_later_count = self.queued_later.len(),
            "[Queue] get_all() called"
        );

        tracks
    }

    /// Get track at index (in priority order)
    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> Option<&QueueTrack> {
        let play_next_len = self.play_next.len();
        let remaining_source = self.source.len().saturating_sub(self.source_index);

        if index < play_next_len {
            self.play_next.get(index)
        } else if index < play_next_len + remaining_source {
            let source_idx = self.source_index + (index - play_next_len);
            self.source.get(source_idx)
        } else {
            let queued_later_idx = index - play_next_len - remaining_source;
            self.queued_later.get(queued_later_idx)
        }
    }

    /// Total number of remaining tracks in queue
    ///
    /// Returns the count of tracks that will still be played.
    /// Includes: Play Next + remaining Source + Add to Queue
    pub fn len(&self) -> usize {
        let remaining_source = self.source.len().saturating_sub(self.source_index);
        self.play_next.len() + remaining_source + self.queued_later.len()
    }

    /// Check if queue is empty (no remaining tracks to play)
    pub fn is_empty(&self) -> bool {
        self.play_next.is_empty()
            && self.source_index >= self.source.len()
            && self.queued_later.is_empty()
    }

    /// Check if source queue is shuffled
    #[allow(dead_code)]
    pub fn is_shuffled(&self) -> bool {
        self.is_shuffled
    }

    /// Get current position in source queue (for lazy loading)
    pub fn current_position_in_source(&self) -> usize {
        self.source_index
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
    pub fn skip_to_index(&mut self, index: usize) -> Option<Vec<QueueTrack>> {
        let play_next_len = self.play_next.len();
        let remaining_source = self.source.len().saturating_sub(self.source_index);

        if index >= self.len() {
            return None;
        }

        let mut skipped = Vec::new();

        if index < play_next_len {
            // Target is in Play Next queue
            for _ in 0..index {
                if let Some(track) = self.play_next.first() {
                    skipped.push(track.clone());
                    self.play_next.remove(0);
                }
            }
        } else if index < play_next_len + remaining_source {
            // Target is in source queue
            // First, add all Play Next tracks to skipped list
            skipped.append(&mut self.play_next);

            // Calculate target position in source queue
            let target_in_source = index - play_next_len;

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
                return None;
            }
        } else {
            // Target is in Add to Queue
            // Add all Play Next tracks
            skipped.append(&mut self.play_next);

            // Add all remaining source tracks
            for i in self.source_index..self.source.len() {
                skipped.push(self.source[i].clone());
            }
            self.source_index = self.source.len();

            // Add Add to Queue tracks up to target
            let target_in_queued_later = index - play_next_len - remaining_source;
            for _ in 0..target_in_queued_later {
                if let Some(track) = self.queued_later.first() {
                    skipped.push(track.clone());
                    self.queued_later.remove(0);
                }
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
    fn add_to_play_next_queue() {
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
    fn play_next_queue_has_highest_priority() {
        let mut queue = Queue::new();

        // Add to source queue
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);

        // Add to Play Next queue
        queue.add_next(create_test_track("n1", "Next 1"));

        // Play Next should be next
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "n1");

        // Then source
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "s1");
    }

    #[test]
    fn queued_later_has_lowest_priority() {
        let mut queue = Queue::new();

        // Add to source queue
        queue.set_source(vec![create_test_track("s1", "Source 1")]);

        // Add to Add to Queue
        queue.add_to_end(create_test_track("q1", "Queue 1"));

        // Source should be first
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "s1");

        // Then Add to Queue
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "q1");
    }

    #[test]
    fn three_tier_priority_order() {
        let mut queue = Queue::new();

        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_to_end(create_test_track("q1", "Queue 1"));
        queue.add_next(create_test_track("n1", "Next 1"));

        // Priority: Play Next → Source → Add to Queue
        assert_eq!(queue.pop_next().unwrap().id, "n1");
        assert_eq!(queue.pop_next().unwrap().id, "s1");
        assert_eq!(queue.pop_next().unwrap().id, "q1");
    }

    #[test]
    fn set_source_clears_play_next() {
        let mut queue = Queue::new();

        queue.add_next(create_test_track("n1", "Next 1"));
        queue.set_source(vec![create_test_track("s1", "Source 1")]);

        // Play Next should be cleared
        let next = queue.pop_next().unwrap();
        assert_eq!(next.id, "s1"); // Source plays, not n1
    }

    #[test]
    fn set_source_keeps_queued_later() {
        let mut queue = Queue::new();

        queue.add_to_end(create_test_track("q1", "Queue 1"));
        queue.set_source(vec![create_test_track("s1", "Source 1")]);

        // Source plays first
        assert_eq!(queue.pop_next().unwrap().id, "s1");

        // Add to Queue persists
        assert_eq!(queue.pop_next().unwrap().id, "q1");
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
    fn clear_play_next_only() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_to_end(create_test_track("q1", "Queue 1"));

        queue.clear_play_next();

        assert_eq!(queue.len(), 2); // Source + Add to Queue remain
    }

    #[test]
    fn clear_queued_later_only() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_next(create_test_track("n1", "Next 1")); // Add after set_source
        queue.add_to_end(create_test_track("q1", "Queue 1"));

        queue.clear_queued_later();

        assert_eq!(queue.len(), 2); // Play Next + Source remain
    }

    #[test]
    fn add_next_inserts_at_front() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));

        // add_next inserts at front (LIFO), so order should be 3, 2, 1
        assert_eq!(queue.pop_next().unwrap().id, "3");
        assert_eq!(queue.pop_next().unwrap().id, "2");
        assert_eq!(queue.pop_next().unwrap().id, "1");
    }

    #[test]
    fn add_to_end_fifo_order() {
        let mut queue = Queue::new();
        // Exhaust source queue first
        queue.set_source(vec![]);

        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));

        // FIFO order
        assert_eq!(queue.pop_next().unwrap().id, "1");
        assert_eq!(queue.pop_next().unwrap().id, "2");
        assert_eq!(queue.pop_next().unwrap().id, "3");
    }

    #[test]
    fn get_all_returns_ordered() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_next(create_test_track("n1", "Next 1")); // Add after set_source
        queue.add_to_end(create_test_track("q1", "Queue 1"));

        let all = queue.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "n1"); // Play Next
        assert_eq!(all[1].id, "s1"); // Source
        assert_eq!(all[2].id, "q1"); // Add to Queue
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

        // Manually shuffle
        queue.source_mut().reverse();
        queue.set_shuffled(true);

        assert_eq!(queue.get(0).unwrap().id, "3"); // Reversed

        // Restore original
        queue.restore_original_order();
        assert_eq!(queue.get(0).unwrap().id, "1"); // Back to original
        assert!(!queue.is_shuffled());
    }
}
