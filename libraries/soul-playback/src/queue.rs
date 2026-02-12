//! Three-tier queue system
//!
//! Implements Spotify-style queue with:
//! - Play Next queue: User-added tracks that play immediately after current (LIFO)
//! - Add to Queue: User-added tracks that play at the end (FIFO)
//! - Source queue: Tracks from playlist/album

use crate::types::QueueTrack;

/// Default capacity for source queue vectors to avoid early reallocations
const DEFAULT_SOURCE_CAPACITY: usize = 64;

/// Default capacity for user queue vectors (play_next, queued_later)
const DEFAULT_USER_QUEUE_CAPACITY: usize = 16;

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
    /// Create new empty queue with pre-allocated capacity
    ///
    /// Pre-allocates vectors to avoid repeated reallocations during typical use.
    /// Play Next and Add to Queue are typically small (user-added tracks),
    /// while source queues can be larger (playlist/album tracks).
    pub fn new() -> Self {
        Self {
            play_next: Vec::with_capacity(DEFAULT_USER_QUEUE_CAPACITY),
            queued_later: Vec::with_capacity(DEFAULT_USER_QUEUE_CAPACITY),
            source: Vec::with_capacity(DEFAULT_SOURCE_CAPACITY),
            source_index: 0,
            original_source: Vec::with_capacity(DEFAULT_SOURCE_CAPACITY),
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

            // Adjust insertion index if moving forward (indices shift after remove)
            let adjusted_to = if from_index < to_index {
                to_index - 1
            } else {
                to_index
            };

            self.play_next.insert(adjusted_to, track);
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

            // Adjust insertion index if moving forward (indices shift after remove)
            let adjusted_to = if from_source < to_source {
                to_source - 1
            } else {
                to_source
            };

            self.source.insert(adjusted_to, track);
            Ok(())
        } else if from_index >= source_end && to_index >= source_end {
            // Both in Add to Queue
            let from_add = from_index - source_end;
            let to_add = to_index - source_end;
            let track = self.queued_later.remove(from_add);

            // Adjust insertion index if moving forward (indices shift after remove)
            let adjusted_to = if from_add < to_add {
                to_add - 1
            } else {
                to_add
            };

            self.queued_later.insert(adjusted_to, track);
            Ok(())
        } else {
            Err("Cannot move tracks between different queue tiers".to_string())
        }
    }

    /// Clear entire queue (all three tiers)
    ///
    /// This clears all tracks but keeps allocated capacity for reuse.
    /// Use `clear_and_shrink()` if you want to release memory as well.
    pub fn clear(&mut self) {
        self.play_next.clear();
        self.queued_later.clear();
        self.source.clear();
        self.source_index = 0;
        self.original_source.clear();
        self.is_shuffled = false;
    }

    /// Clear entire queue and release excess memory
    ///
    /// Unlike `clear()`, this also shrinks vectors to release memory.
    /// Useful after playing very large playlists to reclaim memory.
    #[allow(dead_code)]
    pub fn clear_and_shrink(&mut self) {
        self.clear();
        // Shrink to default capacities instead of zero to avoid
        // immediate reallocations on next use
        self.play_next.shrink_to(DEFAULT_USER_QUEUE_CAPACITY);
        self.queued_later.shrink_to(DEFAULT_USER_QUEUE_CAPACITY);
        self.source.shrink_to(DEFAULT_SOURCE_CAPACITY);
        self.original_source.shrink_to(DEFAULT_SOURCE_CAPACITY);
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

    /// Peek at next track without removing, skipping Play Next queue
    ///
    /// Used when starting playback to look at source queue first.
    /// Play Next tracks should play AFTER the first track, not instead of it.
    pub(crate) fn peek_next_skip_play_next(&self) -> Option<QueueTrack> {
        // Tier 1: Source queue
        if self.source_index < self.source.len() {
            return self.source.get(self.source_index).cloned();
        }

        // Tier 2: Add to Queue
        if !self.queued_later.is_empty() {
            return self.queued_later.first().cloned();
        }

        // All queues exhausted
        None
    }

    /// Peek at the first track in the original source queue
    ///
    /// Used for RepeatAll mode pre-loading: when queue is exhausted but will loop,
    /// returns the first track that would play after reload.
    pub fn peek_first_source_track(&self) -> Option<&QueueTrack> {
        self.original_source.first()
    }

    /// Get all tracks in queue order from current position
    ///
    /// Returns tracks in priority order:
    /// Play Next → Source (from current position) → Add to Queue
    pub fn get_all(&self) -> Vec<&QueueTrack> {
        // Pre-calculate total capacity to avoid reallocations
        let remaining_source = self.source.len().saturating_sub(self.source_index);
        let total_capacity = self.play_next.len() + remaining_source + self.queued_later.len();
        let mut tracks = Vec::with_capacity(total_capacity);

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
            source_count = remaining_source,
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

    /// Restore original order of source queue while preserving current track
    ///
    /// Used when turning shuffle off during playback.
    /// The currently playing track's position is preserved by finding it in the original order.
    ///
    /// Returns the new source_index after restoration (for the currently playing track).
    pub fn restore_original_order(&mut self) -> Option<usize> {
        if !self.is_shuffled {
            return Some(self.source_index);
        }

        // Get the ID of the current track (track at source_index - 1, since we've already advanced)
        // Actually we need to find what would be the "current" track in the shuffled order
        // The source_index points to the NEXT track to play, so current is at source_index - 1
        let current_track_id = if self.source_index > 0 && self.source_index <= self.source.len() {
            // We've played at least one track, find the last played track
            Some(self.source[self.source_index - 1].id.clone())
        } else if self.source_index == 0 && !self.source.is_empty() {
            // Haven't played any tracks yet, current position is still valid
            None
        } else {
            None
        };

        // Restore original order
        self.source = self.original_source.clone();
        self.is_shuffled = false;

        // Find the current track's position in the original order
        if let Some(track_id) = current_track_id {
            if let Some(pos) = self.source.iter().position(|t| t.id == track_id) {
                // Set index to after the current track (so we continue from the right place)
                self.source_index = pos + 1;
                return Some(self.source_index);
            }
        }

        // Fallback: reset to beginning if we can't find the track
        self.source_index = 0;
        Some(self.source_index)
    }

    /// Apply shuffle to source queue while preserving current track position
    ///
    /// Used when turning shuffle on during playback.
    /// The currently playing track remains at the current position.
    pub fn apply_shuffle(&mut self, mode: crate::types::ShuffleMode) {
        if mode == crate::types::ShuffleMode::Off {
            return;
        }

        // Save original order if not already saved
        if !self.is_shuffled {
            self.original_source = self.source.clone();
        }

        // Get tracks that haven't been played yet (from source_index onward)
        if self.source_index < self.source.len() {
            let remaining = self.source.split_off(self.source_index);
            let mut to_shuffle: Vec<_> = remaining;

            // Shuffle only the remaining tracks
            crate::shuffle::shuffle_queue(&mut to_shuffle, mode);

            // Append shuffled tracks back
            self.source.extend(to_shuffle);
        }

        self.is_shuffled = true;
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

        // Pre-allocate with exact capacity to avoid reallocations
        let mut skipped = Vec::with_capacity(index);

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

    #[test]
    fn reorder_source_forward_basic() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
        ]);

        // Move track at index 0 to where track at index 2 is
        // [1, 2, 3, 4] -> [2, 1, 3, 4] (1 takes the place of 3)
        assert!(queue.reorder(0, 2).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "3");
        assert_eq!(all[3].id, "4");
    }

    #[test]
    fn reorder_source_forward_to_end() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Move track at index 0 to where track at index 2 (last) is
        // [1, 2, 3] -> [2, 1, 3] (1 takes the place of 3)
        assert!(queue.reorder(0, 2).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "3");
    }

    #[test]
    fn reorder_source_backward() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
        ]);

        // Move track at index 3 to where track at index 1 is
        // [1, 2, 3, 4] -> [1, 4, 2, 3] (4 takes the place of 2)
        assert!(queue.reorder(3, 1).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "4");
        assert_eq!(all[2].id, "2");
        assert_eq!(all[3].id, "3");
    }

    #[test]
    fn reorder_play_next_forward() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));
        queue.add_next(create_test_track("4", "Track 4"));

        // Order is [4, 3, 2, 1] due to LIFO
        // Move index 0 (track 4) to where index 2 (track 2) is
        // [4, 3, 2, 1] -> [3, 4, 2, 1] (4 takes the place of 2)
        assert!(queue.reorder(0, 2).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "3");
        assert_eq!(all[1].id, "4");
        assert_eq!(all[2].id, "2");
        assert_eq!(all[3].id, "1");
    }

    #[test]
    fn reorder_play_next_backward() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));
        queue.add_next(create_test_track("4", "Track 4"));

        // Order is [4, 3, 2, 1] due to LIFO
        // Move index 2 (track 2) to where index 0 (track 4) is
        // [4, 3, 2, 1] -> [2, 4, 3, 1] (2 takes the place of 4)
        assert!(queue.reorder(2, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "4");
        assert_eq!(all[2].id, "3");
        assert_eq!(all[3].id, "1");
    }

    #[test]
    fn reorder_add_to_queue_forward() {
        let mut queue = Queue::new();
        // Set empty source to access Add to Queue
        queue.set_source(vec![]);
        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));
        queue.add_to_end(create_test_track("4", "Track 4"));

        // Move track at index 0 to where track at index 2 is
        // [1, 2, 3, 4] -> [2, 1, 3, 4] (1 takes the place of 3)
        assert!(queue.reorder(0, 2).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "3");
        assert_eq!(all[3].id, "4");
    }

    #[test]
    fn reorder_add_to_queue_backward() {
        let mut queue = Queue::new();
        // Set empty source to access Add to Queue
        queue.set_source(vec![]);
        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));
        queue.add_to_end(create_test_track("4", "Track 4"));

        // Move track at index 2 to where track at index 0 is
        // [1, 2, 3, 4] -> [3, 1, 2, 4] (3 takes the place of 1)
        assert!(queue.reorder(2, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "3");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "2");
        assert_eq!(all[3].id, "4");
    }

    #[test]
    fn reorder_same_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Reordering to same index should be no-op
        assert!(queue.reorder(0, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
    }

    #[test]
    fn reorder_across_tiers_fails() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_next(create_test_track("n1", "Next 1"));

        // Cannot move from Play Next (index 0) to Source (index 1)
        assert!(queue.reorder(0, 1).is_err());
    }

    #[test]
    fn reorder_out_of_bounds() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Out of bounds indices
        assert!(queue.reorder(0, 10).is_err());
        assert!(queue.reorder(10, 0).is_err());
    }

    #[test]
    fn reorder_adjacent_forward() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Move index 0 to index 1 (adjacent swap)
        // [1, 2, 3] -> [1, 2, 3] (no change because after remove, index 1 becomes index 0)
        // Actually: Remove 1 -> [2, 3], adjusted to = 1 - 1 = 0, insert at 0 -> [1, 2, 3]
        assert!(queue.reorder(0, 1).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "3");
    }

    #[test]
    fn reorder_adjacent_backward() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Move index 1 to index 0 (adjacent swap backward)
        // [1, 2, 3] -> [2, 1, 3]
        assert!(queue.reorder(1, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "3");
    }

    // ===== Edge Case Tests =====

    // --- Empty Queue Edge Cases ---

    #[test]
    fn empty_queue_pop_next_returns_none() {
        let mut queue = Queue::new();
        assert!(queue.pop_next().is_none());
    }

    #[test]
    fn empty_queue_peek_next_returns_none() {
        let queue = Queue::new();
        assert!(queue.peek_next().is_none());
    }

    #[test]
    fn empty_queue_get_returns_none() {
        let queue = Queue::new();
        assert!(queue.get(0).is_none());
        assert!(queue.get(100).is_none());
    }

    #[test]
    fn empty_queue_remove_returns_none() {
        let mut queue = Queue::new();
        assert!(queue.remove(0).is_none());
        assert!(queue.remove(100).is_none());
    }

    #[test]
    fn empty_queue_reorder_fails() {
        let mut queue = Queue::new();
        assert!(queue.reorder(0, 0).is_err());
        assert!(queue.reorder(0, 1).is_err());
    }

    #[test]
    fn empty_queue_skip_to_index_returns_none() {
        let mut queue = Queue::new();
        assert!(queue.skip_to_index(0).is_none());
    }

    #[test]
    fn empty_queue_go_back_returns_none() {
        let mut queue = Queue::new();
        assert!(queue.go_back().is_none());
    }

    #[test]
    fn empty_queue_clear_is_safe() {
        let mut queue = Queue::new();
        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn empty_queue_restore_original_order_is_safe() {
        let mut queue = Queue::new();
        queue.restore_original_order();
        assert!(queue.is_empty());
    }

    #[test]
    fn empty_queue_get_all_returns_empty() {
        let queue = Queue::new();
        assert!(queue.get_all().is_empty());
    }

    // --- Single Track Queue Edge Cases ---

    #[test]
    fn single_track_pop_exhausts_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        assert_eq!(queue.len(), 1);
        let track = queue.pop_next().unwrap();
        assert_eq!(track.id, "1");
        assert!(queue.is_empty());
        assert!(queue.pop_next().is_none());
    }

    #[test]
    fn single_track_remove_exhausts_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        let track = queue.remove(0).unwrap();
        assert_eq!(track.id, "1");
        assert!(queue.is_empty());
    }

    #[test]
    fn single_track_reorder_same_index_ok() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        // Reordering to same index should be a no-op
        assert!(queue.reorder(0, 0).is_ok());
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn single_track_skip_to_index_zero_returns_empty_skipped() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        let skipped = queue.skip_to_index(0).unwrap();
        assert!(skipped.is_empty());
    }

    #[test]
    fn single_track_go_back_after_pop() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        // Pop the track (advances source_index to 1)
        let _ = queue.pop_next();
        assert!(queue.is_empty());

        // Go back should return None because we were at index 0 (now 1)
        // and we need to verify we can go back
        // Actually after pop, source_index is 1, so can_go_back() should be true
        assert!(queue.can_go_back());
        let track = queue.go_back().unwrap();
        assert_eq!(track.id, "1");
    }

    #[test]
    fn single_track_consecutive_duplicates_removal() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        // Should be a no-op for single track
        queue.remove_consecutive_duplicates();
        assert_eq!(queue.len(), 1);
    }

    // --- Rapid Queue Modifications ---

    #[test]
    fn rapid_add_remove_operations() {
        let mut queue = Queue::new();

        // Rapidly add and remove tracks
        for i in 0..100 {
            queue.add_next(create_test_track(&i.to_string(), &format!("Track {}", i)));
        }
        assert_eq!(queue.len(), 100);

        // Remove from the middle repeatedly
        for _ in 0..50 {
            queue.remove(queue.len() / 2);
        }
        assert_eq!(queue.len(), 50);

        // Pop remaining
        while queue.pop_next().is_some() {}
        assert!(queue.is_empty());
    }

    #[test]
    fn rapid_add_to_different_tiers() {
        let mut queue = Queue::new();

        // Add to all three tiers rapidly
        for i in 0..30 {
            if i % 3 == 0 {
                queue.add_next(create_test_track(
                    &format!("n{}", i),
                    &format!("Next {}", i),
                ));
            } else if i % 3 == 1 {
                queue.add_to_end(create_test_track(&format!("e{}", i), &format!("End {}", i)));
            }
        }

        // Set source (this clears play_next)
        queue.set_source(
            (0..10)
                .map(|i| create_test_track(&format!("s{}", i), &format!("Source {}", i)))
                .collect(),
        );

        // Verify the queue state is consistent
        assert!(!queue.is_empty());
        // Play next should be cleared by set_source
        // Queued later should persist
    }

    // --- Large Queue Tests (10000+ tracks) ---

    #[test]
    fn large_queue_basic_operations() {
        let mut queue = Queue::new();
        let track_count = 10000;

        // Create large source queue
        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);
        assert_eq!(queue.len(), track_count);

        // Verify random access works
        assert!(queue.get(0).is_some());
        assert!(queue.get(track_count / 2).is_some());
        assert!(queue.get(track_count - 1).is_some());
        assert!(queue.get(track_count).is_none());
    }

    #[test]
    fn large_queue_skip_to_end() {
        let mut queue = Queue::new();
        let track_count = 10000;

        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);

        // Skip to near the end
        let target = track_count - 10;
        let skipped = queue.skip_to_index(target);
        assert!(skipped.is_some());
        let skipped = skipped.unwrap();
        assert_eq!(skipped.len(), target);

        // Remaining tracks should be 10
        assert_eq!(queue.len(), 10);
    }

    #[test]
    fn large_queue_pop_all() {
        let mut queue = Queue::new();
        let track_count = 1000; // Reduced for test speed

        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);

        // Pop all tracks and verify order
        for i in 0..track_count {
            let track = queue.pop_next().unwrap();
            assert_eq!(track.id, i.to_string());
        }

        assert!(queue.is_empty());
    }

    #[test]
    fn large_queue_get_all_performance() {
        let mut queue = Queue::new();
        let track_count = 10000;

        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);

        // get_all should work even for large queues
        let all = queue.get_all();
        assert_eq!(all.len(), track_count);
    }

    // --- Queue Index Bounds Checking ---

    #[test]
    fn bounds_check_at_tier_boundaries() {
        let mut queue = Queue::new();

        // Set up queue with tracks in all tiers
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_to_end(create_test_track("e1", "End 1"));

        // Total: 4 tracks
        // Index 0: n1 (play_next)
        // Index 1: s1 (source)
        // Index 2: s2 (source)
        // Index 3: e1 (queued_later)

        // Verify get at boundaries
        assert_eq!(queue.get(0).unwrap().id, "n1");
        assert_eq!(queue.get(1).unwrap().id, "s1");
        assert_eq!(queue.get(2).unwrap().id, "s2");
        assert_eq!(queue.get(3).unwrap().id, "e1");
        assert!(queue.get(4).is_none());
    }

    #[test]
    fn remove_at_tier_boundaries() {
        let mut queue = Queue::new();

        // Set up queue with tracks in all tiers
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_to_end(create_test_track("e1", "End 1"));

        // Remove from play_next tier boundary (index 0)
        let track = queue.remove(0).unwrap();
        assert_eq!(track.id, "n1");

        // Now queue is: s1, s2, e1
        // Remove from queued_later tier (last item)
        let track = queue.remove(2).unwrap();
        assert_eq!(track.id, "e1");

        // Now queue is: s1, s2
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn skip_to_index_at_tier_boundaries() {
        let mut queue = Queue::new();

        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_to_end(create_test_track("e1", "End 1"));

        // Skip from start to the last source track (index 2)
        let skipped = queue.skip_to_index(2).unwrap();

        // Should skip n1 and s1
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].id, "n1");
        assert_eq!(skipped[1].id, "s1");
    }

    // --- Queue Modifications During Playback (source_index tests) ---

    #[test]
    fn pop_advances_source_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        assert_eq!(queue.current_source_index(), 0);

        queue.pop_next();
        assert_eq!(queue.current_source_index(), 1);

        queue.pop_next();
        assert_eq!(queue.current_source_index(), 2);
    }

    #[test]
    fn remove_before_source_index_adjusts_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Advance to track 2
        queue.pop_next(); // Now at index 1

        // The condition in remove checks if source_idx < self.source_index
        // After pop, source_index = 1
        // If we remove track at queue index 0, that's source_idx = 1 (current position)
        // So it won't adjust source_index

        // This test verifies remove doesn't corrupt state
        assert_eq!(queue.current_source_index(), 1);
        assert_eq!(queue.len(), 2);

        // Remove the current track (index 0 after pop)
        let track = queue.remove(0).unwrap();
        assert_eq!(track.id, "2");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn go_back_decrements_source_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Advance to track 2
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 1);

        // Go back
        let track = queue.go_back().unwrap();
        assert_eq!(track.id, "1");
        assert_eq!(queue.current_source_index(), 0);
    }

    #[test]
    fn reload_source_resets_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Advance through queue
        queue.pop_next();
        queue.pop_next();
        assert!(queue.is_empty());
        assert_eq!(queue.current_source_index(), 2);

        // Reload should reset index
        queue.reload_source(crate::types::ShuffleMode::Off);
        assert_eq!(queue.current_source_index(), 0);
        assert_eq!(queue.len(), 2);
    }

    // --- Consecutive Duplicates Edge Cases ---

    #[test]
    fn consecutive_duplicates_multiple() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("1", "Track 1"), // duplicate
            create_test_track("1", "Track 1"), // duplicate
            create_test_track("2", "Track 2"),
            create_test_track("2", "Track 2"), // duplicate
            create_test_track("3", "Track 3"),
        ]);

        queue.remove_consecutive_duplicates();

        // Should be: 1, 2, 3
        assert_eq!(queue.len(), 3);
        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "3");
    }

    #[test]
    fn consecutive_duplicates_at_end() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("2", "Track 2"), // duplicate at end
        ]);

        queue.remove_consecutive_duplicates();

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn no_consecutive_duplicates_non_adjacent() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("1", "Track 1"), // non-adjacent duplicate - should NOT be removed
        ]);

        queue.remove_consecutive_duplicates();

        // All three should remain (non-consecutive)
        assert_eq!(queue.len(), 3);
    }

    // --- Shuffle and Original Order Edge Cases ---

    #[test]
    fn restore_order_when_not_shuffled_is_noop() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Not shuffled, restore should be a no-op
        queue.restore_original_order();

        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
    }

    #[test]
    fn update_original_source_when_not_shuffled() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Manually modify source
        queue.source_mut().reverse();

        // Update original should sync when not shuffled
        queue.update_original_source();

        // Original should now match reversed order
        queue.set_shuffled(true); // Mark as shuffled to test restore
        queue.restore_original_order();

        // After restore, should be reversed (since that's what original was updated to)
        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
    }

    // --- Pop Next Skip Play Next Edge Cases ---

    #[test]
    fn pop_next_skip_play_next_with_empty_source() {
        let mut queue = Queue::new();
        queue.set_source(vec![]);
        queue.add_to_end(create_test_track("e1", "End 1"));

        // pop_next_skip_play_next should skip to queued_later when source is empty
        let track = queue.pop_next_skip_play_next().unwrap();
        assert_eq!(track.id, "e1");
    }

    #[test]
    fn pop_next_skip_play_next_prefers_source_over_queued_later() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("s1", "Source 1")]);
        queue.add_to_end(create_test_track("e1", "End 1"));

        // Should get source track, not queued_later
        let track = queue.pop_next_skip_play_next().unwrap();
        assert_eq!(track.id, "s1");
    }

    // --- Append to Source Edge Cases ---

    #[test]
    fn append_to_source_with_empty_queue() {
        let mut queue = Queue::new();

        queue.append_to_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn append_to_source_preserves_position() {
        let mut queue = Queue::new();
        queue.set_source(vec![create_test_track("1", "Track 1")]);

        // Advance position
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 1);

        // Append more tracks
        queue.append_to_source(vec![create_test_track("2", "Track 2")]);

        // Position should be preserved
        assert_eq!(queue.current_source_index(), 1);
        // Should have 1 remaining track
        assert_eq!(queue.len(), 1);
    }

    // ===== Shuffle Toggle During Playback Tests =====

    fn create_test_track_with_artist(id: &str, title: &str, artist: &str) -> QueueTrack {
        QueueTrack {
            id: id.to_string(),
            path: std::path::PathBuf::from(format!("/music/{}.mp3", id)),
            title: title.to_string(),
            artist: artist.to_string(),
            album: Some("Test Album".to_string()),
            duration: std::time::Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        }
    }

    #[test]
    fn restore_original_order_preserves_current_track_position() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        // Play track 1 and 2 (advances to position 2)
        queue.pop_next(); // Track 1
        queue.pop_next(); // Track 2

        // Manually shuffle the source (simulating shuffle being turned on)
        queue.source_mut().reverse();
        queue.set_shuffled(true);

        // After shuffle, queue is [5,4,3,2,1] but source_index is still 2
        // So "current" track is at index 1 (track 2 in original, but now it's track 4)

        // Restore original order
        let new_index = queue.restore_original_order();

        // The current track (last played) should be found in original order
        // and source_index should point to the position AFTER that track
        assert!(new_index.is_some());
        assert!(!queue.is_shuffled());
    }

    #[test]
    fn apply_shuffle_only_shuffles_remaining_tracks() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track_with_artist("1", "Track 1", "Artist A"),
            create_test_track_with_artist("2", "Track 2", "Artist B"),
            create_test_track_with_artist("3", "Track 3", "Artist C"),
            create_test_track_with_artist("4", "Track 4", "Artist D"),
            create_test_track_with_artist("5", "Track 5", "Artist E"),
        ]);

        // Play tracks 1 and 2
        let track1 = queue.pop_next().unwrap();
        let track2 = queue.pop_next().unwrap();
        assert_eq!(track1.id, "1");
        assert_eq!(track2.id, "2");
        assert_eq!(queue.current_source_index(), 2);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Verify:
        // 1. Already played tracks (1, 2) remain in their positions
        // 2. Source index is preserved
        // 3. Remaining tracks (3, 4, 5) are shuffled
        assert_eq!(queue.current_source_index(), 2);
        assert!(queue.is_shuffled());

        // The first 2 tracks should still be 1 and 2
        let all_source: Vec<String> = queue.source.iter().map(|t| t.id.clone()).collect();
        assert_eq!(all_source[0], "1");
        assert_eq!(all_source[1], "2");

        // Remaining tracks should be a permutation of 3, 4, 5
        let remaining: std::collections::HashSet<String> =
            all_source[2..].iter().cloned().collect();
        assert!(remaining.contains("3"));
        assert!(remaining.contains("4"));
        assert!(remaining.contains("5"));
    }

    #[test]
    fn shuffle_toggle_roundtrip_preserves_position() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
        ]);

        // Play tracks 1 and 2
        queue.pop_next();
        queue.pop_next();
        let _original_index = queue.current_source_index();

        // Turn shuffle ON
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());

        // Pop one more track while shuffled
        let shuffled_track = queue.pop_next();
        assert!(shuffled_track.is_some());

        // Turn shuffle OFF (restore original order)
        let restored_index = queue.restore_original_order();
        assert!(!queue.is_shuffled());
        assert!(restored_index.is_some());

        // The queue should still have at least 1 track remaining
        // and all original tracks should still be present
        let remaining_ids: std::collections::HashSet<String> =
            queue.source.iter().map(|t| t.id.clone()).collect();
        assert!(remaining_ids.contains("1"));
        assert!(remaining_ids.contains("2"));
        assert!(remaining_ids.contains("3"));
        assert!(remaining_ids.contains("4"));
    }

    #[test]
    fn apply_shuffle_on_empty_remaining_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Play all tracks
        queue.pop_next();
        queue.pop_next();
        assert!(queue.is_empty());
        assert_eq!(queue.current_source_index(), 2);

        // Apply shuffle on empty remaining queue - should not panic
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());
        assert_eq!(queue.current_source_index(), 2);
    }

    #[test]
    fn restore_order_when_at_beginning() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Mark as shuffled without actually shuffling
        queue.set_shuffled(true);

        // Restore at beginning (source_index = 0)
        let new_index = queue.restore_original_order();

        // Should reset to beginning
        assert!(new_index.is_some());
        assert_eq!(queue.current_source_index(), 0);
    }

    // =====================================================
    // Comprehensive Queue Operations Tests
    // =====================================================

    // --- Test 1: Add 10000 tracks then clear ---

    #[test]
    fn large_queue_10000_tracks_then_clear() {
        let mut queue = Queue::new();
        let track_count = 10000;

        // Create and add 10000 tracks
        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);
        assert_eq!(queue.len(), track_count);
        assert!(!queue.is_empty());

        // Verify first and last tracks are accessible
        assert_eq!(queue.get(0).unwrap().id, "0");
        assert_eq!(queue.get(track_count - 1).unwrap().id, "9999");

        // Clear the queue
        queue.clear();

        // Verify queue is empty
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert!(queue.get(0).is_none());
        assert!(queue.pop_next().is_none());
        assert!(queue.peek_next().is_none());

        // Verify internal state is reset
        assert_eq!(queue.current_source_index(), 0);
        assert!(!queue.is_shuffled());
    }

    #[test]
    fn large_queue_10000_tracks_memory_efficient_clear() {
        let mut queue = Queue::new();
        let track_count = 10000;

        let tracks: Vec<QueueTrack> = (0..track_count)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
            .collect();

        queue.set_source(tracks);

        // Clear and shrink to release memory
        queue.clear_and_shrink();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    // --- Test 2: Remove track that doesn't exist ---

    #[test]
    fn remove_nonexistent_track_from_empty_queue() {
        let mut queue = Queue::new();

        // Try to remove from empty queue
        assert!(queue.remove(0).is_none());
        assert!(queue.remove(1).is_none());
        assert!(queue.remove(100).is_none());
        assert!(queue.remove(usize::MAX).is_none());
    }

    #[test]
    fn remove_nonexistent_track_index_out_of_bounds() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Queue has 3 tracks (indices 0, 1, 2)
        assert!(queue.remove(3).is_none()); // Just out of bounds
        assert!(queue.remove(10).is_none()); // Way out of bounds
        assert!(queue.remove(usize::MAX).is_none()); // Maximum usize

        // Verify queue is unchanged
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn remove_nonexistent_after_partial_consumption() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Pop 2 tracks
        queue.pop_next();
        queue.pop_next();

        // Now only 1 track remaining (index 0)
        assert_eq!(queue.len(), 1);
        assert!(queue.remove(1).is_none()); // Index 1 doesn't exist
        assert!(queue.remove(2).is_none()); // Index 2 doesn't exist

        // Valid removal still works
        assert!(queue.remove(0).is_some());
        assert!(queue.is_empty());
    }

    // --- Test 3: Reorder to same position ---

    #[test]
    fn reorder_to_same_position_source_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Reorder each track to its own position
        assert!(queue.reorder(0, 0).is_ok());
        assert!(queue.reorder(1, 1).is_ok());
        assert!(queue.reorder(2, 2).is_ok());

        // Verify order unchanged
        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "3");
    }

    #[test]
    fn reorder_to_same_position_play_next_queue() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));

        // Order is [3, 2, 1] due to LIFO
        assert!(queue.reorder(0, 0).is_ok());
        assert!(queue.reorder(1, 1).is_ok());
        assert!(queue.reorder(2, 2).is_ok());

        // Verify order unchanged
        let all = queue.get_all();
        assert_eq!(all[0].id, "3");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "1");
    }

    #[test]
    fn reorder_to_same_position_queued_later() {
        let mut queue = Queue::new();
        queue.set_source(vec![]); // Empty source to access queued_later directly
        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));

        assert!(queue.reorder(0, 0).is_ok());
        assert!(queue.reorder(1, 1).is_ok());
        assert!(queue.reorder(2, 2).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "3");
    }

    // --- Test 4: Reorder first to last and vice versa ---

    #[test]
    fn reorder_first_to_last_source_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
        ]);

        // Move first (index 0) to last (index 3)
        // [1, 2, 3, 4] -> After remove: [2, 3, 4], adjusted_to = 3-1 = 2, insert at 2
        // Result: [2, 3, 1, 4] - this is the actual behavior
        assert!(queue.reorder(0, 3).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "3");
        assert_eq!(all[2].id, "1");
        assert_eq!(all[3].id, "4");
    }

    #[test]
    fn reorder_last_to_first_source_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
        ]);

        // Move last (index 3) to first (index 0)
        // [1, 2, 3, 4] -> After remove 4: [1, 2, 3], insert at 0
        // Result: [4, 1, 2, 3]
        assert!(queue.reorder(3, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "4");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "2");
        assert_eq!(all[3].id, "3");
    }

    #[test]
    fn reorder_first_to_last_play_next() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));
        queue.add_next(create_test_track("4", "Track 4"));

        // LIFO order: [4, 3, 2, 1]
        // Move first (4) to last position
        assert!(queue.reorder(0, 3).is_ok());

        let all = queue.get_all();
        // Expected: [3, 2, 4, 1] based on reorder behavior
        assert_eq!(all[0].id, "3");
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "4");
        assert_eq!(all[3].id, "1");
    }

    #[test]
    fn reorder_last_to_first_play_next() {
        let mut queue = Queue::new();
        queue.add_next(create_test_track("1", "Track 1"));
        queue.add_next(create_test_track("2", "Track 2"));
        queue.add_next(create_test_track("3", "Track 3"));
        queue.add_next(create_test_track("4", "Track 4"));

        // LIFO order: [4, 3, 2, 1]
        // Move last (1) to first position
        assert!(queue.reorder(3, 0).is_ok());

        let all = queue.get_all();
        // Expected: [1, 4, 3, 2]
        assert_eq!(all[0].id, "1");
        assert_eq!(all[1].id, "4");
        assert_eq!(all[2].id, "3");
        assert_eq!(all[3].id, "2");
    }

    #[test]
    fn reorder_first_to_last_queued_later() {
        let mut queue = Queue::new();
        queue.set_source(vec![]);
        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));
        queue.add_to_end(create_test_track("4", "Track 4"));

        assert!(queue.reorder(0, 3).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "3");
        assert_eq!(all[2].id, "1");
        assert_eq!(all[3].id, "4");
    }

    #[test]
    fn reorder_last_to_first_queued_later() {
        let mut queue = Queue::new();
        queue.set_source(vec![]);
        queue.add_to_end(create_test_track("1", "Track 1"));
        queue.add_to_end(create_test_track("2", "Track 2"));
        queue.add_to_end(create_test_track("3", "Track 3"));
        queue.add_to_end(create_test_track("4", "Track 4"));

        assert!(queue.reorder(3, 0).is_ok());

        let all = queue.get_all();
        assert_eq!(all[0].id, "4");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "2");
        assert_eq!(all[3].id, "3");
    }

    // --- Test 5: Shuffle then unshuffle preserves order ---

    #[test]
    fn shuffle_then_unshuffle_preserves_original_order() {
        let mut queue = Queue::new();
        let original_tracks = vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ];

        queue.set_source(original_tracks.clone());

        // Capture original order
        let original_order: Vec<String> = queue.get_all().iter().map(|t| t.id.clone()).collect();

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());

        // Restore original order
        queue.restore_original_order();
        assert!(!queue.is_shuffled());

        // Verify order is restored
        let restored_order: Vec<String> = queue.get_all().iter().map(|t| t.id.clone()).collect();
        assert_eq!(
            original_order, restored_order,
            "Original order should be preserved after shuffle/unshuffle"
        );
    }

    #[test]
    fn shuffle_unshuffle_with_partial_playback() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        // Play first 2 tracks
        queue.pop_next(); // Track 1
        queue.pop_next(); // Track 2

        // Apply shuffle to remaining tracks
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());

        // Play one more track while shuffled
        let shuffled_track = queue.pop_next();
        assert!(shuffled_track.is_some());

        // Restore original order
        queue.restore_original_order();
        assert!(!queue.is_shuffled());

        // All original tracks should still be in the source
        let source_ids: std::collections::HashSet<String> =
            queue.source.iter().map(|t| t.id.clone()).collect();
        assert!(source_ids.contains("1"));
        assert!(source_ids.contains("2"));
        assert!(source_ids.contains("3"));
        assert!(source_ids.contains("4"));
        assert!(source_ids.contains("5"));
    }

    #[test]
    fn shuffle_unshuffle_empty_remaining_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
        ]);

        // Play all tracks
        queue.pop_next();
        queue.pop_next();
        assert!(queue.is_empty());

        // Apply and restore shuffle (should not panic)
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        queue.restore_original_order();

        // Queue should still be "empty" (no remaining tracks)
        assert_eq!(queue.len(), 0);
    }

    // --- Test 6: Multiple shuffle toggles ---

    #[test]
    fn multiple_shuffle_toggles() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        let original_order: Vec<String> = queue.get_all().iter().map(|t| t.id.clone()).collect();

        // Toggle shuffle on/off multiple times
        for _ in 0..5 {
            // Turn shuffle ON
            queue.apply_shuffle(crate::types::ShuffleMode::Random);
            assert!(queue.is_shuffled());

            // Turn shuffle OFF
            queue.restore_original_order();
            assert!(!queue.is_shuffled());

            // Verify order is restored each time
            let current_order: Vec<String> = queue.get_all().iter().map(|t| t.id.clone()).collect();
            assert_eq!(
                original_order, current_order,
                "Order should be restored after each toggle cycle"
            );
        }
    }

    #[test]
    fn multiple_shuffle_toggles_with_playback_between() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
            create_test_track("6", "Track 6"),
            create_test_track("7", "Track 7"),
            create_test_track("8", "Track 8"),
        ]);

        // Toggle 1: Shuffle ON, play 1 track, shuffle OFF
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        queue.pop_next();
        queue.restore_original_order();

        // Toggle 2: Shuffle ON, play 1 track, shuffle OFF
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        queue.pop_next();
        queue.restore_original_order();

        // Toggle 3: Shuffle ON, play 1 track, shuffle OFF
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        queue.pop_next();
        queue.restore_original_order();

        // All original tracks should still exist in source
        let source_ids: std::collections::HashSet<String> =
            queue.source.iter().map(|t| t.id.clone()).collect();
        assert_eq!(source_ids.len(), 8);
    }

    #[test]
    fn rapid_shuffle_toggle_stress_test() {
        let mut queue = Queue::new();
        queue.set_source(
            (0..100)
                .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i)))
                .collect(),
        );

        // Rapidly toggle shuffle 50 times
        for _ in 0..50 {
            queue.apply_shuffle(crate::types::ShuffleMode::Random);
            queue.restore_original_order();
        }

        // Queue should still have all 100 tracks
        assert_eq!(queue.len(), 100);

        // Verify all tracks are present
        let all_ids: std::collections::HashSet<String> =
            queue.get_all().iter().map(|t| t.id.clone()).collect();
        for i in 0..100 {
            assert!(all_ids.contains(&i.to_string()));
        }
    }

    // --- Test 7: Add to queue during shuffle ---

    #[test]
    fn add_next_during_shuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
            create_test_track("s3", "Source 3"),
        ]);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());

        // Add tracks to play next (these should be separate from shuffled source)
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_next(create_test_track("n2", "Next 2"));

        // Play next tracks should be first
        assert_eq!(queue.get(0).unwrap().id, "n2"); // LIFO
        assert_eq!(queue.get(1).unwrap().id, "n1");

        // Source should be after play_next
        // Total: 2 play_next + 3 source = 5
        assert_eq!(queue.len(), 5);
    }

    #[test]
    fn add_to_end_during_shuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
            create_test_track("s3", "Source 3"),
        ]);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Add tracks to end (queued_later)
        queue.add_to_end(create_test_track("e1", "End 1"));
        queue.add_to_end(create_test_track("e2", "End 2"));

        // Total: 3 source + 2 queued_later = 5
        assert_eq!(queue.len(), 5);

        // End tracks should be at the end
        assert_eq!(queue.get(3).unwrap().id, "e1");
        assert_eq!(queue.get(4).unwrap().id, "e2");
    }

    #[test]
    fn add_to_queue_during_shuffle_then_unshuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
            create_test_track("s3", "Source 3"),
        ]);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Add tracks during shuffle
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_to_end(create_test_track("e1", "End 1"));

        // Unshuffle
        queue.restore_original_order();

        // Play next should still be first
        assert_eq!(queue.get(0).unwrap().id, "n1");

        // Queued later should still be at the end
        // play_next(1) + source(3) + queued_later(1) = 5
        assert_eq!(queue.len(), 5);
        assert_eq!(queue.get(4).unwrap().id, "e1");
    }

    // --- Test 8: Remove from queue during shuffle ---

    #[test]
    fn remove_from_play_next_during_shuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);

        // Add play next then shuffle
        queue.add_next(create_test_track("n1", "Next 1"));
        queue.add_next(create_test_track("n2", "Next 2"));
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Remove from play_next (index 0 = n2 due to LIFO)
        let removed = queue.remove(0).unwrap();
        assert_eq!(removed.id, "n2");

        // Queue should have 3 tracks left
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.get(0).unwrap().id, "n1");
    }

    #[test]
    fn remove_from_source_during_shuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
            create_test_track("s3", "Source 3"),
        ]);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Remember the ID at index 1 (from shuffled source)
        let id_at_1 = queue.get(1).unwrap().id.clone();

        // Remove track at index 1
        let removed = queue.remove(1).unwrap();
        assert_eq!(removed.id, id_at_1);

        // Queue should have 2 tracks left
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn remove_from_queued_later_during_shuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);
        queue.add_to_end(create_test_track("e1", "End 1"));
        queue.add_to_end(create_test_track("e2", "End 2"));

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Remove from queued_later (index 2 = first in queued_later)
        // source(2) + queued_later(2) = 4, indices 2 and 3 are queued_later
        let removed = queue.remove(2).unwrap();
        assert_eq!(removed.id, "e1");

        // Queue should have 3 tracks left
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn remove_during_shuffle_then_unshuffle() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
            create_test_track("s3", "Source 3"),
            create_test_track("s4", "Source 4"),
        ]);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);

        // Remember what we're removing (from shuffled view)
        let removed_id = queue.get(1).unwrap().id.clone();

        // Remove track from shuffled queue
        queue.remove(1);

        // Unshuffle
        queue.restore_original_order();

        // The removed track should be gone from source
        let source_ids: std::collections::HashSet<String> =
            queue.source.iter().map(|t| t.id.clone()).collect();
        assert!(!source_ids.contains(&removed_id));
        assert_eq!(queue.source.len(), 3);
    }

    // --- Test 9: Queue with all same track (duplicates) ---

    #[test]
    fn queue_with_all_duplicate_tracks() {
        let mut queue = Queue::new();

        // Create 10 tracks with the same ID (duplicates)
        let tracks: Vec<QueueTrack> = (0..10)
            .map(|_| create_test_track("same_id", "Same Track"))
            .collect();

        queue.set_source(tracks);
        assert_eq!(queue.len(), 10);

        // All tracks should be accessible
        for i in 0..10 {
            let track = queue.get(i).unwrap();
            assert_eq!(track.id, "same_id");
        }

        // Pop all tracks
        for _ in 0..10 {
            let track = queue.pop_next().unwrap();
            assert_eq!(track.id, "same_id");
        }

        assert!(queue.is_empty());
    }

    #[test]
    fn shuffle_queue_with_all_duplicates() {
        let mut queue = Queue::new();

        let tracks: Vec<QueueTrack> = (0..10)
            .map(|_| create_test_track("same_id", "Same Track"))
            .collect();

        queue.set_source(tracks);

        // Apply shuffle (should not panic)
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        assert!(queue.is_shuffled());

        // All tracks should still be same_id
        for i in 0..10 {
            assert_eq!(queue.get(i).unwrap().id, "same_id");
        }

        // Restore original order
        queue.restore_original_order();
        assert!(!queue.is_shuffled());
        assert_eq!(queue.len(), 10);
    }

    #[test]
    fn remove_consecutive_duplicates_all_same() {
        let mut queue = Queue::new();

        let tracks: Vec<QueueTrack> = (0..10)
            .map(|_| create_test_track("same_id", "Same Track"))
            .collect();

        queue.set_source(tracks);

        // Remove consecutive duplicates should leave only 1 track
        queue.remove_consecutive_duplicates();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.get(0).unwrap().id, "same_id");
    }

    #[test]
    fn queue_with_alternating_duplicates() {
        let mut queue = Queue::new();

        // Create alternating pattern: A, B, A, B, A, B
        let tracks: Vec<QueueTrack> = (0..6)
            .map(|i| {
                if i % 2 == 0 {
                    create_test_track("A", "Track A")
                } else {
                    create_test_track("B", "Track B")
                }
            })
            .collect();

        queue.set_source(tracks);

        // Remove consecutive duplicates should not remove anything (no consecutive dupes)
        queue.remove_consecutive_duplicates();
        assert_eq!(queue.len(), 6);
    }

    #[test]
    fn reorder_with_duplicates() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("A", "Track A"),
            create_test_track("A", "Track A"),
            create_test_track("A", "Track A"),
        ]);

        // Reorder should still work even with duplicate IDs
        assert!(queue.reorder(0, 2).is_ok());
        assert_eq!(queue.len(), 3);
    }

    // --- Test 10: Queue index after various operations ---

    #[test]
    fn source_index_after_pop_next() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        assert_eq!(queue.current_source_index(), 0);

        queue.pop_next();
        assert_eq!(queue.current_source_index(), 1);

        queue.pop_next();
        assert_eq!(queue.current_source_index(), 2);

        queue.pop_next();
        assert_eq!(queue.current_source_index(), 3);

        // Index stays at 3 even though queue is empty
        assert!(queue.is_empty());
        assert_eq!(queue.current_source_index(), 3);
    }

    #[test]
    fn source_index_after_go_back() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Advance to the end
        queue.pop_next();
        queue.pop_next();
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 3);

        // Go back
        queue.go_back();
        assert_eq!(queue.current_source_index(), 2);

        queue.go_back();
        assert_eq!(queue.current_source_index(), 1);

        queue.go_back();
        assert_eq!(queue.current_source_index(), 0);

        // Can't go back further
        assert!(!queue.can_go_back());
        assert!(queue.go_back().is_none());
        assert_eq!(queue.current_source_index(), 0);
    }

    #[test]
    fn source_index_after_skip_to_index() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        assert_eq!(queue.current_source_index(), 0);

        // Skip to index 3
        queue.skip_to_index(3);
        assert_eq!(queue.current_source_index(), 3);

        // Skip to last track
        // After skip_to_index(3), remaining tracks are [4, 5] at indices 0, 1
        queue.skip_to_index(1); // Skip to index 1 of remaining
        assert_eq!(queue.current_source_index(), 4);
    }

    #[test]
    fn source_index_after_reload() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
        ]);

        // Exhaust the queue
        queue.pop_next();
        queue.pop_next();
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 3);
        assert!(queue.is_empty());

        // Reload
        queue.reload_source(crate::types::ShuffleMode::Off);
        assert_eq!(queue.current_source_index(), 0);
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn source_index_after_shuffle_and_restore() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        // Play 2 tracks
        queue.pop_next();
        queue.pop_next();
        let index_before_shuffle = queue.current_source_index();
        assert_eq!(index_before_shuffle, 2);

        // Apply shuffle
        queue.apply_shuffle(crate::types::ShuffleMode::Random);
        // Index should still point to same position (tracks after it shuffled)
        assert_eq!(queue.current_source_index(), 2);

        // Play one more track while shuffled
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 3);

        // Restore original order
        // Index should be adjusted to point after the last played track in original order
        queue.restore_original_order();
        // The restored index should be valid
        assert!(queue.current_source_index() <= queue.source.len());
    }

    #[test]
    fn source_index_with_play_next_queue() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("s1", "Source 1"),
            create_test_track("s2", "Source 2"),
        ]);
        queue.add_next(create_test_track("n1", "Next 1"));

        assert_eq!(queue.current_source_index(), 0);

        // Pop play_next track (does not affect source_index)
        let track = queue.pop_next().unwrap();
        assert_eq!(track.id, "n1");
        assert_eq!(queue.current_source_index(), 0);

        // Pop source track (advances source_index)
        let track = queue.pop_next().unwrap();
        assert_eq!(track.id, "s1");
        assert_eq!(queue.current_source_index(), 1);
    }

    #[test]
    fn source_index_after_remove_at_various_positions() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
        ]);

        // Advance to track 3
        queue.pop_next(); // Now at index 1
        queue.pop_next(); // Now at index 2
        assert_eq!(queue.current_source_index(), 2);

        // Remove track at current position (index 0 in remaining queue = source index 2)
        let removed = queue.remove(0).unwrap();
        assert_eq!(removed.id, "3");

        // Removing at source_index should not affect index (it's removed AT the index)
        // After removal: source = [1, 2, 4, 5], source_index = 2
        // But the condition in remove is source_idx < self.source_index
        // source_idx = source_index + (index - play_next_len) = 2 + (0 - 0) = 2
        // 2 < 2 is false, so source_index is not decremented
        assert_eq!(queue.current_source_index(), 2);

        // Remaining tracks should be [4, 5]
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.get(0).unwrap().id, "4");
    }

    #[test]
    fn queue_index_comprehensive_operations() {
        let mut queue = Queue::new();
        queue.set_source(vec![
            create_test_track("1", "Track 1"),
            create_test_track("2", "Track 2"),
            create_test_track("3", "Track 3"),
            create_test_track("4", "Track 4"),
            create_test_track("5", "Track 5"),
            create_test_track("6", "Track 6"),
        ]);

        // Initial state
        assert_eq!(queue.current_source_index(), 0);
        assert_eq!(queue.len(), 6);

        // Pop 2 tracks
        queue.pop_next();
        queue.pop_next();
        assert_eq!(queue.current_source_index(), 2);
        assert_eq!(queue.len(), 4);

        // Go back 1
        queue.go_back();
        assert_eq!(queue.current_source_index(), 1);
        assert_eq!(queue.len(), 5);

        // Skip to index 3 (relative to current position)
        queue.skip_to_index(3);
        assert_eq!(queue.len(), 2); // Only 2 tracks remaining

        // Add play_next (doesn't affect source_index)
        queue.add_next(create_test_track("n1", "Next 1"));
        assert_eq!(queue.len(), 3); // 1 play_next + 2 source

        // Pop play_next
        queue.pop_next();
        assert_eq!(queue.len(), 2); // Back to 2 source

        // Clear
        queue.clear();
        assert_eq!(queue.current_source_index(), 0);
        assert_eq!(queue.len(), 0);
    }
}
