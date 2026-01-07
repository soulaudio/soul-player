//! Playback history tracking
//!
//! Maintains a bounded history of played tracks for "previous" functionality

use crate::types::QueueTrack;
use std::collections::VecDeque;

/// Playback history with bounded size
///
/// Tracks recently played songs for "previous" navigation.
/// Implements a ring buffer that automatically discards oldest entries.
#[derive(Debug, Clone)]
pub struct History {
    /// History buffer (most recent = back)
    tracks: VecDeque<QueueTrack>,

    /// Maximum history size
    max_size: usize,
}

impl History {
    /// Create new history with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            tracks: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Add track to history
    ///
    /// If history is full, oldest track is discarded
    pub fn push(&mut self, track: QueueTrack) {
        if self.tracks.len() >= self.max_size {
            self.tracks.pop_front(); // Remove oldest
        }
        self.tracks.push_back(track);
    }

    /// Get most recent track (without removing)
    #[allow(dead_code)]
    pub fn peek(&self) -> Option<&QueueTrack> {
        self.tracks.back()
    }

    /// Pop most recent track from history
    ///
    /// Returns the track for "previous" functionality
    pub fn pop(&mut self) -> Option<QueueTrack> {
        self.tracks.pop_back()
    }

    /// Get all history tracks (oldest first)
    pub fn get_all(&self) -> Vec<&QueueTrack> {
        self.tracks.iter().collect()
    }

    /// Get number of tracks in history
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Check if history is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Clear all history
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    /// Get maximum history size
    #[allow(dead_code)]
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Set maximum history size
    ///
    /// If new size is smaller than current, oldest entries are discarded
    #[allow(dead_code)]
    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;

        // Trim if needed
        while self.tracks.len() > max_size {
            self.tracks.pop_front();
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(50) // Default: 50 tracks
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
    fn create_history() {
        let history = History::new(10);
        assert_eq!(history.max_size(), 10);
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }

    #[test]
    fn push_to_history() {
        let mut history = History::new(10);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));

        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
    }

    #[test]
    fn peek_most_recent() {
        let mut history = History::new(10);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));

        let recent = history.peek().unwrap();
        assert_eq!(recent.id, "2");

        // Still there
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn pop_from_history() {
        let mut history = History::new(10);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));
        history.push(create_test_track("3", "Track 3"));

        // Pop most recent
        let track = history.pop().unwrap();
        assert_eq!(track.id, "3");
        assert_eq!(history.len(), 2);

        // Pop again
        let track = history.pop().unwrap();
        assert_eq!(track.id, "2");
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn history_bounded() {
        let mut history = History::new(3); // Max 3 tracks

        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));
        history.push(create_test_track("3", "Track 3"));
        assert_eq!(history.len(), 3);

        // Add 4th track - oldest should be discarded
        history.push(create_test_track("4", "Track 4"));
        assert_eq!(history.len(), 3);

        // Oldest (Track 1) should be gone
        let all = history.get_all();
        assert_eq!(all[0].id, "2"); // Track 1 discarded
        assert_eq!(all[1].id, "3");
        assert_eq!(all[2].id, "4");
    }

    #[test]
    fn get_all_ordered() {
        let mut history = History::new(10);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));
        history.push(create_test_track("3", "Track 3"));

        let all = history.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "1"); // Oldest
        assert_eq!(all[1].id, "2");
        assert_eq!(all[2].id, "3"); // Most recent
    }

    #[test]
    fn clear_history() {
        let mut history = History::new(10);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));

        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn resize_history() {
        let mut history = History::new(5);
        for i in 1..=5 {
            history.push(create_test_track(&i.to_string(), &format!("Track {}", i)));
        }
        assert_eq!(history.len(), 5);

        // Reduce size to 3
        history.set_max_size(3);
        assert_eq!(history.len(), 3);
        assert_eq!(history.max_size(), 3);

        // Oldest 2 tracks should be gone
        let all = history.get_all();
        assert_eq!(all[0].id, "3"); // Tracks 1 and 2 discarded
        assert_eq!(all[1].id, "4");
        assert_eq!(all[2].id, "5");
    }

    #[test]
    fn increase_max_size() {
        let mut history = History::new(3);
        history.push(create_test_track("1", "Track 1"));
        history.push(create_test_track("2", "Track 2"));

        history.set_max_size(10);
        assert_eq!(history.max_size(), 10);
        assert_eq!(history.len(), 2); // Existing tracks preserved
    }

    #[test]
    fn default_history() {
        let history = History::default();
        assert_eq!(history.max_size(), 50);
    }
}
