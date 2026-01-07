//! Playback manager - core orchestration
//!
//! Coordinates queue, history, volume, shuffle, and audio processing

use crate::{
    error::{PlaybackError, Result},
    history::History,
    queue::Queue,
    shuffle::shuffle_queue,
    source::AudioSource,
    types::{PlaybackConfig, PlaybackState, QueueTrack, RepeatMode, ShuffleMode},
    volume::Volume,
};

#[cfg(feature = "effects")]
use soul_audio::effects::EffectChain;

use std::time::Duration;

/// Central playback management
///
/// Orchestrates all playback functionality:
/// - Queue management (two-tier: explicit + source)
/// - History tracking (for "previous" button)
/// - Volume control (logarithmic, 0-100%)
/// - Shuffle modes (Off, Random, Smart)
/// - Repeat modes (Off, All, One)
/// - Audio effects processing
/// - Gapless playback support
pub struct PlaybackManager {
    // State
    state: PlaybackState,
    current_track: Option<QueueTrack>,

    // Queue and history
    queue: Queue,
    history: History,

    // Settings
    volume: Volume,
    shuffle: ShuffleMode,
    repeat: RepeatMode,
    gapless_enabled: bool,

    // Audio processing
    #[cfg(feature = "effects")]
    effect_chain: EffectChain,
    audio_source: Option<Box<dyn AudioSource>>,
    next_source: Option<Box<dyn AudioSource>>, // For gapless

    // Sample rate (for effects processing)
    sample_rate: u32,
}

impl PlaybackManager {
    /// Create new playback manager
    pub fn new(config: PlaybackConfig) -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_track: None,
            queue: Queue::new(),
            history: History::new(config.history_size),
            volume: Volume::new(config.volume),
            shuffle: config.shuffle,
            repeat: config.repeat,
            gapless_enabled: config.gapless,
            #[cfg(feature = "effects")]
            effect_chain: EffectChain::new(),
            audio_source: None,
            next_source: None,
            sample_rate: 44100, // Default, will be updated by platform
        }
    }

    // ===== Playback Control =====

    /// Start or resume playback
    pub fn play(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Paused => {
                // Resume from pause
                self.state = PlaybackState::Playing;
                Ok(())
            }
            PlaybackState::Stopped | PlaybackState::Loading => {
                // Start playing from queue
                self.play_next_in_queue()
            }
            PlaybackState::Playing => {
                // Already playing
                Ok(())
            }
        }
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Stop playback
    ///
    /// Stops playback and clears current track (but not queue)
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_track = None;
        self.audio_source = None;
        self.next_source = None;
    }

    /// Skip to next track
    pub fn next(&mut self) -> Result<()> {
        // Save current track to history (if any)
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        self.play_next_in_queue()
    }

    /// Go to previous track
    ///
    /// If >3 seconds into current track, restarts current track.
    /// Otherwise, pops from history.
    pub fn previous(&mut self) -> Result<()> {
        // Check position in current track
        if let Some(ref source) = self.audio_source {
            if source.position() > Duration::from_secs(3) {
                // Restart current track
                if let Some(ref mut src) = self.audio_source {
                    src.reset()?;
                }
                return Ok(());
            }
        }

        // Go to previous track from history
        if let Some(prev_track) = self.history.pop() {
            // Put current track back in queue (at front)
            if let Some(current) = self.current_track.take() {
                self.queue.add_next(current);
            }

            // Load previous track
            self.current_track = Some(prev_track);
            self.state = PlaybackState::Loading;
            // Platform will need to call load_current_track()
            Ok(())
        } else {
            // No history, restart current track
            if let Some(ref mut source) = self.audio_source {
                source.reset()?;
            }
            Ok(())
        }
    }

    /// Internal: Play next track from queue
    fn play_next_in_queue(&mut self) -> Result<()> {
        // Handle repeat one
        if self.repeat == RepeatMode::One && self.current_track.is_some() {
            // Restart current track
            if let Some(ref mut source) = self.audio_source {
                source.reset()?;
                self.state = PlaybackState::Playing;
                return Ok(());
            }
        }

        // Get next track from queue
        let next_track = self.get_next_track_from_queue()?;

        // Save current track to history
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        // Load next track
        self.current_track = Some(next_track);
        self.state = PlaybackState::Loading;
        // Platform will need to call load_current_track()

        Ok(())
    }

    /// Get next track considering repeat mode
    fn get_next_track_from_queue(&mut self) -> Result<QueueTrack> {
        if let Some(track) = self.queue.pop_next() {
            return Ok(track);
        }

        // Queue empty - check repeat mode
        match self.repeat {
            RepeatMode::All => {
                // TODO: Reload source queue if it was cleared
                // For now, just error
                Err(PlaybackError::QueueEmpty)
            }
            RepeatMode::Off | RepeatMode::One => Err(PlaybackError::QueueEmpty),
        }
    }

    // ===== Seek =====

    /// Seek to position in current track (by duration)
    pub fn seek_to(&mut self, position: Duration) -> Result<()> {
        if let Some(ref mut source) = self.audio_source {
            source.seek(position)?;
            Ok(())
        } else {
            Err(PlaybackError::NoTrackLoaded)
        }
    }

    /// Seek to position in current track (by percentage)
    pub fn seek_to_percent(&mut self, percent: f32) -> Result<()> {
        let percent = percent.clamp(0.0, 1.0);

        if let Some(ref source) = self.audio_source {
            let duration = source.duration();
            let position = duration.mul_f32(percent);
            self.seek_to(position)
        } else {
            Err(PlaybackError::NoTrackLoaded)
        }
    }

    // ===== Volume =====

    /// Set volume (0-100)
    pub fn set_volume(&mut self, level: u8) {
        self.volume.set_level(level);
    }

    /// Get current volume level (0-100)
    pub fn get_volume(&self) -> u8 {
        self.volume.level()
    }

    /// Mute audio
    pub fn mute(&mut self) {
        self.volume.mute();
    }

    /// Unmute audio
    pub fn unmute(&mut self) {
        self.volume.unmute();
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.volume.toggle_mute();
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.volume.is_muted()
    }

    // ===== Queue Management =====

    /// Add track to play next (top of explicit queue)
    pub fn add_to_queue_next(&mut self, track: QueueTrack) {
        self.queue.add_next(track);
    }

    /// Add track to end of explicit queue
    pub fn add_to_queue_end(&mut self, track: QueueTrack) {
        self.queue.add_to_end(track);
    }

    /// Load playlist/album to source queue
    pub fn add_playlist_to_queue(&mut self, mut tracks: Vec<QueueTrack>) {
        // Apply shuffle if enabled
        if self.shuffle != ShuffleMode::Off {
            shuffle_queue(&mut tracks, self.shuffle);
        }

        self.queue.set_source(tracks);
    }

    /// Append tracks to source queue
    pub fn append_to_queue(&mut self, mut tracks: Vec<QueueTrack>) {
        // Apply shuffle if enabled
        if self.shuffle != ShuffleMode::Off {
            shuffle_queue(&mut tracks, self.shuffle);
        }

        self.queue.append_to_source(tracks);
    }

    /// Remove track from queue by index
    pub fn remove_from_queue(&mut self, index: usize) -> Result<QueueTrack> {
        self.queue
            .remove(index)
            .ok_or(PlaybackError::IndexOutOfBounds(index))
    }

    /// Reorder track in queue
    pub fn reorder_queue(&mut self, from: usize, to: usize) -> Result<()> {
        self.queue
            .reorder(from, to)
            .map_err(PlaybackError::InvalidOperation)
    }

    /// Clear entire queue
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Get all tracks in queue
    pub fn get_queue(&self) -> Vec<&QueueTrack> {
        self.queue.get_all()
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    // ===== Shuffle & Repeat =====

    /// Set shuffle mode
    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        if self.shuffle == mode {
            return;
        }

        let old_mode = self.shuffle;
        self.shuffle = mode;

        match mode {
            ShuffleMode::Off => {
                // Restore original order
                self.queue.restore_original_order();
            }
            ShuffleMode::Random | ShuffleMode::Smart => {
                // Apply shuffle to source queue
                if old_mode == ShuffleMode::Off {
                    // Save current order before shuffling
                    self.queue.update_original_source();
                }

                let source = self.queue.source_mut();
                shuffle_queue(source, mode);
                self.queue.set_shuffled(true);
            }
        }
    }

    /// Get current shuffle mode
    pub fn get_shuffle(&self) -> ShuffleMode {
        self.shuffle
    }

    /// Set repeat mode
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Get current repeat mode
    pub fn get_repeat(&self) -> RepeatMode {
        self.repeat
    }

    // ===== State Queries =====

    /// Get current playback state
    pub fn get_state(&self) -> PlaybackState {
        self.state
    }

    /// Get currently playing track
    pub fn get_current_track(&self) -> Option<&QueueTrack> {
        self.current_track.as_ref()
    }

    /// Get current playback position
    pub fn get_position(&self) -> Duration {
        self.audio_source
            .as_ref()
            .map(|s| s.position())
            .unwrap_or(Duration::ZERO)
    }

    /// Get current track duration
    pub fn get_duration(&self) -> Option<Duration> {
        self.audio_source.as_ref().map(|s| s.duration())
    }

    /// Get playback history
    pub fn get_history(&self) -> Vec<&QueueTrack> {
        self.history.get_all()
    }

    /// Get total queue length
    pub fn get_queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Check if there is a next track
    pub fn has_next(&self) -> bool {
        !self.queue.is_empty() || self.repeat == RepeatMode::One
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        !self.history.get_all().is_empty() || self.repeat == RepeatMode::One
    }

    // ===== Audio Processing =====

    /// Process audio samples for output
    ///
    /// Called by platform audio callback. Applies effects and volume.
    /// Returns number of samples written to output buffer.
    ///
    /// # Arguments
    /// * `output` - Output buffer (interleaved stereo f32)
    ///
    /// # Returns
    /// Number of samples written (0 = no audio available)
    pub fn process_audio(&mut self, output: &mut [f32]) -> Result<usize> {
        if self.state != PlaybackState::Playing {
            // Not playing - output silence
            output.fill(0.0);
            return Ok(output.len());
        }

        let Some(ref mut source) = self.audio_source else {
            // No audio source - output silence
            output.fill(0.0);
            return Ok(output.len());
        };

        // Read samples from source
        let samples_read = source.read_samples(output)?;

        if samples_read == 0 {
            // Track finished
            self.handle_track_finished()?;
            return Ok(0);
        }

        // Apply effects (if feature enabled)
        #[cfg(feature = "effects")]
        self.effect_chain.process(output, self.sample_rate);

        // Apply volume
        self.volume.apply(output);

        Ok(samples_read)
    }

    /// Handle track finished
    fn handle_track_finished(&mut self) -> Result<()> {
        // Auto-advance to next track
        self.next()
    }

    /// Set sample rate (called by platform)
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    /// Get effect chain (for adding/configuring effects)
    #[cfg(feature = "effects")]
    pub fn effect_chain_mut(&mut self) -> &mut EffectChain {
        &mut self.effect_chain
    }

    /// Set audio source (called by platform after loading track)
    pub fn set_audio_source(&mut self, source: Box<dyn AudioSource>) {
        self.audio_source = Some(source);
        self.state = PlaybackState::Playing;
    }
}

impl Default for PlaybackManager {
    fn default() -> Self {
        Self::new(PlaybackConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::DummyAudioSource;
    use crate::types::TrackSource;
    use std::path::PathBuf;

    fn create_test_track(id: &str) -> QueueTrack {
        QueueTrack {
            id: id.to_string(),
            path: PathBuf::from(format!("/music/{}.mp3", id)),
            title: format!("Track {}", id),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        }
    }

    #[test]
    fn create_playback_manager() {
        let manager = PlaybackManager::new(PlaybackConfig::default());
        assert_eq!(manager.get_state(), PlaybackState::Stopped);
        assert_eq!(manager.get_volume(), 80);
        assert!(manager.get_queue().is_empty());
    }

    #[test]
    fn set_volume() {
        let mut manager = PlaybackManager::default();

        manager.set_volume(50);
        assert_eq!(manager.get_volume(), 50);

        manager.set_volume(100);
        assert_eq!(manager.get_volume(), 100);
    }

    #[test]
    fn mute_unmute() {
        let mut manager = PlaybackManager::default();

        assert!(!manager.is_muted());

        manager.mute();
        assert!(manager.is_muted());

        manager.unmute();
        assert!(!manager.is_muted());
    }

    #[test]
    fn add_to_queue() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_next(create_test_track("1"));
        manager.add_to_queue_end(create_test_track("2"));

        assert_eq!(manager.queue_len(), 2);
    }

    #[test]
    fn shuffle_modes() {
        let mut manager = PlaybackManager::default();

        // Add some tracks
        manager.add_playlist_to_queue(vec![
            create_test_track("1"),
            create_test_track("2"),
            create_test_track("3"),
        ]);

        assert_eq!(manager.get_shuffle(), ShuffleMode::Off);

        // Enable shuffle
        manager.set_shuffle(ShuffleMode::Random);
        assert_eq!(manager.get_shuffle(), ShuffleMode::Random);

        // Disable shuffle (should restore original order)
        manager.set_shuffle(ShuffleMode::Off);
        assert_eq!(manager.get_shuffle(), ShuffleMode::Off);
    }

    #[test]
    fn repeat_modes() {
        let mut manager = PlaybackManager::default();

        assert_eq!(manager.get_repeat(), RepeatMode::Off);

        manager.set_repeat(RepeatMode::All);
        assert_eq!(manager.get_repeat(), RepeatMode::All);

        manager.set_repeat(RepeatMode::One);
        assert_eq!(manager.get_repeat(), RepeatMode::One);
    }

    #[test]
    fn process_audio_when_stopped() {
        let mut manager = PlaybackManager::default();
        let mut buffer = [1.0f32; 1024];

        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());

        // Should output silence
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1023], 0.0);
    }

    #[test]
    fn set_audio_source_changes_state() {
        let mut manager = PlaybackManager::default();

        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }
}
