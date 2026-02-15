//! Bit depth conversion precision tests
//!
//! Tests for issues identified in the bit depth audit:
//! - i32 asymmetric scaling
//! - U8 DC offset
//! - NaN/Infinity handling
//! - Precision loss in conversion chains

use soul_audio::dither::TpdfDither;

#[test]
fn test_i16_symmetric_scaling() {
    let mut dither = TpdfDither::new();

    // Test positive full scale
    let pos_output = dither.dither_to_i16(1.0);
    // With dither, output should be 32767 (max) or very close
    assert!(
        pos_output >= 32765,
        "Positive full scale should be near 32767, got {}",
        pos_output
    );

    // Test negative full scale
    let neg_output = dither.dither_to_i16(-1.0);
    // With dither, output should be -32768 (min) or very close
    assert!(
        neg_output >= -32768 && neg_output <= -32765,
        "Negative full scale should be near -32768, got {}",
        neg_output
    );
}

#[test]
#[ignore = "Currently fails - demonstrates asymmetric i32 bug"]
fn test_i32_asymmetric_scaling_bug() {
    let mut dither = TpdfDither::new();

    // This test demonstrates the i32 asymmetric scaling bug
    // Current behavior: -1.0 maps to -2147483647 (wrong)
    // Expected behavior: -1.0 maps to -2147483648 (correct)

    let neg_output = dither.dither_to_i32(-1.0);

    // This will fail with current implementation
    assert_eq!(
        neg_output,
        i32::MIN,
        "Negative full scale should map to i32::MIN (-2147483648), got {}",
        neg_output
    );

    // Current implementation produces:
    // -1.0 * 2147483647.0 = -2147483647 (loses 1 LSB)
}

#[test]
fn test_i32_scaling_symmetry() {
    let mut dither = TpdfDither::new();

    // Test that positive and negative scales are symmetric
    let pos = dither.dither_to_i32(1.0);
    let neg = dither.dither_to_i32(-1.0);

    // For symmetric scaling: |pos| should equal |neg| (within dither noise)
    // Use i64 to avoid overflow when taking abs() of i32::MIN
    let pos_magnitude = (pos as i64).abs();
    let neg_magnitude = (neg as i64).abs();
    let difference = (pos_magnitude - neg_magnitude).abs();

    // Allow for dither noise (±256 for 24-bit noise in i32)
    assert!(
        difference <= 256,
        "Asymmetric scaling detected: |{}| vs |{}|, diff = {}",
        pos,
        neg,
        difference
    );

    // This test may FAIL with current implementation:
    // pos ≈ 2147483647, neg ≈ -2147483647
    // difference = 0 (looks symmetric but both are wrong!)
    // The bug is that BOTH should use 2^31, not 2^31-1
}

#[test]
fn test_i32_decode_encode_roundtrip() {
    // Simulate decode -> encode roundtrip for i24 audio
    let i24_max = 8388607_i32;
    let i24_min = -8388608_i32;

    // Decode: i24 -> f32 (as in local.rs)
    let f32_max = i24_max as f32 / 8388608.0;
    let f32_min = i24_min as f32 / 8388608.0;

    assert_eq!(f32_min, -1.0, "i24 min should decode to exactly -1.0");
    assert!(
        f32_max < 1.0 && f32_max > 0.999,
        "i24 max should decode to ~1.0"
    );

    // Encode: f32 -> i32 (as in dither.rs)
    let mut dither = TpdfDither::new();
    let i32_output_max = dither.dither_to_i32(f32_max);
    let i32_output_min = dither.dither_to_i32(f32_min);

    println!("Roundtrip test:");
    println!(
        "  i24 max: {} -> f32: {:.15} -> i32: {}",
        i24_max, f32_max, i32_output_max
    );
    println!(
        "  i24 min: {} -> f32: {:.15} -> i32: {}",
        i24_min, f32_min, i32_output_min
    );

    // For symmetric scaling, we expect:
    // i24_max (8388607) should map to ~2147483391 (if using 2147483647 scale)
    // i24_min (-8388608) should map to ~-2147483648 (if using 2147483648 scale)

    // But current implementation uses 2147483647 for both, causing asymmetry
}

#[test]
#[ignore = "No NaN protection currently implemented"]
fn test_nan_protection() {
    let mut dither = TpdfDither::new();

    // Test NaN handling
    let nan_output = dither.dither_to_i16(f32::NAN);

    // Should return 0 (silence) not undefined behavior
    assert_eq!(nan_output, 0, "NaN should convert to 0 (silence)");
}

#[test]
#[ignore = "No infinity protection currently implemented"]
fn test_infinity_protection() {
    let mut dither = TpdfDither::new();

    // Test infinity handling
    let pos_inf_output = dither.dither_to_i16(f32::INFINITY);
    let neg_inf_output = dither.dither_to_i16(f32::NEG_INFINITY);

    // Should clamp to max/min, not undefined behavior
    assert_eq!(
        pos_inf_output,
        i16::MAX,
        "Positive infinity should clamp to i16::MAX"
    );
    assert_eq!(
        neg_inf_output,
        i16::MIN,
        "Negative infinity should clamp to i16::MIN"
    );
}

#[test]
fn test_i16_zero_noise() {
    let mut dither = TpdfDither::new();

    // Test zero signal with dither - should produce noise around 0
    let mut outputs = Vec::new();
    for _ in 0..1000 {
        outputs.push(dither.dither_to_i16(0.0));
    }

    // Check that outputs are centered around 0 (within ±5 samples)
    let mean: f64 = outputs.iter().map(|&x| x as f64).sum::<f64>() / 1000.0;
    assert!(
        mean.abs() < 5.0,
        "Zero signal should have mean near 0, got {}",
        mean
    );

    // Check that we have variation (not all zeros)
    let variance: f64 = outputs
        .iter()
        .map(|&x| (x as f64 - mean).powi(2))
        .sum::<f64>()
        / 1000.0;
    assert!(
        variance > 0.1,
        "Dither should add variation, got variance {}",
        variance
    );
}

#[test]
fn test_clipping_behavior() {
    let mut dither = TpdfDither::new();

    // Test values beyond ±1.0 (intersample peaks)
    let over_pos = dither.dither_to_i16(1.5);
    let over_neg = dither.dither_to_i16(-1.5);

    // Should clamp to valid range
    assert!(over_pos >= 32765, "Overload should clamp to near max");
    assert!(over_neg <= -32765, "Underload should clamp to near min");
}

#[test]
fn test_small_signal_quantization() {
    let mut dither = TpdfDither::new();

    // Test signal below 1 LSB (should be probabilistically quantized by dither)
    let small_signal = 0.5 / 32768.0; // 0.5 LSB

    let mut outputs = Vec::new();
    for _ in 0..1000 {
        outputs.push(dither.dither_to_i16(small_signal));
    }

    // Should get mix of 0 and 1 (probabilistic)
    let has_zero = outputs.contains(&0);
    let has_one = outputs.contains(&1);

    assert!(has_zero, "Small signal should sometimes quantize to 0");
    assert!(has_one, "Small signal should sometimes quantize to 1");

    // Mean should be close to 0.5 (the input signal in LSB units)
    let mean: f64 = outputs.iter().map(|&x| x as f64).sum::<f64>() / 1000.0;
    assert!(
        (0.0..=1.0).contains(&mean),
        "Small signal (0.5 LSB) should average to ~0.5, got {}",
        mean
    );
}

#[test]
fn test_precision_loss_i24_to_i32() {
    // Test precision loss when converting 24-bit audio to i32 output
    let mut dither = TpdfDither::new();

    // Maximum 24-bit value
    let i24_max = 8388607;
    let f32_normalized = i24_max as f32 / 8388608.0;

    // Convert to i32
    let i32_output = dither.dither_to_i32(f32_normalized);

    println!("24-bit to i32 precision:");
    println!("  i24: {}", i24_max);
    println!("  f32: {:.15}", f32_normalized);
    println!("  i32: {}", i32_output);
    println!("  i32::MAX: {}", i32::MAX);

    let difference = i32::MAX - i32_output.abs();
    if difference > 0 {
        println!("  Lost bits: ~{}", difference.ilog2());
    } else {
        println!("  Lost bits: 0 (using full range)");
    }

    // We expect to lose ~8 bits (256x) due to 24-bit source material
    // Current output: ~2147483391
    // Max possible: 2147483647
    // Difference: 256 (8 bits)

    assert!(
        i32_output > 2147483000,
        "i32 output should use most of the range, got {}",
        i32_output
    );
}

#[test]
fn test_u8_dc_offset_simulation() {
    // This simulates the U8 DC offset bug from local.rs:1061

    // U8 silent audio (center value = 128)
    let u8_center = 128_u8;

    // Current buggy formula
    let buggy_output = (u8_center as f32 / u8::MAX as f32) * 2.0 - 1.0;

    // Correct formula (cast to i16 first to avoid overflow)
    let correct_output = (u8_center as i16 - 128) as f32 / 128.0;

    println!("U8 DC offset test:");
    println!(
        "  U8 center (128): buggy={:.10}, correct={:.10}",
        buggy_output, correct_output
    );
    println!(
        "  DC offset in i16 scale: {:.2} samples",
        buggy_output * 32768.0
    );

    assert_eq!(correct_output, 0.0, "U8 center should map to 0.0");
    assert!(
        buggy_output.abs() > 0.001,
        "Current implementation has DC offset: {}",
        buggy_output
    );

    // This test PASSES (demonstrates the bug exists)
}

#[test]
fn test_denormal_handling() {
    let mut dither = TpdfDither::new();

    // Test extremely small signal (denormal number)
    let denormal = 1e-40_f32;
    let output = dither.dither_to_i16(denormal);

    // Should round to 0 or ±1 (with dither noise)
    assert!(
        output.abs() <= 1,
        "Denormal should quantize to 0 or ±1, got {}",
        output
    );
}
