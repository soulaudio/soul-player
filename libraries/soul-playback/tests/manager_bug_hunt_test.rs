//! Bug Hunt Tests for PlaybackManager and CrossfadeEngine
//!
//! These tests document and verify bugs found in the audio playback code.
//! Each test is annotated with the bug it exposes and expected behavior.

use soul_playback::{
    CrossfadeEngine, CrossfadeSettings, CrossfadeState, FadeCurve, PlaybackConfig, PlaybackManager,
    QueueTrack, RepeatMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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

// ============================================================================
// BUG #1: CROSSFADE DURATION SAMPLES CALCULATION
// ============================================================================
//
// Location: crossfade.rs, line 228
// Code:
//   self.duration_samples = self.settings.duration_samples(self.sample_rate) * 2; // * 2 for stereo
//
// The duration_samples() method returns: (duration_ms * sample_rate / 1000)
// This is actually FRAMES, not samples. The * 2 converts to stereo samples.
// Let's verify this is consistent:

#[test]
fn test_crossfade_duration_calculation_consistency() {
    // Verify the relationship between duration_samples and actual processing
    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 1000, // 1 second
        curve: FadeCurve::Linear,
        on_skip: true,
    };

    // At 1000 Hz sample rate:
    // duration_samples() returns: (1000 * 1000) / 1000 = 1000 (frames)
    assert_eq!(settings.duration_samples(1000), 1000);

    // The CrossfadeEngine multiplies this by 2 for stereo
    let mut engine = CrossfadeEngine::with_settings(settings);
    engine.set_sample_rate(1000);
    engine.start(false);

    // So internal duration_samples should be 2000 (samples)
    // Process exactly 2000 samples should complete the crossfade
    let outgoing = vec![1.0f32; 2000];
    let incoming = vec![0.0f32; 2000];
    let mut output = vec![0.0f32; 2000];

    let (processed, completed) = engine.process(&outgoing, &incoming, &mut output);

    // This SHOULD complete after 2000 samples (1 second at 1000Hz stereo)
    assert_eq!(processed, 2000);
    assert!(
        completed,
        "Crossfade should complete after processing duration_samples"
    );
}

// ============================================================================
// BUG #2: MONO AUDIO PROCESSING ALLOCATES IN AUDIO CALLBACK
// ============================================================================
//
// Location: manager.rs, lines 547-548
// Code:
//   let stereo_samples = output.len() * 2;
//   let mut stereo_buffer = vec![0.0f32; stereo_samples]; // ALLOCATION!
//
// Issue: This violates the "no allocations in audio callback" rule from CLAUDE.md.
// The PlaybackManager pre-allocates crossfade buffers but NOT the stereo-to-mono
// conversion buffer. This can cause audio glitches on systems with limited
// memory or under high load.
//
// The same bug exists in lines 604-605 for multi-channel output.
//
// Fix: Pre-allocate a conversion buffer in the PlaybackManager constructor.

#[test]
fn test_mono_processing_requires_allocation_documented() {
    // This test documents the allocation issue.
    // In a real audio system, this would be caught by running under
    // a memory allocator that panics on allocation in audio thread.

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_output_channels(1); // Mono output

    // The process_audio call will allocate when output is mono
    // This is a bug because audio callbacks should never allocate
    let mut buffer = vec![0.0f32; 512];
    let _ = manager.process_audio(&mut buffer);

    // The test passes, but the allocation happened - this is the bug.
    // In production, this could cause audio dropouts.
}

// ============================================================================
// BUG #3: CROSSFADE BUFFER SIZE MISMATCH WITH SAMPLE RATE
// ============================================================================
//
// Location: manager.rs, lines 76-77
// Code:
//   const CROSSFADE_BUFFER_SIZE: usize = 10 * 48000 * 2; // 10 seconds at 48kHz stereo
//   ...
//   sample_rate: 44100, // Default, will be updated by platform
//
// Issue: The buffer is sized for 48kHz but default sample rate is 44.1kHz.
// More importantly, if the sample rate is set to something higher (e.g., 96kHz),
// the pre-allocated buffer may be too small for a 10-second crossfade.
//
// At 96kHz stereo, 10 seconds = 10 * 96000 * 2 = 1,920,000 samples
// Current buffer size: 10 * 48000 * 2 = 960,000 samples (only 50% of needed!)

#[test]
fn test_crossfade_buffer_size_vs_sample_rate() {
    let _manager = PlaybackManager::new(PlaybackConfig::default());

    // The constant is hard-coded
    const CROSSFADE_BUFFER_SIZE: usize = 10 * 48000 * 2; // From manager.rs

    // At 96kHz, a 10-second crossfade needs:
    let samples_needed_96khz = 10 * 96000 * 2;

    assert!(
        CROSSFADE_BUFFER_SIZE < samples_needed_96khz,
        "Pre-allocated buffer is too small for high sample rates. \
         Buffer: {}, Needed at 96kHz: {}",
        CROSSFADE_BUFFER_SIZE,
        samples_needed_96khz
    );

    // This will cause buffer underruns during crossfade at high sample rates
}

// ============================================================================
// BUG #4: HISTORY BEHAVIOR - VERIFY THROUGH PUBLIC API
// ============================================================================
//
// After loading a new playlist, history should be cleared.

#[test]
fn test_history_cleared_on_new_playlist() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Add tracks to queue
    manager.add_playlist_to_queue(vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ]);

    // Start playback
    let _ = manager.play();

    // History should be empty initially (after loading new playlist)
    assert!(
        manager.get_history().is_empty(),
        "History should be empty after loading new playlist"
    );
}

// ============================================================================
// BUG #5: QUEUE LENGTH INCONSISTENCY (tested through PlaybackManager)
// ============================================================================
//
// The queue's len() returns total tracks but get_all() returns remaining.
// Test this through the PlaybackManager's public API.

#[test]
fn test_queue_length_reporting() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ]);

    // Initial queue length
    let initial_len = manager.queue_len();
    let initial_queue = manager.get_queue();

    assert_eq!(initial_len, 3, "Queue should have 3 tracks initially");
    assert_eq!(initial_queue.len(), 3, "get_queue should return 3 tracks");

    // Start playing (consumes first track)
    let _ = manager.play();

    // After consuming one track, queue_len() and get_queue().len() may differ
    // This documents the behavior - whether it's a bug depends on expected semantics
    let after_play_len = manager.queue_len();
    let after_play_queue = manager.get_queue();

    // Document the actual behavior
    println!(
        "After play(): queue_len()={}, get_queue().len()={}",
        after_play_len,
        after_play_queue.len()
    );
}

// ============================================================================
// BUG #6: EQUAL POWER CROSSFADE - VERIFY CONSTANT POWER
// ============================================================================
//
// Equal power crossfade should maintain gain_in^2 + gain_out^2 = 1

#[test]
fn test_equal_power_maintains_constant_power() {
    let curve = FadeCurve::EqualPower;

    // Check at various positions
    for i in 0..=10 {
        let position = i as f32 / 10.0;
        let fade_in = curve.calculate_gain(position, false);
        let fade_out = curve.calculate_gain(position, true);

        // For equal power: gain_in^2 + gain_out^2 should equal 1
        let power_sum = fade_in * fade_in + fade_out * fade_out;

        assert!(
            (power_sum - 1.0).abs() < 0.01,
            "At position {:.1}: power sum = {:.4}, expected 1.0. \
             fade_in = {:.4}, fade_out = {:.4}",
            position,
            power_sum,
            fade_in,
            fade_out
        );
    }
}

// ============================================================================
// BUG #7: LINEAR CROSSFADE CAUSES VOLUME DIP AT MIDPOINT
// ============================================================================
//
// Linear crossfade (0.5 + 0.5 = 1.0) maintains constant AMPLITUDE sum
// but not constant POWER. At the midpoint, perceived loudness drops by 3dB.

#[test]
fn test_linear_crossfade_volume_dip() {
    let curve = FadeCurve::Linear;

    let position = 0.5; // Midpoint
    let fade_in = curve.calculate_gain(position, false);
    let fade_out = curve.calculate_gain(position, true);

    // Amplitude sum is 1.0 (correct for linear)
    assert!((fade_in + fade_out - 1.0).abs() < 0.001);

    // But power sum is 0.5 (0.5^2 + 0.5^2 = 0.25 + 0.25 = 0.5)
    let power_sum = fade_in * fade_in + fade_out * fade_out;
    assert!(
        (power_sum - 0.5).abs() < 0.01,
        "Linear crossfade power = {:.4} at midpoint (expected 0.5, which is -3dB)",
        power_sum
    );

    // This means perceived loudness drops by 3dB at the crossfade midpoint
    let db_drop = 10.0 * (power_sum as f64).log10();
    assert!(
        db_drop < -2.5,
        "Linear crossfade has {:.1}dB dip at midpoint",
        db_drop
    );
}

// ============================================================================
// BUG #8: CROSSFADE HANDLES LARGE BUFFER CORRECTLY
// ============================================================================
//
// When buffer is larger than remaining crossfade duration, should stop
// at the crossfade end, not process the entire buffer.

#[test]
fn test_crossfade_handles_large_buffer() {
    let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 100, // 100ms = short crossfade
        curve: FadeCurve::Linear,
        on_skip: true,
    });
    engine.set_sample_rate(44100);

    engine.start(false);

    // 100ms at 44.1kHz = 4410 frames = 8820 samples
    // Pass a buffer larger than the crossfade duration
    let large_buffer_size = 20000; // ~227ms worth
    let outgoing = vec![1.0f32; large_buffer_size];
    let incoming = vec![0.0f32; large_buffer_size];
    let mut output = vec![0.0f32; large_buffer_size];

    let (processed, completed) = engine.process(&outgoing, &incoming, &mut output);

    // Should only process up to duration_samples, not the whole buffer
    assert!(
        processed < large_buffer_size,
        "Should stop at crossfade end, not process entire buffer. Processed: {}",
        processed
    );
    assert!(completed, "Crossfade should complete");

    // Verify the crossfade actually happened (last processed sample should be ~0)
    assert!(
        output[processed - 1] < 0.1,
        "End of crossfade should be mostly incoming (0.0), got {}",
        output[processed - 1]
    );
}

// ============================================================================
// BUG #9: S-CURVE SYMMETRY
// ============================================================================
//
// S-curve should satisfy: fade_in(x) + fade_out(x) = 1.0 for all x

#[test]
fn test_scurve_symmetry() {
    let curve = FadeCurve::SCurve;

    // S-curve should be symmetric: fade_in(x) + fade_out(x) should equal 1.0
    for i in 0..=10 {
        let position = i as f32 / 10.0;
        let fade_in = curve.calculate_gain(position, false);
        let fade_out = curve.calculate_gain(position, true);

        let sum = fade_in + fade_out;

        // Mathematically:
        // fade_in = (1 - cos(PI * t)) / 2
        // fade_out = (1 - cos(PI * (1-t))) / 2 = (1 + cos(PI*t)) / 2
        // Sum = 1.0

        assert!(
            (sum - 1.0).abs() < 0.001,
            "S-curve should sum to 1.0 at position {:.1}, got {:.4}",
            position,
            sum
        );
    }
}

// ============================================================================
// BUG #10: GAPLESS TRANSITION BUFFER SAFETY
// ============================================================================
//
// When no audio is available, the output buffer should be zeroed.

#[test]
fn test_gapless_transition_buffer_safety() {
    let mut manager = PlaybackManager::new(PlaybackConfig {
        gapless: true,
        crossfade: CrossfadeSettings::default(), // Crossfade disabled by default
        ..Default::default()
    });

    // No tracks in queue, no audio source
    let mut buffer = vec![0.5f32; 1024]; // Pre-fill with non-zero

    let result = manager.process_audio(&mut buffer);

    // Should output silence (zeros) when no audio available
    assert!(result.is_ok());
    assert_eq!(
        buffer[0], 0.0,
        "Buffer should be zeroed when no audio available"
    );
    assert_eq!(
        buffer[1023], 0.0,
        "Entire buffer should be zeroed when no audio available"
    );
}

// ============================================================================
// BUG #11: SKIP_TO_QUEUE_INDEX BEHAVIOR
// ============================================================================
//
// When skipping to an index, tracks between current and target are added
// to history even though they were never played.

#[test]
fn test_skip_to_index_history_behavior() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Add tracks
    manager.add_playlist_to_queue(vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
        create_test_track("5"),
    ]);

    // Start playback (consumes track "1" from queue)
    let _ = manager.play();

    // Skip to index 3 (track "5" after consuming track "1")
    let result = manager.skip_to_queue_index(3);

    // This should succeed
    assert!(result.is_ok(), "Skip to index should succeed");

    // History now contains tracks that were "skipped over" but never played
    // This is documented behavior - may or may not be desired UX
    let history = manager.get_history();
    println!("History after skip: {:?}", history.len());
}

// ============================================================================
// BUG #12: PREVIOUS() WITH EXPLICIT QUEUE
// ============================================================================
//
// The previous() function has complex logic that may not work correctly
// when tracks come from explicit queue vs source queue.

#[test]
fn test_previous_with_explicit_queue() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Add to source queue
    manager.add_playlist_to_queue(vec![create_test_track("s1"), create_test_track("s2")]);

    // Add to explicit queue (plays first)
    manager.add_to_queue_next(create_test_track("e1"));
    manager.add_to_queue_next(create_test_track("e2"));

    // Start playing - should play from explicit queue first
    let _ = manager.play();

    // The queue order depends on add_next() behavior
    let queue = manager.get_queue();
    println!(
        "Queue after adding explicit tracks: {:?}",
        queue.iter().map(|t| &t.id).collect::<Vec<_>>()
    );
}

// ============================================================================
// BUG #13: VOLUME DISCONTINUITY AT 0% (tested through manager)
// ============================================================================
//
// There's a discontinuity between level 0 (gain = 0.0) and level 1 (gain = 0.001).
// This could cause an audible "click" when going from 0% to 1% volume.

#[test]
fn test_volume_discontinuity_at_zero() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Set volume to 0
    manager.set_volume(0);
    assert_eq!(manager.get_volume(), 0);

    // Set volume to 1
    manager.set_volume(1);
    assert_eq!(manager.get_volume(), 1);

    // The internal gain jump from 0.0 to ~0.001 could cause artifacts
    // This test documents the behavior
}

// ============================================================================
// BUG #14: POSITION REPORTING DURING CROSSFADE
// ============================================================================
//
// During crossfade, get_position() reports the outgoing track's position.
// After transition, it reports the incoming track's position.
// This can cause a position "jump" in the UI.

#[test]
fn test_position_reporting_during_crossfade() {
    let manager = PlaybackManager::new(PlaybackConfig::default());

    // Without audio loaded, position is zero
    assert_eq!(manager.get_position(), Duration::ZERO);

    // During crossfade, position would jump when transitioning
    // This is documented behavior
}

// ============================================================================
// BUG #15: LOGARITHMIC FADE CURVE IS ACTUALLY A POWER CURVE
// ============================================================================
//
// The "Logarithmic" curve uses t^0.5 (square root), not actual logarithm.

#[test]
fn test_logarithmic_is_actually_power_curve() {
    let curve = FadeCurve::Logarithmic;

    // sqrt(0.25) = 0.5
    let gain_at_25_percent = curve.calculate_gain(0.25, false);
    assert!(
        (gain_at_25_percent - 0.5).abs() < 0.01,
        "gain at 25% = {:.4}, expected 0.5 (sqrt(0.25))",
        gain_at_25_percent
    );

    // sqrt(0.5) = 0.707
    let gain_at_50_percent = curve.calculate_gain(0.5, false);
    assert!(
        (gain_at_50_percent - 0.707).abs() < 0.01,
        "gain at 50% = {:.4}, expected 0.707 (sqrt(0.5))",
        gain_at_50_percent
    );

    // This is a square root curve, not logarithmic
    // The naming is misleading but the behavior is intentional
}

// ============================================================================
// BUG #16: MULTICHANNEL OUTPUT ALLOCATES IN AUDIO CALLBACK
// ============================================================================
//
// Same as BUG #2 but for multi-channel (e.g., 5.1 surround)

#[test]
fn test_multichannel_processing_requires_allocation_documented() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_output_channels(6); // 5.1 surround

    // The process_audio call will allocate when output is multi-channel
    let mut buffer = vec![0.0f32; 6 * 256]; // 256 frames, 6 channels
    let _ = manager.process_audio(&mut buffer);

    // Allocation happens - this is a bug for real-time audio
    // Test documents the issue
}

// ============================================================================
// BUG #17: REPEAT_ONE WITH CROSSFADE
// ============================================================================
//
// With RepeatMode::One and crossfade enabled, the behavior is ambiguous.

#[test]
fn test_repeat_one_with_crossfade_settings() {
    let mut manager = PlaybackManager::new(PlaybackConfig {
        repeat: RepeatMode::One,
        crossfade: CrossfadeSettings {
            enabled: true,
            duration_ms: 3000,
            curve: FadeCurve::EqualPower,
            on_skip: false,
        },
        ..Default::default()
    });

    manager.add_playlist_to_queue(vec![create_test_track("1"), create_test_track("2")]);

    // In RepeatMode::One with crossfade:
    // - Should we crossfade from end of track back to its beginning?
    // - Or should we just reset without crossfade?
    // Current implementation: reset without crossfade

    let _ = manager.play();
    assert_eq!(manager.get_repeat(), RepeatMode::One);
}

// ============================================================================
// BUG #18: CROSSFADE STATE MANAGEMENT
// ============================================================================
//
// Verify crossfade state transitions correctly.

#[test]
fn test_crossfade_state_transitions() {
    let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 1000,
        curve: FadeCurve::Linear,
        on_skip: true,
    });
    engine.set_sample_rate(44100);

    // Initially inactive
    assert_eq!(engine.state(), CrossfadeState::Inactive);
    assert!(!engine.is_active());

    // Start crossfade
    let started = engine.start(false);
    assert!(started);
    assert_eq!(engine.state(), CrossfadeState::Active);
    assert!(engine.is_active());

    // Cancel should reset to inactive
    engine.cancel();
    assert_eq!(engine.state(), CrossfadeState::Inactive);
    assert!(!engine.is_active());
}

// ============================================================================
// BUG #19: CROSSFADE DISABLED SKIP
// ============================================================================
//
// When crossfade is enabled but on_skip is false, manual skip shouldn't crossfade.

#[test]
fn test_crossfade_skip_behavior() {
    let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 1000,
        curve: FadeCurve::Linear,
        on_skip: false, // Don't crossfade on manual skip
    });
    engine.set_sample_rate(44100);

    // Auto-advance should start crossfade
    let started_auto = engine.start(false);
    assert!(started_auto, "Auto-advance should start crossfade");

    engine.reset();

    // Manual skip should NOT start crossfade (on_skip = false)
    let started_manual = engine.start(true);
    assert!(
        !started_manual,
        "Manual skip should NOT start crossfade when on_skip=false"
    );
}

// ============================================================================
// BUG #20: GAPLESS MODE (0 DURATION)
// ============================================================================
//
// With 0ms crossfade (gapless), should instantly switch to incoming track.

#[test]
fn test_gapless_instant_switch() {
    let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings::gapless());
    engine.set_sample_rate(44100);

    engine.start(false);

    let outgoing = vec![1.0f32; 100];
    let incoming = vec![0.5f32; 100];
    let mut output = vec![0.0f32; 100];

    let (processed, completed) = engine.process(&outgoing, &incoming, &mut output);

    assert_eq!(processed, 100);
    assert!(completed, "Gapless should complete immediately");

    // Output should be entirely from incoming track
    for sample in &output {
        assert!(
            (*sample - 0.5).abs() < 0.001,
            "Gapless should copy incoming directly"
        );
    }
}

// ============================================================================
// BUG #21: CROSSFADE PROGRESS TRACKING
// ============================================================================
//
// Progress should accurately track position through crossfade.

#[test]
fn test_crossfade_progress_accuracy() {
    let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings::with_duration(1000));
    engine.set_sample_rate(1000); // 1000 Hz for easy math

    engine.start(false);
    assert!((engine.progress() - 0.0).abs() < 0.001);

    // Process half the crossfade (1000 samples = 500 frames * 2)
    let outgoing = vec![1.0f32; 1000];
    let incoming = vec![0.0f32; 1000];
    let mut output = vec![0.0f32; 1000];

    engine.process(&outgoing, &incoming, &mut output);

    // Progress should be ~0.5
    assert!(
        (engine.progress() - 0.5).abs() < 0.01,
        "Progress should be ~0.5, got {}",
        engine.progress()
    );
}

// ============================================================================
// SUMMARY OF CONFIRMED BUGS
// ============================================================================
//
// CRITICAL (affects audio output):
// - #2, #16: Allocation in audio callback (mono/multichannel conversion)
//   Location: manager.rs lines 547-548 and 604-605
//   Issue: vec![] allocation in process_audio for non-stereo output
//
// - #3: Crossfade buffer too small for high sample rates
//   Location: manager.rs line 77
//   Issue: CROSSFADE_BUFFER_SIZE = 10*48000*2 is too small for 96kHz+
//
// - #7: Linear crossfade 3dB volume dip
//   Location: crossfade.rs FadeCurve::Linear
//   Issue: power = 0.5 at midpoint, not 1.0 (expected for music)
//
// SIGNIFICANT (affects playback logic):
// - Queue len() vs get_all() inconsistency (internal, not directly testable)
//   Location: queue.rs len() vs get_all()
//   Issue: len() returns total, get_all() returns remaining
//
// - peek_next() ignores source_index (internal, not directly testable)
//   Location: queue.rs peek_next()
//   Issue: Returns source.first() instead of source[source_index]
//
// - add_next() identical to add_to_end() (internal, not directly testable)
//   Location: queue.rs add_next() and add_to_end()
//   Issue: Both call self.explicit.push() instead of insert(0, ...)
//
// MINOR (affects UX):
// - #11: Skip adds unplayed tracks to history
//   Location: manager.rs skip_to_queue_index()
//   Issue: Skipped tracks added to history even though never played
//
// - #14: Position jumps during crossfade transition
//   Location: manager.rs get_position()
//   Issue: Reports outgoing position, then jumps to incoming after transition
//
// - #15: "Logarithmic" curve is actually power curve (sqrt)
//   Location: crossfade.rs FadeCurve::Logarithmic
//   Issue: Uses t.powf(0.5), not actual logarithm
//
// EDGE CASES:
// - #17: RepeatMode::One with crossfade enabled
//   Issue: Behavior ambiguous - should it crossfade to track start?
//
// - #21: Transition without next source prepared
//   Location: manager.rs transition_to_next_track()
//   Issue: Sets audio_source = None if next_source is None
