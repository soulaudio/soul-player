//! Critical bug hunt tests for dynamics processors (compressor and limiter)
//!
//! These tests expose REAL bugs in the DSP implementations, not just coverage tests.
//! Each test documents a specific bug with the problematic code, math explanation,
//! and inputs that reveal the issue.

use soul_audio::effects::{AudioEffect, Compressor, CompressorSettings, Limiter, LimiterSettings};

// =============================================================================
// BUG #1: Limiter envelope initialized to 1.0 causes quiet signal attenuation
// =============================================================================
//
// Problematic code (limiter.rs:85):
//   envelope: 1.0,
//
// The issue: When envelope starts at 1.0 (linear), quiet signals like 0.1 get
// attenuated because: gain = threshold_linear / envelope = 0.977 / 1.0 = 0.977
// This applies ~0.2 dB of attenuation to signals that shouldn't be affected!
//
// Fix: Initialize envelope to 0.0 (meaning "no signal detected yet")

#[test]
fn bug_limiter_envelope_init_attenuates_quiet_signal() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -0.3, // Linear threshold ~0.966
        release_ms: 50.0,
    });

    // Very quiet signal - should NOT be affected by limiter
    let original = 0.1_f32;
    let mut buffer = vec![original, original]; // Single stereo frame

    limiter.process(&mut buffer, 44100);

    // BUG REVEALED: The quiet signal is attenuated!
    // Expected: buffer[0] == 0.1 (no limiting needed)
    // Actual: buffer[0] < 0.1 (incorrectly attenuated)
    let attenuation = (original - buffer[0]) / original;
    println!("Attenuation on quiet signal: {:.2}%", attenuation * 100.0);

    // This assertion SHOULD pass but currently FAILS due to the bug
    assert!(
        (buffer[0] - original).abs() < 0.001,
        "Quiet signal at {} was incorrectly attenuated to {}. \
         Limiter should not affect signals below threshold!",
        original,
        buffer[0]
    );
}

// =============================================================================
// BUG #2: Compressor envelope init to 0.0 makes attack time ineffective
// =============================================================================
//
// Problematic code (compressor.rs:136-137):
//   envelope_l: 0.0,
//   envelope_r: 0.0,
//
// The issue: When envelope starts at 0.0 dB (which is VERY LOUD in dB scale),
// and the first sample arrives, the envelope doesn't "attack" up from a quiet
// state - it's already at 0 dB. This makes attack time meaningless for the
// initial transient.
//
// Actually, let's reconsider: 0.0 in the envelope represents 0 dB level.
// The envelope is in dB scale. When a signal arrives, if it's > 0 dB (envelope),
// the compressor uses attack coefficient. But envelope starting at 0.0 dB
// means the first loud transient will trigger immediate compression if above
// threshold, regardless of attack time.
//
// Wait - the envelope is in dB and 0.0 dB means full scale. A signal of 0.1
// linear = -20 dB. So envelope at 0 is ABOVE most signals, meaning release
// is used, not attack. This causes the opposite problem - compression doesn't
// engage properly!

#[test]
fn bug_compressor_envelope_init_breaks_attack() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,     // Very aggressive
        attack_ms: 50.0, // Slow attack - should let transients through
        release_ms: 100.0,
        knee_db: 0.0, // Hard knee for predictable behavior
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Generate a loud transient
    let loud_signal = 0.9_f32; // About -0.9 dB, well above -20 dB threshold
    let buffer = vec![loud_signal; 20]; // 10 stereo frames

    // Make it stereo interleaved
    let mut stereo_buffer: Vec<f32> = Vec::with_capacity(buffer.len() * 2);
    for &sample in &buffer {
        stereo_buffer.push(sample);
        stereo_buffer.push(sample);
    }

    comp.process(&mut stereo_buffer, 44100);

    // With 50ms attack time, the first few milliseconds should NOT be compressed
    // because the envelope hasn't "caught up" yet.
    // At 44100 Hz, 50ms = 2205 samples. Our 20 samples = 0.45ms
    // So first sample should be nearly unchanged!

    let first_sample_reduction = (loud_signal - stereo_buffer[0]) / loud_signal;
    println!(
        "First sample: input={}, output={}, reduction={:.1}%",
        loud_signal,
        stereo_buffer[0],
        first_sample_reduction * 100.0
    );

    // BUG REVEALED: First sample is immediately compressed despite slow attack
    // This happens because envelope init of 0.0 (dB) causes wrong behavior
    assert!(
        first_sample_reduction < 0.05,
        "With 50ms attack, first sample should not be significantly compressed! \
         Got {:.1}% reduction. Attack time is not working.",
        first_sample_reduction * 100.0
    );
}

// =============================================================================
// TEST #3: Verify soft knee produces correct gain reduction
// =============================================================================
//
// NOTE: Initial analysis suggested the soft knee formula was wrong, but testing
// shows it actually works correctly. The formula:
//   -knee_gain * knee * (1.0 - 1.0 / ratio)
// DOES produce negative gain (compression) in the knee region.
//
// This test verifies the soft knee is working correctly.

#[test]
fn verify_soft_knee_compresses_at_threshold() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 0.1,   // Fast attack
        release_ms: 10.0, // Fast release
        knee_db: 6.0,     // 6dB soft knee
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Signal in the middle of the knee region
    // Knee spans: -23 dB to -17 dB (threshold -20, knee 6)
    // Let's use -20 dB = 0.1 linear (exactly at threshold)
    let input_linear = 0.1_f32;
    let input_db = 20.0 * input_linear.log10(); // = -20 dB

    println!("Input level: {:.2} dB", input_db);

    // Process enough samples for envelope to settle with fresh input
    for _ in 0..1000 {
        let mut buffer = vec![input_linear, input_linear];
        comp.process(&mut buffer, 44100);
    }

    // Measure with fresh input
    let mut buffer = vec![input_linear, input_linear];
    comp.process(&mut buffer, 44100);

    // At exactly the threshold, with soft knee, there should be SOME compression
    // (soft knee starts compressing below the threshold)
    let output = buffer[0];
    let output_db = 20.0 * output.abs().max(0.000001).log10();
    let gain_db = output_db - input_db;

    println!(
        "Output level: {:.2} dB, Gain applied: {:.2} dB",
        output_db, gain_db
    );

    // With correct soft knee, at threshold, gain should be NEGATIVE (compression)
    // Specifically, at exact threshold, gain ≈ -knee/2 * (1 - 1/ratio) / 2 ≈ -1.1 dB
    assert!(
        gain_db < 0.0,
        "At threshold with soft knee, output should be REDUCED (negative gain). \
         Got {:.2} dB gain. The soft knee formula has wrong sign!",
        gain_db
    );
}

// =============================================================================
// BUG #4: Limiter claims lookahead but has NO delay buffer
// =============================================================================
//
// Problematic code (limiter.rs:4-5):
//   /// This implementation uses a lookahead buffer for zero-latency brick-wall limiting.
//
// This is IMPOSSIBLE. Lookahead REQUIRES latency. You cannot look ahead in time
// without introducing delay. The documentation is false advertising.
//
// A true lookahead limiter:
// 1. Delays the audio by N samples
// 2. Looks at upcoming peaks N samples ahead
// 3. Pre-emptively reduces gain before the peak arrives
//
// This limiter has NO delay buffer, so it CANNOT look ahead.
// It's an instant-attack limiter, which causes distortion on transients.

#[test]
fn bug_limiter_no_lookahead_causes_distortion() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0, // -1 dB threshold
        release_ms: 200.0,
    });

    // Create a sharp transient: silence -> loud peak -> silence
    let peak = 0.95_f32;
    let _threshold_linear = 10.0_f32.powf(-1.0 / 20.0); // ~0.89

    // Build test signal: ramp up quickly
    let mut buffer: Vec<f32> = Vec::new();
    for i in 0..10 {
        let sample = (i as f32 / 9.0) * peak;
        buffer.push(sample);
        buffer.push(sample);
    }

    limiter.process(&mut buffer, 44100);

    // With true lookahead, the limiter would see the peak coming and smoothly
    // reduce gain BEFORE the peak arrives. Without lookahead, it reacts
    // AFTER the peak, causing gain modulation that follows the signal.

    // Check if the limiter is actually just tracking the input (no lookahead)
    // A true lookahead limiter would have constant or smoothly varying gain
    let mut gain_variations = 0;
    let mut prev_gain = 1.0_f32;

    for chunk in buffer.chunks(2) {
        let input_approx = chunk[0].abs().max(chunk[1].abs());
        if input_approx > 0.01 {
            let gain = chunk[0] / input_approx.max(0.0001);
            if (gain - prev_gain).abs() > 0.01 {
                gain_variations += 1;
            }
            prev_gain = gain;
        }
    }

    println!(
        "Gain variations during transient: {} (0 = true lookahead, >3 = reactive)",
        gain_variations
    );

    // BUG: The limiter has many gain variations because it's reacting, not predicting
    // A true lookahead limiter would have <=1 gain change
    assert!(
        gain_variations <= 2,
        "Limiter has {} gain variations on a simple ramp, indicating it has NO lookahead. \
         The documentation claims lookahead but implementation lacks delay buffer!",
        gain_variations
    );
}

// =============================================================================
// BUG #5: Stereo channels processed independently (no linking)
// =============================================================================
//
// Problematic code (compressor.rs:282-285):
//   for chunk in buffer.chunks_exact_mut(2) {
//       chunk[0] = self.process_sample(chunk[0], &mut env_l);
//       chunk[1] = self.process_sample(chunk[1], &mut env_r);
//   }
//
// Each channel has its own envelope! This causes stereo image shifting.
// When left channel is loud and right is quiet, left gets compressed more,
// shifting the stereo image to the right.
//
// Professional compressors use "stereo linking" where both channels are
// compressed by the same amount (typically using max or sum of both envelopes).

#[test]
fn bug_compressor_stereo_image_shift() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 8.0,       // Strong compression
        attack_ms: 0.1,   // Fast attack
        release_ms: 10.0, // Fast release
        knee_db: 0.0,     // Hard knee
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Create asymmetric stereo signal: loud left, quiet right
    let left_level = 0.8_f32; // Loud (above threshold)
    let right_level = 0.1_f32; // Quiet (below threshold)

    // Process enough frames for envelopes to stabilize with fresh input
    for _ in 0..2000 {
        let mut buffer = vec![left_level, right_level];
        comp.process(&mut buffer, 44100);
    }

    // Measure with fresh input
    let mut buffer = vec![left_level, right_level];
    comp.process(&mut buffer, 44100);

    let output_left = buffer[0];
    let output_right = buffer[1];

    // Calculate the stereo balance change
    let input_balance = left_level / right_level; // 8.0 (left is 8x louder)
    let output_balance = output_left / output_right;

    println!(
        "Input balance (L/R): {:.2}, Output balance: {:.2}",
        input_balance, output_balance
    );

    // BUG REVEALED: The balance changes because only the loud channel is compressed
    // Input: left is 8x louder than right
    // Output: left is compressed, right is not, so balance decreases

    // With proper stereo linking, balance should be preserved
    assert!(
        (output_balance - input_balance).abs() / input_balance < 0.1,
        "Stereo balance changed from {:.2} to {:.2} ({:.1}% change). \
         Compressor should use stereo linking to preserve stereo image!",
        input_balance,
        output_balance,
        (input_balance - output_balance).abs() / input_balance * 100.0
    );
}

// =============================================================================
// BUG #6: Attack/release time constants are ~5x longer than specified
// =============================================================================
//
// Problematic code (compressor.rs:198-200):
//   self.attack_coeff = (-1.0 / (self.settings.attack_ms * sr / 1000.0)).exp();
//
// The formula computes: coeff = exp(-1 / N) where N is samples in the time
//
// The issue: This is a one-pole filter time constant, but the resulting
// behavior reaches 63.2% (1 - 1/e) of the target in the specified time.
// Most audio engineers expect the specified time to be when the envelope
// reaches 90% or 99% of the target!
//
// For standard "time to reach 90%", the formula should be:
//   coeff = exp(-2.2 / N)  // 2.2 ≈ -ln(0.1)
//
// Current implementation: 5ms attack takes ~23ms to reach 99% of target
// (5x longer than expected!)

#[test]
fn bug_attack_time_5x_too_slow() {
    let attack_ms = 10.0_f32;
    let settings = CompressorSettings {
        threshold_db: -40.0, // Low threshold so everything compresses
        ratio: 20.0,         // Maximum compression
        attack_ms,
        release_ms: 1000.0, // Long release so we only measure attack
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Start with silence to reset envelope
    comp.reset();

    // Then hit it with a loud signal
    let loud = 0.9_f32;
    let sample_rate = 44100_u32;

    // Calculate how many samples until envelope should reach 90% of target
    let expected_samples = (attack_ms / 1000.0 * sample_rate as f32) as usize;

    // Process samples and find when compression reaches 90%
    let mut samples_to_90_percent = 0;
    let input_db = 20.0 * loud.log10();
    let _target_envelope = input_db; // Final envelope should match input

    for i in 0..(expected_samples * 10) {
        let mut buffer = vec![loud, loud];
        comp.process(&mut buffer, sample_rate);

        // Estimate current envelope from compression amount
        // (This is approximate since we can't directly access envelope)
        let output = buffer[0];
        let compression_ratio = output / loud;

        // When envelope reaches target, compression_ratio should stabilize
        // We're looking for 90% of final compression
        if samples_to_90_percent == 0 && compression_ratio < 0.95 {
            samples_to_90_percent = i;
        }
    }

    let actual_time_ms = samples_to_90_percent as f32 / sample_rate as f32 * 1000.0;
    let ratio = actual_time_ms / attack_ms;

    println!(
        "Expected attack time: {}ms, Actual time to 90%: {:.1}ms, Ratio: {:.1}x",
        attack_ms, actual_time_ms, ratio
    );

    // BUG: Attack takes much longer than specified
    assert!(
        ratio < 2.0,
        "Attack time is {:.1}x longer than specified! \
         Expected ~{}ms, got ~{:.1}ms. \
         The time constant formula is missing the standard -2.2 factor.",
        ratio,
        attack_ms,
        actual_time_ms
    );
}

// =============================================================================
// BUG #7: Ratio = infinity (very high) edge case
// =============================================================================
//
// When ratio is very high (effectively infinity/brickwall limiting),
// the compressor should behave like a limiter, keeping output at threshold.
// But the gain calculation might have numerical issues.
//
// Problematic code (compressor.rs:220):
//   (threshold - input_db) + (input_db - threshold) / ratio
//   = (input_db - threshold) * (1/ratio - 1)
//
// With ratio = 20 (max), 1/ratio = 0.05, so formula is:
//   = (input_db - threshold) * (-0.95)
//
// For input 10dB above threshold: gain_reduction = 10 * -0.95 = -9.5 dB
// Expected (infinite ratio): -10 dB
// This is correct but let's verify edge cases.

#[test]
fn edge_case_ratio_infinity_should_brick_wall() {
    let settings = CompressorSettings {
        threshold_db: -10.0,
        ratio: 20.0, // Maximum ratio (pseudo-infinity)
        attack_ms: 0.1,
        release_ms: 10.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Signal way above threshold
    let input = 0.9_f32; // About -0.9 dB

    // Let envelope settle - use fresh input each time
    for _ in 0..5000 {
        let mut buffer = vec![input, input];
        comp.process(&mut buffer, 44100);
    }

    // Now measure with fresh input
    let mut buffer = vec![input, input];
    comp.process(&mut buffer, 44100);

    let output_db = 20.0 * buffer[0].abs().max(0.000001).log10();

    println!(
        "Input: {:.1} dB, Threshold: {:.1} dB, Output: {:.2} dB",
        20.0 * input.log10(),
        settings.threshold_db,
        output_db
    );

    // With ratio=20 and input 9dB above threshold,
    // output should be very close to threshold (within ~0.5dB)
    assert!(
        (output_db - settings.threshold_db).abs() < 1.0,
        "With ratio=20 (near infinity), output should be at threshold. \
         Expected ~{:.1} dB, got {:.2} dB",
        settings.threshold_db,
        output_db
    );
}

// =============================================================================
// BUG #8: Ratio = 1 should do nothing (unity gain in dB)
// =============================================================================
//
// With ratio = 1:1, there should be NO compression at all.
// gain_reduction = (threshold - input_db) + (input_db - threshold) / 1
//                = (threshold - input_db) + (input_db - threshold)
//                = 0
// This should work correctly, let's verify.

#[test]
fn edge_case_ratio_1_no_compression() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 1.0, // 1:1 = no compression
        attack_ms: 0.1,
        release_ms: 10.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Loud signal
    let input = 0.8_f32;

    // Let envelope settle with fresh input each time
    for _ in 0..1000 {
        let mut buffer = vec![input, input];
        comp.process(&mut buffer, 44100);
    }

    // Now measure with fresh input
    let mut buffer = vec![input, input];
    comp.process(&mut buffer, 44100);

    // With ratio=1, output should equal input
    assert!(
        (buffer[0] - input).abs() < 0.01,
        "With ratio=1:1, signal should be unchanged. \
         Input: {}, Output: {}",
        input,
        buffer[0]
    );
}

// =============================================================================
// BUG #9: Threshold = 0 dB edge case
// =============================================================================
//
// With threshold at 0 dB (maximum level), only signals AT or ABOVE 0 dB
// should be compressed. A signal at 0.9 (-0.9 dB) should pass through.

#[test]
fn edge_case_threshold_0db() {
    let settings = CompressorSettings {
        threshold_db: 0.0, // Max threshold
        ratio: 10.0,
        attack_ms: 0.1,
        release_ms: 10.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Signal just below 0 dB
    let input = 0.9_f32; // -0.9 dB

    // Let envelope settle with fresh input each time
    for _ in 0..1000 {
        let mut buffer = vec![input, input];
        comp.process(&mut buffer, 44100);
    }

    // Now measure with fresh input
    let mut buffer = vec![input, input];
    comp.process(&mut buffer, 44100);

    // Should be unchanged (below threshold)
    assert!(
        (buffer[0] - input).abs() < 0.01,
        "Signal at {} (-0.9 dB) should not be compressed with threshold at 0 dB. \
         Output: {}",
        input,
        buffer[0]
    );
}

// =============================================================================
// BUG #10: Makeup gain clamped to positive only
// =============================================================================
//
// Problematic code (compressor.rs:97):
//   self.makeup_gain_db = self.makeup_gain_db.clamp(0.0, 24.0);
//
// Negative makeup gain is actually useful! Sometimes after compression,
// the signal is still too loud and needs to be attenuated.
// Clamping to 0.0 minimum is unnecessarily restrictive.

#[test]
fn issue_negative_makeup_gain_not_allowed() {
    let mut settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 0.0,
        makeup_gain_db: -6.0, // User wants to REDUCE output by 6 dB
    };

    settings.validate();

    // BUG: Negative makeup gain is clamped to 0
    assert_eq!(
        settings.makeup_gain_db, 0.0,
        "Negative makeup gain was clamped to 0. \
         This is a design issue - negative makeup gain should be allowed!"
    );

    // This test documents the limitation rather than a bug
    // Remove the assertion to make it pass, or keep it to document the issue
}

// =============================================================================
// BUG #11: Limiter reset() sets envelope to 1.0 (same as init bug)
// =============================================================================
//
// Problematic code (limiter.rs:163-165):
//   fn reset(&mut self) {
//       self.envelope = 1.0;
//   }
//
// This perpetuates bug #1. After reset, quiet signals will be attenuated.

#[test]
fn bug_limiter_reset_wrong_envelope() {
    let mut limiter = Limiter::new();

    // Process some loud signal
    let mut buffer = vec![0.95_f32; 100];
    limiter.process(&mut buffer, 44100);

    // Reset
    limiter.reset();

    // Now process quiet signal
    let quiet = 0.1_f32;
    let mut buffer = vec![quiet, quiet];
    limiter.process(&mut buffer, 44100);

    // BUG: Quiet signal is attenuated after reset
    assert!(
        (buffer[0] - quiet).abs() < 0.001,
        "After reset(), quiet signal {} was changed to {}. \
         Reset sets envelope to 1.0 causing same bug as initialization.",
        quiet,
        buffer[0]
    );
}

// =============================================================================
// BUG #12: Compressor processes mono buffer incorrectly (odd length)
// =============================================================================
//
// Problematic code (compressor.rs:282):
//   for chunk in buffer.chunks_exact_mut(2)
//
// chunks_exact_mut(2) silently drops the last sample if buffer length is odd.
// This would cause audio glitches in mono or non-standard buffer sizes.

#[test]
fn edge_case_mono_buffer_last_sample_dropped() {
    let mut comp = Compressor::new();

    // Mono buffer with odd length
    let mut buffer = vec![0.5_f32; 5]; // 5 samples
    let original_last = buffer[4];

    comp.process(&mut buffer, 44100);

    // BUG: Last sample is not processed (chunks_exact_mut drops remainder)
    assert_eq!(
        buffer[4], original_last,
        "Last sample in odd-length buffer was not processed. \
         chunks_exact_mut(2) drops remainder samples!"
    );
}

// =============================================================================
// BUG #12: Compressor produces WRONG gain reduction amount
// =============================================================================
//
// With ratio 4:1, a signal 12 dB above threshold should be reduced to:
// - Input: threshold + 12 dB
// - Output: threshold + 12/4 = threshold + 3 dB
// - Gain reduction: -9 dB
//
// Let's verify the actual math is correct.

#[test]
fn verify_compressor_gain_reduction_amount() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 0.1,
        release_ms: 10.0,
        knee_db: 0.0, // Hard knee for predictable math
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    // Signal at -8 dB (12 dB above -20 dB threshold)
    let input_db = -8.0_f32;
    let input_linear = 10.0_f32.powf(input_db / 20.0); // ~0.398

    // Let envelope settle
    for _ in 0..5000 {
        let mut buffer = vec![input_linear, input_linear];
        comp.process(&mut buffer, 44100);
    }

    // Measure
    let mut buffer = vec![input_linear, input_linear];
    comp.process(&mut buffer, 44100);

    let output_db = 20.0 * buffer[0].abs().max(0.000001).log10();
    let gain_reduction = output_db - input_db;

    // Expected: output should be at threshold + (input - threshold) / ratio
    // = -20 + (-8 - (-20)) / 4 = -20 + 12/4 = -20 + 3 = -17 dB
    // So gain reduction should be -17 - (-8) = -9 dB
    let expected_gain_reduction = -9.0_f32;

    println!(
        "Input: {:.1} dB, Output: {:.1} dB, Gain reduction: {:.1} dB (expected: {:.1} dB)",
        input_db, output_db, gain_reduction, expected_gain_reduction
    );

    assert!(
        (gain_reduction - expected_gain_reduction).abs() < 0.5,
        "Gain reduction is wrong! Expected {:.1} dB, got {:.1} dB. \
         The compression ratio math may be incorrect.",
        expected_gain_reduction,
        gain_reduction
    );
}

// =============================================================================
// Summary test: Verify basic functionality still works
// =============================================================================

#[test]
fn sanity_check_compressor_reduces_loud_signal() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 0.1,
        release_ms: 10.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut comp = Compressor::with_settings(settings);

    let input = 0.8_f32;

    // Process many frames to let envelope settle
    // Use fresh input each time (don't feed output back as input)
    for _ in 0..5000 {
        let mut buffer = vec![input, input];
        comp.process(&mut buffer, 44100);
    }

    // Now measure the final output
    let mut buffer = vec![input, input];
    comp.process(&mut buffer, 44100);

    // Output should be reduced
    assert!(
        buffer[0] < input,
        "Compressor should reduce loud signal. Input: {}, Output: {}",
        input,
        buffer[0]
    );

    // But not to zero
    assert!(
        buffer[0] > 0.1,
        "Compressor should not over-compress. Output: {}",
        buffer[0]
    );
}

#[test]
fn sanity_check_limiter_prevents_clipping() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0,
        release_ms: 50.0,
    });

    // Signal that would clip
    let mut buffer = vec![1.5_f32, 1.5_f32];

    limiter.process(&mut buffer, 44100);

    let threshold_linear = 10.0_f32.powf(-1.0 / 20.0);

    // Output should be at or below threshold
    assert!(
        buffer[0] <= threshold_linear + 0.01,
        "Limiter should prevent signal above threshold. \
         Threshold: {:.3}, Output: {:.3}",
        threshold_linear,
        buffer[0]
    );
}
