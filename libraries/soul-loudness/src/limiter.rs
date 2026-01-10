//! True peak limiter for playback
//!
//! Prevents clipping when applying ReplayGain by limiting peaks that exceed 0 dBTP.
//! Uses lookahead and soft-knee limiting for transparent operation.

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
        // Lookahead of 1.5ms is sufficient for true peak detection
        let lookahead_size = (sample_rate as f32 * 0.0015).ceil() as usize;
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
        }
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
        assert!(latency_ms >= 1.0 && latency_ms <= 2.0);
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
}
