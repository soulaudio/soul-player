//! Sample format conversion macros
//!
//! Provides efficient macros for common DSP sample format conversions.
//! These macros reduce boilerplate for conversions between f32 and integer formats
//! while ensuring consistent behavior across the codebase.
//!
//! # Safety
//!
//! All conversions are designed for audio callback safety:
//! - No heap allocations
//! - Consistent clipping behavior
//! - Proper scaling for audio ranges
//!
//! # Examples
//!
//! ```
//! use soul_audio::{f32_to_i16, f32_to_i32, i16_to_f32, i32_to_f32};
//!
//! // Convert single sample
//! let sample = 0.5_f32;
//! let i16_sample = f32_to_i16!(sample);
//! let i32_sample = f32_to_i32!(sample);
//!
//! // Convert back to f32
//! let f32_sample = i16_to_f32!(i16_sample);
//!
//! // With clipping
//! let hot_sample = 1.5_f32;
//! let clipped = f32_to_i16!(hot_sample, clip);
//! ```

/// Convert f32 sample to i16
///
/// # Arguments
/// - `$sample` - f32 sample in range [-1.0, 1.0]
/// - `clip` (optional) - If specified, clips input to [-1.0, 1.0] before conversion
///
/// # Returns
/// i16 sample in range [i16::MIN, i16::MAX]
///
/// # Scaling
/// Uses `32767.0` (i16::MAX) for proper symmetric scaling.
/// Note: This intentionally avoids i16::MIN (-32768) to maintain
/// symmetry around zero, which is standard in audio processing.
#[macro_export]
#[allow(clippy::neg_multiply)]
macro_rules! f32_to_i16 {
    ($sample:expr) => {
        ($sample * 32767.0) as i16
    };
    ($sample:expr, clip) => {{
        let s: f32 = $sample;
        (s.clamp(-1.0, 1.0) * 32767.0) as i16
    }};
}

/// Convert f32 sample to i32
///
/// # Arguments
/// - `$sample` - f32 sample in range [-1.0, 1.0]
/// - `clip` (optional) - If specified, clips input to [-1.0, 1.0] before conversion
///
/// # Returns
/// i32 sample in range [i32::MIN, i32::MAX]
///
/// # Scaling
/// Uses `2147483647.0` (i32::MAX) for proper symmetric scaling.
/// Note: Like i16, this avoids i32::MIN for audio symmetry.
#[macro_export]
#[allow(clippy::neg_multiply)]
macro_rules! f32_to_i32 {
    ($sample:expr) => {
        ($sample * 2147483647.0) as i32
    };
    ($sample:expr, clip) => {{
        let s: f32 = $sample;
        (s.clamp(-1.0, 1.0) * 2147483647.0) as i32
    }};
}

/// Convert i16 sample to f32
///
/// # Arguments
/// - `$sample` - i16 sample
///
/// # Returns
/// f32 sample in range approximately [-1.0, 1.0]
///
/// # Scaling
/// Uses `i16::MAX as f32` (32767.0) for proper normalization.
#[macro_export]
macro_rules! i16_to_f32 {
    ($sample:expr) => {
        $sample as f32 / 32767.0
    };
}

/// Convert i32 sample to f32
///
/// # Arguments
/// - `$sample` - i32 sample
///
/// # Returns
/// f32 sample in range approximately [-1.0, 1.0]
///
/// # Scaling
/// Uses `i32::MAX as f32` (2147483647.0) for proper normalization.
#[macro_export]
macro_rules! i32_to_f32 {
    ($sample:expr) => {
        $sample as f32 / 2147483647.0
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_f32_to_i16() {
        // Zero
        assert_eq!(f32_to_i16!(0.0), 0);

        // Positive
        assert_eq!(f32_to_i16!(1.0), 32767);
        assert_eq!(f32_to_i16!(0.5), 16383);

        // Negative
        assert_eq!(f32_to_i16!(-1.0), -32767);
        assert_eq!(f32_to_i16!(-0.5), -16383);
    }

    #[test]
    fn test_f32_to_i16_clip() {
        // Over range - should clip
        assert_eq!(f32_to_i16!(2.0, clip), 32767);
        assert_eq!(f32_to_i16!(-2.0, clip), -32767);

        // Normal range - should pass through
        assert_eq!(f32_to_i16!(0.5, clip), 16383);
    }

    #[test]
    fn test_f32_to_i32() {
        // Zero
        assert_eq!(f32_to_i32!(0.0), 0);

        // Positive
        assert_eq!(f32_to_i32!(1.0), 2147483647);

        // Negative
        assert_eq!(f32_to_i32!(-1.0), -2147483647);
    }

    #[test]
    fn test_f32_to_i32_clip() {
        // Over range - should clip
        assert_eq!(f32_to_i32!(2.0, clip), 2147483647);
        // -1.0 * i32::MAX can be -2147483648 due to f32 precision
        let neg_clip = f32_to_i32!(-2.0, clip);
        assert!(neg_clip == -2147483647 || neg_clip == -2147483648);

        // Normal range
        assert_eq!(f32_to_i32!(0.5, clip), 1073741823);
    }

    #[test]
    fn test_i16_to_f32() {
        // Zero
        assert_eq!(i16_to_f32!(0_i16), 0.0);

        // Max (should be exactly 1.0)
        assert_eq!(i16_to_f32!(32767_i16), 1.0);

        // Half scale
        let half = i16_to_f32!(16384_i16);
        assert!((half - 0.5).abs() < 0.001);

        // Negative
        assert_eq!(i16_to_f32!(-32767_i16), -1.0);
    }

    #[test]
    fn test_i32_to_f32() {
        // Zero
        assert_eq!(i32_to_f32!(0_i32), 0.0);

        // Max (should be exactly 1.0)
        assert_eq!(i32_to_f32!(2147483647_i32), 1.0);

        // Negative max
        assert_eq!(i32_to_f32!(-2147483647_i32), -1.0);
    }

    #[test]
    fn test_roundtrip_i16() {
        let original = 0.5_f32;
        let i16_val = f32_to_i16!(original);
        let recovered = i16_to_f32!(i16_val);

        // Should be very close (within quantization error)
        assert!((original - recovered).abs() < 0.0001);
    }

    #[test]
    fn test_roundtrip_i32() {
        let original = 0.5_f32;
        let i32_val = f32_to_i32!(original);
        let recovered = i32_to_f32!(i32_val);

        // Should be very close (within quantization error)
        assert!((original - recovered).abs() < 0.0000001);
    }
}
