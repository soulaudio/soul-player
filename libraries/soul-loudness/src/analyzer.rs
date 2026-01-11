//! EBU R128 loudness analysis
//!
//! This module provides EBU R128-compliant loudness measurement using the ebur128 crate.
//! It measures:
//! - Integrated loudness (LUFS) - the overall perceived loudness
//! - Loudness range (LRA) - the variation in loudness
//! - True peak (dBTP) - the maximum inter-sample peak level
//! - Sample peak (dBFS) - the maximum sample value

use crate::error::{LoudnessError, Result};
use ebur128::{EbuR128, Mode};
use std::fmt;

/// Information about the loudness characteristics of an audio track
#[derive(Debug, Clone, PartialEq)]
pub struct LoudnessInfo {
    /// Integrated loudness in LUFS (Loudness Units Full Scale)
    /// This is the main measure of perceived loudness over the entire track
    pub integrated_lufs: f64,

    /// Loudness range in LU (Loudness Units)
    /// Measures the variation in loudness - lower values indicate more compressed audio
    pub loudness_range_lu: f64,

    /// True peak in dBTP (decibels True Peak)
    /// The maximum inter-sample peak, accounting for potential peaks between samples
    /// Uses 4x oversampling for sample rates < 96kHz as per ITU-R BS.1770
    pub true_peak_dbfs: f64,

    /// Sample peak in dBFS (decibels Full Scale)
    /// The maximum sample value (not accounting for inter-sample peaks)
    pub sample_peak_dbfs: f64,

    /// Duration of the analyzed audio in seconds
    pub duration_seconds: f64,

    /// Sample rate of the analyzed audio
    pub sample_rate: u32,

    /// Number of channels
    pub channels: u32,
}

impl LoudnessInfo {
    /// Check if the audio might clip when applying gain
    pub fn will_clip_at_gain(&self, gain_db: f64) -> bool {
        self.true_peak_dbfs + gain_db > 0.0
    }

    /// Calculate the maximum safe gain (to prevent clipping)
    pub fn max_safe_gain(&self) -> f64 {
        -self.true_peak_dbfs
    }
}

impl Default for LoudnessInfo {
    fn default() -> Self {
        Self {
            integrated_lufs: -23.0, // EBU R128 reference
            loudness_range_lu: 0.0,
            true_peak_dbfs: -f64::INFINITY,
            sample_peak_dbfs: -f64::INFINITY,
            duration_seconds: 0.0,
            sample_rate: 44100,
            channels: 2,
        }
    }
}

impl fmt::Display for LoudnessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Loudness: {:.1} LUFS, Range: {:.1} LU, True Peak: {:.1} dBTP, Sample Peak: {:.1} dBFS",
            self.integrated_lufs,
            self.loudness_range_lu,
            self.true_peak_dbfs,
            self.sample_peak_dbfs
        )
    }
}

/// EBU R128 loudness analyzer
///
/// Analyzes audio samples to measure loudness according to EBU R128 / ITU-R BS.1770.
///
/// # Example
///
/// ```ignore
/// use soul_loudness::LoudnessAnalyzer;
///
/// let mut analyzer = LoudnessAnalyzer::new(44100, 2)?;
///
/// // Feed audio samples (interleaved f32)
/// analyzer.add_frames(&audio_samples)?;
///
/// // Get results
/// let info = analyzer.finalize()?;
/// println!("Integrated loudness: {:.1} LUFS", info.integrated_lufs);
/// ```
pub struct LoudnessAnalyzer {
    /// EBU R128 analyzer instance
    ebur128: EbuR128,
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u32,
    /// Total samples processed
    samples_processed: usize,
}

impl LoudnessAnalyzer {
    /// Create a new loudness analyzer
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz (8000-384000)
    /// * `channels` - Number of channels (1-8)
    ///
    /// # Errors
    /// Returns error if sample rate or channel count is invalid
    pub fn new(sample_rate: u32, channels: u32) -> Result<Self> {
        // Validate inputs
        if !(8000..=384000).contains(&sample_rate) {
            return Err(LoudnessError::InvalidSampleRate(sample_rate));
        }
        if !(1..=8).contains(&channels) {
            return Err(LoudnessError::InvalidChannelCount(channels));
        }

        // Create EBU R128 analyzer with all measurements enabled
        // Mode::I = Integrated loudness
        // Mode::LRA = Loudness range
        // Mode::SAMPLE_PEAK = Maximum sample value
        // Mode::TRUE_PEAK = Inter-sample peak (4x oversampling)
        let mode = Mode::I | Mode::LRA | Mode::SAMPLE_PEAK | Mode::TRUE_PEAK;

        let ebur128 = EbuR128::new(channels, sample_rate, mode)?;

        Ok(Self {
            ebur128,
            sample_rate,
            channels,
            samples_processed: 0,
        })
    }

    /// Add audio frames for analysis
    ///
    /// # Arguments
    /// * `samples` - Interleaved audio samples as f32 (-1.0 to 1.0)
    ///
    /// # Notes
    /// - Samples should be interleaved (L R L R... for stereo)
    /// - Length must be divisible by channel count
    pub fn add_frames(&mut self, samples: &[f32]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        // Verify sample count is divisible by channels
        if samples.len() % self.channels as usize != 0 {
            return Err(LoudnessError::AnalysisError(format!(
                "Sample count {} is not divisible by channel count {}",
                samples.len(),
                self.channels
            )));
        }

        // Add samples to analyzer
        self.ebur128.add_frames_f32(samples)?;
        self.samples_processed += samples.len();

        Ok(())
    }

    /// Add audio frames as i16 samples
    ///
    /// # Arguments
    /// * `samples` - Interleaved audio samples as i16
    pub fn add_frames_i16(&mut self, samples: &[i16]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        if samples.len() % self.channels as usize != 0 {
            return Err(LoudnessError::AnalysisError(format!(
                "Sample count {} is not divisible by channel count {}",
                samples.len(),
                self.channels
            )));
        }

        self.ebur128.add_frames_i16(samples)?;
        self.samples_processed += samples.len();

        Ok(())
    }

    /// Add audio frames as i32 samples
    ///
    /// # Arguments
    /// * `samples` - Interleaved audio samples as i32
    pub fn add_frames_i32(&mut self, samples: &[i32]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        if samples.len() % self.channels as usize != 0 {
            return Err(LoudnessError::AnalysisError(format!(
                "Sample count {} is not divisible by channel count {}",
                samples.len(),
                self.channels
            )));
        }

        self.ebur128.add_frames_i32(samples)?;
        self.samples_processed += samples.len();

        Ok(())
    }

    /// Finalize analysis and get loudness information
    ///
    /// # Returns
    /// Loudness information including integrated loudness, range, and peak values
    ///
    /// # Errors
    /// Returns error if no samples were provided or audio is completely silent
    pub fn finalize(self) -> Result<LoudnessInfo> {
        if self.samples_processed == 0 {
            return Err(LoudnessError::NoSamples);
        }

        // Calculate total frames (samples / channels)
        let frames = self.samples_processed / self.channels as usize;
        let duration_seconds = frames as f64 / self.sample_rate as f64;

        // Get integrated loudness
        let integrated_lufs = self.ebur128.loudness_global()?;

        // Check for silent audio (ebur128 returns -inf for silence)
        if integrated_lufs.is_infinite() || integrated_lufs.is_nan() {
            return Err(LoudnessError::SilentAudio);
        }

        // Get loudness range
        let loudness_range_lu = self.ebur128.loudness_range().unwrap_or(0.0);

        // Get true peak (maximum across all channels)
        let mut true_peak_linear = 0.0_f64;
        for ch in 0..self.channels {
            let peak = self.ebur128.true_peak(ch).unwrap_or(0.0);
            if peak > true_peak_linear {
                true_peak_linear = peak;
            }
        }

        // Get sample peak (maximum across all channels)
        let mut sample_peak_linear = 0.0_f64;
        for ch in 0..self.channels {
            let peak = self.ebur128.sample_peak(ch).unwrap_or(0.0);
            if peak > sample_peak_linear {
                sample_peak_linear = peak;
            }
        }

        // Convert linear to dB
        let true_peak_dbfs = if true_peak_linear > 0.0 {
            20.0 * true_peak_linear.log10()
        } else {
            -f64::INFINITY
        };

        let sample_peak_dbfs = if sample_peak_linear > 0.0 {
            20.0 * sample_peak_linear.log10()
        } else {
            -f64::INFINITY
        };

        Ok(LoudnessInfo {
            integrated_lufs,
            loudness_range_lu,
            true_peak_dbfs,
            sample_peak_dbfs,
            duration_seconds,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }

    /// Get the current integrated loudness (can be called during analysis)
    pub fn current_loudness(&self) -> Option<f64> {
        self.ebur128.loudness_global().ok()
    }

    /// Get the number of samples processed
    pub fn samples_processed(&self) -> usize {
        self.samples_processed
    }

    /// Reset the analyzer for reuse
    pub fn reset(&mut self) {
        // Create a new EbuR128 instance (ebur128 doesn't have a reset method)
        let mode = Mode::I | Mode::LRA | Mode::SAMPLE_PEAK | Mode::TRUE_PEAK;
        if let Ok(new_analyzer) = EbuR128::new(self.channels, self.sample_rate, mode) {
            self.ebur128 = new_analyzer;
            self.samples_processed = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        // Valid parameters
        assert!(LoudnessAnalyzer::new(44100, 2).is_ok());
        assert!(LoudnessAnalyzer::new(48000, 1).is_ok());
        assert!(LoudnessAnalyzer::new(96000, 6).is_ok());

        // Invalid sample rate
        assert!(LoudnessAnalyzer::new(100, 2).is_err());
        assert!(LoudnessAnalyzer::new(500000, 2).is_err());

        // Invalid channels
        assert!(LoudnessAnalyzer::new(44100, 0).is_err());
        assert!(LoudnessAnalyzer::new(44100, 10).is_err());
    }

    #[test]
    fn test_silent_audio() {
        let mut analyzer = LoudnessAnalyzer::new(44100, 2).unwrap();
        let silence = vec![0.0_f32; 44100 * 2]; // 1 second of silence
        analyzer.add_frames(&silence).unwrap();

        // Silent audio should return an error
        assert!(matches!(
            analyzer.finalize(),
            Err(LoudnessError::SilentAudio)
        ));
    }

    #[test]
    fn test_sine_wave_loudness() {
        let mut analyzer = LoudnessAnalyzer::new(44100, 2).unwrap();

        // Generate 3 seconds of -20 dBFS sine wave (required for accurate measurement)
        // -20 dBFS = amplitude of 0.1
        let amplitude = 0.1_f32;
        let frequency = 1000.0_f32;
        let sample_rate = 44100.0_f32;
        let duration_samples = 44100 * 3; // 3 seconds

        let mut samples = Vec::with_capacity(duration_samples * 2);
        for i in 0..duration_samples {
            let t = i as f32 / sample_rate;
            let sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
            // Interleaved stereo
            samples.push(sample);
            samples.push(sample);
        }

        analyzer.add_frames(&samples).unwrap();
        let info = analyzer.finalize().unwrap();

        // A -20 dBFS sine wave should measure around -23 LUFS (due to K-weighting)
        // Allow some tolerance
        assert!(
            info.integrated_lufs > -30.0 && info.integrated_lufs < -15.0,
            "Expected loudness around -23 LUFS, got {:.1}",
            info.integrated_lufs
        );

        // True peak should be close to -20 dBFS
        assert!(
            info.true_peak_dbfs > -25.0 && info.true_peak_dbfs < -15.0,
            "Expected true peak around -20 dBFS, got {:.1}",
            info.true_peak_dbfs
        );
    }

    #[test]
    fn test_no_samples_error() {
        let analyzer = LoudnessAnalyzer::new(44100, 2).unwrap();
        assert!(matches!(analyzer.finalize(), Err(LoudnessError::NoSamples)));
    }

    #[test]
    fn test_invalid_sample_count() {
        let mut analyzer = LoudnessAnalyzer::new(44100, 2).unwrap();
        // 5 samples is not divisible by 2 channels
        let samples = vec![0.1_f32; 5];
        assert!(analyzer.add_frames(&samples).is_err());
    }

    #[test]
    fn test_loudness_info_clipping() {
        let info = LoudnessInfo {
            integrated_lufs: -14.0,
            loudness_range_lu: 5.0,
            true_peak_dbfs: -1.0,
            sample_peak_dbfs: -1.5,
            duration_seconds: 180.0,
            sample_rate: 44100,
            channels: 2,
        };

        // +2 dB gain would cause clipping (peak at -1 + 2 = +1 dBTP)
        assert!(info.will_clip_at_gain(2.0));

        // -1 dB gain is safe
        assert!(!info.will_clip_at_gain(-1.0));

        // Max safe gain is 1 dB
        assert!((info.max_safe_gain() - 1.0).abs() < 0.001);
    }
}
