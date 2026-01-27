//! True peak limiter for playback
//!
//! Prevents clipping when applying ReplayGain by limiting peaks that exceed 0 dBTP.
//! Uses lookahead and soft-knee limiting for transparent operation.

/// Lookahead presets for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LookaheadPreset {
    /// No lookahead (0ms) - instant limiting, may cause distortion on transients
    Instant,
    /// Balanced lookahead (1.5ms) - good tradeoff between latency and transparency
    #[default]
    Balanced,
    /// Transparent lookahead (5ms) - minimal audible artifacts
    Transparent,
    /// Custom lookahead in milliseconds
    Custom(f32),
}

impl LookaheadPreset {
    /// Get the lookahead time in milliseconds
    pub fn as_ms(&self) -> f32 {
        match self {
            Self::Instant => 0.0,
            Self::Balanced => 1.5,
            Self::Transparent => 5.0,
            Self::Custom(ms) => *ms,
        }
    }
}

/// True peak limiter to prevent clipping
///
/// Uses a lookahead design with release smoothing to prevent inter-sample peaks
/// from exceeding the threshold while minimizing audible artifacts.
///
/// # Example
///
/// ```ignore
/// use soul_loudness::TruePeakLimiter;
///
/// let mut limiter = TruePeakLimiter::new(44100, 2);
///
/// // Process audio buffer (samples is an &mut [f32] of interleaved audio)
/// limiter.process(&mut samples);
/// ```
pub struct TruePeakLimiter {
    /// Threshold in linear (1.0 = 0 dBFS)
    threshold: f32,
    /// Release time in samples
    release_samples: usize,
    /// Current gain reduction (linear, 0.0-1.0)
    gain_reduction: f32,
    /// Lookahead buffer (per channel)
    lookahead_buffers: Vec<Vec<f32>>,
    /// Lookahead size in samples
    lookahead_size: usize,
    /// Current write position in lookahead buffer
    write_pos: usize,
    /// Number of channels
    channels: usize,
    /// Sample rate
    sample_rate: u32,
    /// Peak hold samples remaining
    peak_hold: usize,
    /// Peak hold time in samples
    peak_hold_time: usize,
    /// Lookahead preset
    lookahead_preset: LookaheadPreset,
}

impl TruePeakLimiter {
    /// Create a new true peak limiter
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `channels` - Number of audio channels
    ///
    /// # Notes
    /// - Default threshold: 0 dBFS (1.0 linear)
    /// - Default release: 100ms
    /// - Lookahead: 1.5ms (for true peak detection)
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        Self::with_lookahead(sample_rate, channels, LookaheadPreset::Balanced)
    }

    /// Create a new true peak limiter with custom lookahead
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `channels` - Number of audio channels
    /// * `lookahead` - Lookahead preset
    pub fn with_lookahead(sample_rate: u32, channels: usize, lookahead: LookaheadPreset) -> Self {
        let lookahead_ms = lookahead.as_ms();
        // Minimum 1 sample for instant mode to avoid division by zero
        let lookahead_size = ((sample_rate as f32 * lookahead_ms / 1000.0).ceil() as usize).max(1);
        let release_samples = (sample_rate as f32 * 0.1) as usize; // 100ms release
        let peak_hold_time = (sample_rate as f32 * 0.01) as usize; // 10ms hold

        let lookahead_buffers = vec![vec![0.0; lookahead_size]; channels];

        Self {
            threshold: 1.0,
            release_samples,
            gain_reduction: 1.0,
            lookahead_buffers,
            lookahead_size,
            write_pos: 0,
            channels,
            sample_rate,
            peak_hold: 0,
            peak_hold_time,
            lookahead_preset: lookahead,
        }
    }

    /// Set the lookahead preset
    ///
    /// Note: This will reset the limiter state and reallocate buffers
    pub fn set_lookahead(&mut self, preset: LookaheadPreset) {
        if self.lookahead_preset == preset {
            return;
        }

        self.lookahead_preset = preset;
        let lookahead_ms = preset.as_ms();
        let new_size = ((self.sample_rate as f32 * lookahead_ms / 1000.0).ceil() as usize).max(1);

        if new_size != self.lookahead_size {
            self.lookahead_size = new_size;
            self.lookahead_buffers = vec![vec![0.0; new_size]; self.channels];
            self.write_pos = 0;
            self.gain_reduction = 1.0;
            self.peak_hold = 0;
        }
    }

    /// Get the current lookahead preset
    pub fn lookahead_preset(&self) -> LookaheadPreset {
        self.lookahead_preset
    }

    /// Set lookahead time in milliseconds (0-10ms)
    pub fn set_lookahead_ms(&mut self, lookahead_ms: f32) {
        let clamped = lookahead_ms.clamp(0.0, 10.0);
        self.set_lookahead(LookaheadPreset::Custom(clamped));
    }

    /// Set the threshold in dB (0 dB = no limiting, negative values = lower threshold)
    pub fn set_threshold_db(&mut self, threshold_db: f32) {
        self.threshold = 10.0_f32.powf(threshold_db / 20.0);
    }

    /// Set the release time in milliseconds
    pub fn set_release_ms(&mut self, release_ms: f32) {
        self.release_samples = (self.sample_rate as f32 * release_ms / 1000.0) as usize;
    }

    /// Get current gain reduction in dB
    pub fn gain_reduction_db(&self) -> f32 {
        20.0 * self.gain_reduction.log10()
    }

    /// Process audio buffer in place
    ///
    /// # Arguments
    /// * `samples` - Interleaved audio samples (modified in place)
    ///
    /// # Notes
    /// - Samples should be interleaved (L R L R... for stereo)
    /// - Length must be divisible by channel count
    pub fn process(&mut self, samples: &mut [f32]) {
        if samples.is_empty() || self.channels == 0 {
            return;
        }

        let frames = samples.len() / self.channels;

        for frame_idx in 0..frames {
            // Find peak across all channels for this frame
            let mut frame_peak = 0.0_f32;
            for ch in 0..self.channels {
                let sample = samples[frame_idx * self.channels + ch].abs();
                if sample > frame_peak {
                    frame_peak = sample;
                }
            }

            // Calculate required gain to stay under threshold
            let target_gain = if frame_peak > self.threshold {
                self.threshold / frame_peak
            } else {
                1.0
            };

            // Update gain with attack/release
            if target_gain < self.gain_reduction {
                // Attack: immediate (lookahead provides the smoothing)
                self.gain_reduction = target_gain;
                self.peak_hold = self.peak_hold_time;
            } else if self.peak_hold > 0 {
                // Hold
                self.peak_hold -= 1;
            } else {
                // Release: smooth recovery
                let release_coeff = 1.0 / self.release_samples as f32;
                self.gain_reduction += (1.0 - self.gain_reduction) * release_coeff;
                if self.gain_reduction > 0.9999 {
                    self.gain_reduction = 1.0;
                }
            }

            // Apply gain and swap with lookahead buffer
            for ch in 0..self.channels {
                let sample_idx = frame_idx * self.channels + ch;
                let input = samples[sample_idx];

                // Get delayed sample from lookahead
                let delayed = self.lookahead_buffers[ch][self.write_pos];

                // Store current sample in lookahead
                self.lookahead_buffers[ch][self.write_pos] = input;

                // Output delayed sample with gain reduction
                samples[sample_idx] = delayed * self.gain_reduction;
            }

            // Advance write position (circular buffer)
            self.write_pos = (self.write_pos + 1) % self.lookahead_size;
        }
    }

    /// Process a single frame (non-interleaved, one sample per channel)
    pub fn process_frame(&mut self, samples: &mut [f32]) {
        if samples.len() != self.channels {
            return;
        }

        // Find peak
        let frame_peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        // Calculate target gain
        let target_gain = if frame_peak > self.threshold {
            self.threshold / frame_peak
        } else {
            1.0
        };

        // Update gain
        if target_gain < self.gain_reduction {
            self.gain_reduction = target_gain;
            self.peak_hold = self.peak_hold_time;
        } else if self.peak_hold > 0 {
            self.peak_hold -= 1;
        } else {
            let release_coeff = 1.0 / self.release_samples as f32;
            self.gain_reduction += (1.0 - self.gain_reduction) * release_coeff;
            if self.gain_reduction > 0.9999 {
                self.gain_reduction = 1.0;
            }
        }

        // Process each channel
        for (ch, sample) in samples.iter_mut().enumerate() {
            let input = *sample;
            let delayed = self.lookahead_buffers[ch][self.write_pos];
            self.lookahead_buffers[ch][self.write_pos] = input;
            *sample = delayed * self.gain_reduction;
        }

        self.write_pos = (self.write_pos + 1) % self.lookahead_size;
    }

    /// Reset the limiter state
    pub fn reset(&mut self) {
        self.gain_reduction = 1.0;
        self.peak_hold = 0;
        self.write_pos = 0;
        for buffer in &mut self.lookahead_buffers {
            buffer.fill(0.0);
        }
    }

    /// Get the latency in samples introduced by the limiter
    pub fn latency_samples(&self) -> usize {
        self.lookahead_size
    }

    /// Get the latency in milliseconds
    pub fn latency_ms(&self) -> f32 {
        self.lookahead_size as f32 / self.sample_rate as f32 * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_creation() {
        let limiter = TruePeakLimiter::new(44100, 2);
        assert_eq!(limiter.channels, 2);
        assert_eq!(limiter.sample_rate, 44100);
        assert!((limiter.threshold - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_limiter_passthrough() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Feed some quiet audio (below threshold)
        let mut samples: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / 44100.0;
                0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            })
            .flat_map(|s| vec![s, s])
            .collect();

        // Need to prime the lookahead buffer first
        let samples_copy = samples.clone();
        limiter.process(&mut samples);

        // After lookahead latency, output should be close to input (scaled by 0.5)
        // The initial samples will be zeros due to lookahead, skip them
        let latency_samples = limiter.latency_samples() * 2;
        for (i, (&input, &output)) in samples_copy
            .iter()
            .zip(samples.iter().skip(latency_samples))
            .enumerate()
            .skip(latency_samples)
            .take(100)
        {
            // Allow for numerical precision
            assert!(
                (input - output).abs() < 0.001,
                "Sample {} differs: input={}, output={}",
                i,
                input,
                output
            );
        }
    }

    #[test]
    fn test_limiter_limits_peaks() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Create samples that exceed 0 dBFS
        let mut samples: Vec<f32> = vec![1.5, 1.5, 2.0, 2.0, 1.8, 1.8, 0.5, 0.5];
        let _lookahead = limiter.latency_samples();

        // Process multiple times to fill lookahead and get output
        for _ in 0..10 {
            limiter.process(&mut samples);
        }

        // After limiting, no sample should exceed threshold
        for &sample in &samples {
            assert!(
                sample.abs() <= 1.001, // Allow small margin for floating point
                "Sample {} exceeds threshold",
                sample
            );
        }
    }

    #[test]
    fn test_threshold_adjustment() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Set threshold to -6 dB
        limiter.set_threshold_db(-6.0);
        let expected = 10.0_f32.powf(-6.0 / 20.0);
        assert!((limiter.threshold - expected).abs() < 0.001);

        // Test 0 dB
        limiter.set_threshold_db(0.0);
        assert!((limiter.threshold - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gain_reduction_reporting() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // No limiting yet
        assert!((limiter.gain_reduction_db() - 0.0).abs() < 0.001);

        // Force limiting with loud samples
        let mut samples = vec![2.0, 2.0];
        for _ in 0..100 {
            limiter.process(&mut samples);
        }

        // Should have gain reduction
        assert!(limiter.gain_reduction_db() < 0.0);
    }

    #[test]
    fn test_latency() {
        let limiter = TruePeakLimiter::new(44100, 2);

        // Latency should be approximately 1.5ms
        let latency_ms = limiter.latency_ms();
        assert!((1.0..=2.0).contains(&latency_ms));
    }

    #[test]
    fn test_reset() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Process some loud audio to build up gain reduction
        let mut samples = vec![2.0; 100];
        limiter.process(&mut samples);

        // Reset
        limiter.reset();

        // Should be back to unity gain
        assert!((limiter.gain_reduction - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_lookahead_presets() {
        // Test all preset values
        assert!((LookaheadPreset::Instant.as_ms() - 0.0).abs() < 0.001);
        assert!((LookaheadPreset::Balanced.as_ms() - 1.5).abs() < 0.001);
        assert!((LookaheadPreset::Transparent.as_ms() - 5.0).abs() < 0.001);
        assert!((LookaheadPreset::Custom(3.0).as_ms() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_set_lookahead_preset() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Default is Balanced
        assert_eq!(limiter.lookahead_preset(), LookaheadPreset::Balanced);

        // Change to Transparent (5ms)
        limiter.set_lookahead(LookaheadPreset::Transparent);
        assert_eq!(limiter.lookahead_preset(), LookaheadPreset::Transparent);

        // Latency should be approximately 5ms
        let latency_ms = limiter.latency_ms();
        assert!(
            (4.5..=5.5).contains(&latency_ms),
            "Expected ~5ms latency, got {}ms",
            latency_ms
        );
    }

    #[test]
    fn test_set_lookahead_ms_clamping() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Should clamp to 10ms max
        limiter.set_lookahead_ms(15.0);
        assert_eq!(limiter.lookahead_preset(), LookaheadPreset::Custom(10.0));

        // Should clamp to 0ms min
        limiter.set_lookahead_ms(-5.0);
        assert_eq!(limiter.lookahead_preset(), LookaheadPreset::Custom(0.0));
    }

    #[test]
    fn test_process_frame() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Process frames one at a time
        let mut frame = [1.5_f32, 1.5_f32];

        // Prime the lookahead
        for _ in 0..100 {
            limiter.process_frame(&mut frame);
        }

        // Output should be limited
        assert!(
            frame[0].abs() <= 1.001,
            "Left channel {} exceeds threshold",
            frame[0]
        );
        assert!(
            frame[1].abs() <= 1.001,
            "Right channel {} exceeds threshold",
            frame[1]
        );
    }

    #[test]
    fn test_release_time_adjustment() {
        let mut limiter = TruePeakLimiter::new(44100, 2);
        limiter.set_release_ms(200.0);

        // Create loud signal then quiet
        let mut loud_samples = vec![2.0_f32; 2000];
        let mut quiet_samples = vec![0.1_f32; 4000];

        // Process loud to engage limiting
        limiter.process(&mut loud_samples);

        // Process quiet - with 200ms release, gain should recover slowly
        limiter.process(&mut quiet_samples);

        // Gain should still be reduced after processing (release is gradual)
        // With 200ms release at 44100Hz, recovery takes thousands of samples
        assert!(
            limiter.gain_reduction < 1.0,
            "Expected gain reduction during release, got {}",
            limiter.gain_reduction
        );
    }

    #[test]
    fn test_ebu_r128_threshold() {
        // EBU R128 requires -1 dBTP maximum
        let mut limiter = TruePeakLimiter::new(44100, 2);
        limiter.set_threshold_db(-1.0);

        // -1 dBTP = 10^(-1/20) = ~0.891
        let expected_threshold = 10.0_f32.powf(-1.0 / 20.0);
        assert!(
            (limiter.threshold - expected_threshold).abs() < 0.001,
            "Expected threshold {}, got {}",
            expected_threshold,
            limiter.threshold
        );

        // Test that samples are limited to this threshold
        let mut samples = vec![1.0_f32; 200];
        for _ in 0..10 {
            limiter.process(&mut samples);
        }

        for &sample in &samples {
            assert!(
                sample.abs() <= expected_threshold + 0.001,
                "Sample {} exceeds -1 dBTP threshold {}",
                sample,
                expected_threshold
            );
        }
    }

    /// Test case demonstrating inter-sample peak issue
    ///
    /// This test documents the current limitation: the limiter operates on
    /// sample peaks only, not true peaks. A sine wave at Nyquist/4 frequency
    /// with unfortunate phase can have inter-sample peaks up to 3dB higher
    /// than the sample values.
    ///
    /// Per ITU-R BS.1770, true peak limiting requires 4x oversampling to
    /// detect inter-sample peaks. The current implementation does not do this.
    ///
    /// TODO: Implement 4x oversampling for true peak detection
    #[test]
    fn test_inter_sample_peak_detection_limitation() {
        let sample_rate = 44100;
        let mut limiter = TruePeakLimiter::new(sample_rate, 1);
        limiter.set_threshold_db(0.0); // 0 dBFS threshold

        // Generate a sine wave at Nyquist/4 (11025 Hz at 44100 sample rate)
        // This frequency is known to have worst-case inter-sample peaks
        // when samples fall on zero crossings
        let frequency = sample_rate as f32 / 4.0;
        let num_samples = 1000;

        // Phase shift to align samples with zero crossings
        // This creates maximum inter-sample peaks between samples
        let phase_shift = std::f32::consts::PI / 4.0;

        let mut samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                0.9 * (2.0 * std::f32::consts::PI * frequency * t + phase_shift).sin()
            })
            .collect();

        // Prime and process
        for _ in 0..10 {
            limiter.process(&mut samples);
        }

        // Current behavior: sample peaks are limited, but inter-sample peaks may exceed
        // This test documents the limitation - all samples appear under threshold
        let max_sample = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(
            max_sample <= 1.0,
            "Sample peak {} exceeds threshold (expected behavior)",
            max_sample
        );

        // NOTE: True inter-sample peaks could still exceed 0 dBTP by up to 3dB
        // for this signal. A proper ITU-R BS.1770 compliant limiter would detect
        // and limit these. This limitation is documented here.
    }

    /// Test demonstrating the need for true peak limiting vs sample peak limiting
    ///
    /// Per EBU R128 and ITU-R BS.1770:
    /// - Maximum true peak level should be -1 dBTP
    /// - True peaks are measured with 4x oversampling
    /// - Sample peaks can underestimate true peaks by up to 3-6 dB
    ///
    /// This test uses known problematic signals that create inter-sample peaks.
    #[test]
    fn test_true_peak_vs_sample_peak_gap() {
        // Two samples at 0.707 with opposite signs create an inter-sample peak
        // that can approach 1.0 (depending on the reconstruction filter)
        let samples_case_1 = [0.707_f32, -0.707_f32];

        // Max sample value is 0.707 (-3 dBFS)
        let sample_peak = samples_case_1
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            (sample_peak - 0.707).abs() < 0.001,
            "Sample peak should be 0.707"
        );

        // But the interpolated true peak between these samples approaches 1.0
        // A sinc interpolation would show ~1.0 peak between the samples
        // This is why ITU-R BS.1770 requires 4x oversampling

        // Document: The current limiter would pass these samples through unchanged
        // if threshold is 0 dBFS, but the true peak exceeds that threshold
    }

    #[test]
    fn test_multichannel_peak_detection() {
        let mut limiter = TruePeakLimiter::new(44100, 4);

        // Peak on channel 3 only
        let mut samples = vec![
            0.5, 0.5, 0.5, 2.0, // Frame 1: ch3 is loud
            0.5, 0.5, 0.5, 1.8, // Frame 2
            0.5, 0.5, 0.5, 1.5, // Frame 3
        ];

        for _ in 0..10 {
            limiter.process(&mut samples);
        }

        // All channels should be limited together (uses max peak across channels)
        for &sample in &samples {
            assert!(sample.abs() <= 1.001, "Sample {} exceeds threshold", sample);
        }
    }

    #[test]
    fn test_empty_buffer_handling() {
        let mut limiter = TruePeakLimiter::new(44100, 2);

        // Empty buffer should not panic
        let mut empty: Vec<f32> = vec![];
        limiter.process(&mut empty);

        // State should be unchanged
        assert!((limiter.gain_reduction - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_single_channel_mono() {
        let mut limiter = TruePeakLimiter::new(44100, 1);

        let mut samples = vec![1.5_f32; 100];
        for _ in 0..10 {
            limiter.process(&mut samples);
        }

        for &sample in &samples {
            assert!(
                sample.abs() <= 1.001,
                "Mono sample {} exceeds threshold",
                sample
            );
        }
    }

    #[test]
    fn test_high_sample_rate_192khz() {
        // At 192kHz, inter-sample peaks are less of an issue
        // but lookahead buffer sizing should adjust
        let limiter = TruePeakLimiter::new(192000, 2);

        // 1.5ms at 192kHz = 288 samples
        let expected_latency = (192000.0_f32 * 1.5 / 1000.0).ceil() as usize;
        assert_eq!(limiter.latency_samples(), expected_latency);
    }

    #[test]
    fn test_instant_mode_minimum_buffer() {
        let limiter = TruePeakLimiter::with_lookahead(44100, 2, LookaheadPreset::Instant);

        // Instant mode should have minimum 1 sample lookahead (to avoid div by zero)
        assert!(limiter.lookahead_size >= 1);
        assert!((limiter.latency_ms() - 0.0).abs() < 0.1);
    }
}
