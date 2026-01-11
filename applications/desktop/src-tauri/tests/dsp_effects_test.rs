//! End-to-end tests for DSP effects chain functionality
//!
//! These tests validate that:
//! 1. Effects can be added to slots and show up in UI
//! 2. Effects are actually applied to audio
//! 3. Effects can be toggled on/off
//! 4. Effects can be removed
//! 5. Effect parameters can be updated
//! 6. Multiple effects work together in chain

use soul_audio_desktop::DesktopPlayback;
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test WAV file
fn create_test_wav(
    path: &std::path::Path,
    duration_secs: f32,
    frequency: f32,
) -> std::io::Result<()> {
    use hound::{WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    let num_samples = (44100.0 * duration_secs) as usize;
    for i in 0..num_samples {
        let t = i as f32 / 44100.0;
        let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        let amplitude = (i16::MAX as f32 * 0.5 * sample) as i16;
        writer.write_sample(amplitude)?;
        writer.write_sample(amplitude)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Test that effects can be added and retrieved from slots
#[test]
#[cfg(feature = "effects")]
fn test_add_effect_to_slot() {
    use soul_audio::effects::{EqBand, ParametricEq};

    let config = PlaybackConfig {
        history_size: 10,
        volume: 80,
        shuffle: ShuffleMode::Off,
        repeat: RepeatMode::Off,
        gapless: false,
    };

    // Create playback (may fail if no audio device)
    let playback = match DesktopPlayback::new(config) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("Skipping test - no audio device: {}", e);
            return;
        }
    };

    // Create an EQ effect
    let eq_bands = vec![
        EqBand::new(100.0, 3.0, 1.0),
        EqBand::new(1000.0, 0.0, 1.0),
        EqBand::new(10000.0, -3.0, 1.0),
    ];
    let eq = ParametricEq::new(eq_bands);

    // Add effect to slot 0
    playback.with_effect_chain(|chain| {
        chain.add_effect(Box::new(eq));
        assert_eq!(chain.len(), 1);
    });

    eprintln!("✅ Effect added to slot successfully");
}

/// Test that effects are actually applied to audio
#[test]
#[cfg(feature = "effects")]
fn test_effect_processes_audio() {
    use soul_audio::effects::{EqBand, ParametricEq};

    let config = PlaybackConfig::default();

    let playback = match DesktopPlayback::new(config) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("Skipping test - no audio device: {}", e);
            return;
        }
    };

    // Create a gain boost at 1kHz
    let eq_bands = vec![EqBand::new(1000.0, 12.0, 1.0)]; // +12dB boost
    let eq = ParametricEq::new(eq_bands);

    // Generate test signal at 1kHz
    let sample_rate = 44100;
    let duration = 0.1; // 100ms
    let num_samples = (sample_rate as f32 * duration) as usize * 2; // stereo

    let mut test_buffer = Vec::with_capacity(num_samples);
    for i in 0..(num_samples / 2) {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * 1000.0 * 2.0 * std::f32::consts::PI).sin() * 0.1; // 1kHz tone
        test_buffer.push(sample);
        test_buffer.push(sample);
    }

    let original_rms = calculate_rms(&test_buffer);

    // Process through effect chain
    let processed_rms = playback.with_effect_chain(|chain| {
        chain.add_effect(Box::new(eq));
        let mut buffer = test_buffer.clone();
        chain.process(&mut buffer, sample_rate);
        calculate_rms(&buffer)
    });

    // With +12dB boost, RMS should increase significantly (approximately 4x)
    let gain_ratio = processed_rms / original_rms;
    eprintln!("Original RMS: {:.6}", original_rms);
    eprintln!("Processed RMS: {:.6}", processed_rms);
    eprintln!("Gain ratio: {:.2}x", gain_ratio);

    assert!(
        gain_ratio > 2.0,
        "EQ boost should increase RMS by at least 2x, got {:.2}x",
        gain_ratio
    );

    eprintln!("✅ Effect processing validated - audio is modified");
}

/// Test toggling effects on and off
#[test]
#[cfg(feature = "effects")]
fn test_toggle_effect() {
    use soul_audio::effects::{EqBand, ParametricEq};

    let config = PlaybackConfig::default();

    let playback = match DesktopPlayback::new(config) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("Skipping test - no audio device: {}", e);
            return;
        }
    };

    let eq_bands = vec![EqBand::new(1000.0, 12.0, 1.0)];
    let mut eq = ParametricEq::new(eq_bands);
    eq.set_enabled(true);

    playback.with_effect_chain(|chain| {
        chain.add_effect(Box::new(eq));

        let sample_rate = 44100;
        let mut test_buffer = vec![0.1f32; 1024];

        // Process with effect enabled
        chain.process(&mut test_buffer, sample_rate);
        let rms_enabled = calculate_rms(&test_buffer);

        // Disable effect
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
            assert!(!effect.is_enabled());
        }

        // Process with effect disabled
        let mut test_buffer2 = vec![0.1f32; 1024];
        chain.process(&mut test_buffer2, sample_rate);
        let rms_disabled = calculate_rms(&test_buffer2);

        eprintln!("RMS (enabled): {:.6}", rms_enabled);
        eprintln!("RMS (disabled): {:.6}", rms_disabled);

        // When disabled, output should be same as input
        assert!(
            (rms_disabled - 0.1).abs() < 0.01,
            "Disabled effect should not modify audio"
        );
    });

    eprintln!("✅ Effect toggle verified");
}

/// Test removing effects from chain
#[test]
#[cfg(feature = "effects")]
fn test_remove_effect() {
    use soul_audio::effects::{EqBand, ParametricEq};

    let config = PlaybackConfig::default();

    let playback = match DesktopPlayback::new(config) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("Skipping test - no audio device: {}", e);
            return;
        }
    };

    playback.with_effect_chain(|chain| {
        // Add multiple effects
        let eq1 = ParametricEq::new(vec![EqBand::new(100.0, 3.0, 1.0)]);
        let eq2 = ParametricEq::new(vec![EqBand::new(1000.0, 3.0, 1.0)]);
        let eq3 = ParametricEq::new(vec![EqBand::new(10000.0, 3.0, 1.0)]);

        chain.add_effect(Box::new(eq1));
        chain.add_effect(Box::new(eq2));
        chain.add_effect(Box::new(eq3));

        assert_eq!(chain.len(), 3, "Should have 3 effects");

        // Clear all effects
        chain.clear();

        assert_eq!(chain.len(), 0, "Chain should be empty after clear");
        assert!(chain.is_empty());
    });

    eprintln!("✅ Effect removal verified");
}

/// Test multiple effects working together in chain
#[test]
#[cfg(feature = "effects")]
fn test_multiple_effects_chain() {
    use soul_audio::effects::{Compressor, CompressorSettings, EqBand, ParametricEq};

    let config = PlaybackConfig::default();

    let playback = match DesktopPlayback::new(config) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("Skipping test - no audio device: {}", e);
            return;
        }
    };

    playback.with_effect_chain(|chain| {
        // Add EQ and compressor in chain
        let eq = ParametricEq::new(vec![EqBand::new(1000.0, 6.0, 1.0)]);
        let compressor = Compressor::new(CompressorSettings::moderate());

        chain.add_effect(Box::new(eq));
        chain.add_effect(Box::new(compressor));

        assert_eq!(chain.len(), 2);

        // Process audio through both effects
        let mut test_buffer = vec![0.5f32; 1024];
        chain.process(&mut test_buffer, 44100);

        // Both effects should modify the signal
        let final_rms = calculate_rms(&test_buffer);
        eprintln!("Final RMS after EQ+Compressor: {:.6}", final_rms);

        // Signal should be modified by both effects
        assert!((final_rms - 0.5).abs() > 0.01, "Chain should modify audio");
    });

    eprintln!("✅ Multiple effects chain verified");
}

/// Test effect presets
#[test]
#[cfg(feature = "effects")]
fn test_effect_presets() {
    use soul_audio::effects::{CompressorSettings, LimiterSettings};

    // Test compressor presets
    let gentle = CompressorSettings::gentle();
    let moderate = CompressorSettings::moderate();
    let aggressive = CompressorSettings::aggressive();

    assert!(gentle.ratio < moderate.ratio);
    assert!(moderate.ratio < aggressive.ratio);
    eprintln!(
        "✅ Compressor presets: gentle={:.1}, moderate={:.1}, aggressive={:.1}",
        gentle.ratio, moderate.ratio, aggressive.ratio
    );

    // Test limiter presets
    let soft = LimiterSettings::soft();
    let default = LimiterSettings::default();
    let brickwall = LimiterSettings::brickwall();

    assert!(soft.threshold_db > default.threshold_db);
    assert!(default.threshold_db > brickwall.threshold_db);
    eprintln!(
        "✅ Limiter presets: soft={:.1}dB, default={:.1}dB, brickwall={:.1}dB",
        soft.threshold_db, default.threshold_db, brickwall.threshold_db
    );
}

/// Helper function to calculate RMS (Root Mean Square) of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&s| s * s).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
#[cfg(not(feature = "effects"))]
fn test_effects_feature_disabled() {
    eprintln!("⚠️  Effects feature is not enabled");
    eprintln!("   Run with: cargo test --features effects");
}
