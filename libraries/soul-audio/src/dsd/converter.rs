//! DSD converter - PCM to DSD conversion using sigma-delta modulation
//!
//! Converts PCM audio to DSD format using a high-quality sigma-delta modulator
//! with configurable noise shaping.

use super::noise_shaper::{NoiseShaper, NoiseShaperOrder};

/// DSD format / rate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DsdFormat {
    /// DSD64: 2.8224 MHz (64x CD rate)
    /// Equivalent to 44100 * 64 = 2,822,400 Hz
    #[default]
    Dsd64,

    /// DSD128: 5.6448 MHz (128x CD rate)
    /// Equivalent to 44100 * 128 = 5,644,800 Hz
    Dsd128,

    /// DSD256: 11.2896 MHz (256x CD rate)
    /// Equivalent to 44100 * 256 = 11,289,600 Hz
    Dsd256,

    /// DSD512: 22.5792 MHz (512x CD rate)
    /// Equivalent to 44100 * 512 = 22,579,200 Hz
    Dsd512,
}

impl DsdFormat {
    /// Get the sample rate multiplier (relative to 44100 Hz)
    pub fn multiplier(&self) -> u32 {
        match self {
            DsdFormat::Dsd64 => 64,
            DsdFormat::Dsd128 => 128,
            DsdFormat::Dsd256 => 256,
            DsdFormat::Dsd512 => 512,
        }
    }

    /// Get the DSD sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        44100 * self.multiplier()
    }

    /// Get the DSD sample rate in MHz
    pub fn sample_rate_mhz(&self) -> f64 {
        self.sample_rate() as f64 / 1_000_000.0
    }

    /// Get a human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            DsdFormat::Dsd64 => "DSD64 (2.8 MHz)",
            DsdFormat::Dsd128 => "DSD128 (5.6 MHz)",
            DsdFormat::Dsd256 => "DSD256 (11.3 MHz)",
            DsdFormat::Dsd512 => "DSD512 (22.6 MHz)",
        }
    }

    /// Recommended noise shaper order for this format
    pub fn recommended_noise_shaper(&self) -> NoiseShaperOrder {
        match self {
            // Higher rates can use simpler shapers (more headroom)
            DsdFormat::Dsd256 | DsdFormat::Dsd512 => NoiseShaperOrder::Second,
            // Lower rates need more aggressive shaping
            DsdFormat::Dsd64 | DsdFormat::Dsd128 => NoiseShaperOrder::Third,
        }
    }
}

/// DSD conversion settings
#[derive(Debug, Clone)]
pub struct DsdSettings {
    /// DSD output format
    pub format: DsdFormat,

    /// Noise shaper order
    pub noise_shaper_order: NoiseShaperOrder,

    /// Enable dithering (adds very small noise to improve quality)
    pub dither: bool,

    /// Soft clipping threshold (0.0-1.0, clips above this)
    /// Default: 0.95 to prevent hard clipping
    pub soft_clip_threshold: f64,
}

impl Default for DsdSettings {
    fn default() -> Self {
        Self {
            format: DsdFormat::Dsd128,
            noise_shaper_order: NoiseShaperOrder::Third,
            dither: true,
            soft_clip_threshold: 0.95,
        }
    }
}

impl DsdSettings {
    /// Create settings for a specific format with recommended defaults
    pub fn for_format(format: DsdFormat) -> Self {
        Self {
            format,
            noise_shaper_order: format.recommended_noise_shaper(),
            ..Default::default()
        }
    }
}

/// DSD converter (PCM to DSD)
///
/// Converts PCM audio to DSD using sigma-delta modulation with noise shaping.
/// The output is packed DSD bits (8 DSD samples per byte).
pub struct DsdConverter {
    /// Current settings
    settings: DsdSettings,

    /// Input PCM sample rate
    pcm_sample_rate: u32,

    /// Noise shapers (one per channel)
    noise_shaper: NoiseShaper,

    /// Number of channels
    channels: usize,

    /// DSD-to-PCM ratio (how many DSD samples per PCM sample)
    oversampling_ratio: u32,

    /// Linear interpolation state per channel
    interp_last: Vec<f64>,

    /// PRNG state for dithering
    dither_state: u32,

    /// Output bit accumulator per channel
    bit_accumulators: Vec<u8>,

    /// Bits accumulated so far per channel
    bits_accumulated: Vec<u8>,
}

impl DsdConverter {
    /// Create a new DSD converter
    ///
    /// # Arguments
    /// * `format` - Target DSD format (DSD64, DSD128, etc.)
    /// * `pcm_sample_rate` - Input PCM sample rate (e.g., 44100, 48000)
    pub fn new(format: DsdFormat, pcm_sample_rate: u32) -> Self {
        Self::with_settings(DsdSettings::for_format(format), pcm_sample_rate, 2)
    }

    /// Create a new DSD converter with custom settings
    pub fn with_settings(settings: DsdSettings, pcm_sample_rate: u32, channels: usize) -> Self {
        let oversampling_ratio = settings.format.sample_rate() / pcm_sample_rate;

        Self {
            noise_shaper: NoiseShaper::new(settings.noise_shaper_order, channels),
            settings,
            pcm_sample_rate,
            channels,
            oversampling_ratio,
            interp_last: vec![0.0; channels],
            dither_state: 0x12345678,
            bit_accumulators: vec![0; channels],
            bits_accumulated: vec![0; channels],
        }
    }

    /// Process PCM samples and output packed DSD bytes
    ///
    /// # Arguments
    /// * `pcm_input` - Interleaved PCM samples (f32, -1.0 to 1.0)
    ///
    /// # Returns
    /// Packed DSD bytes (interleaved by channel, 8 DSD samples per byte)
    pub fn process_pcm(&mut self, pcm_input: &[f32]) -> Vec<u8> {
        let frames = pcm_input.len() / self.channels;
        // Each PCM frame produces (oversampling_ratio / 8) bytes per channel
        let bytes_per_frame = (self.oversampling_ratio / 8) as usize;
        let total_bytes = frames * bytes_per_frame * self.channels;

        let mut output = Vec::with_capacity(total_bytes);

        for frame_idx in 0..frames {
            let pcm_offset = frame_idx * self.channels;

            for ch in 0..self.channels {
                let current_sample = pcm_input[pcm_offset + ch] as f64;
                let last_sample = self.interp_last[ch];

                // Generate oversampling_ratio DSD bits per PCM sample
                let dsd_bytes = self.convert_sample_to_dsd(
                    last_sample,
                    current_sample,
                    ch,
                );

                output.extend_from_slice(&dsd_bytes);

                self.interp_last[ch] = current_sample;
            }
        }

        output
    }

    /// Convert one PCM sample to DSD bytes using linear interpolation
    fn convert_sample_to_dsd(&mut self, last: f64, current: f64, channel: usize) -> Vec<u8> {
        let ratio = self.oversampling_ratio as usize;
        let bytes_needed = ratio / 8;
        let mut bytes = Vec::with_capacity(bytes_needed);

        for dsd_idx in 0..ratio {
            // Linear interpolation between last and current sample
            let t = (dsd_idx as f64 + 0.5) / ratio as f64;
            let mut sample = last + (current - last) * t;

            // Apply soft clipping
            sample = self.soft_clip(sample);

            // Add dither if enabled
            if self.settings.dither {
                sample += self.generate_dither() * 0.00001; // Very small dither
            }

            // Process through noise shaper
            let shaped = self.noise_shaper.process(sample, channel);

            // 1-bit quantization
            let bit = if shaped >= 0.0 { 1u8 } else { 0u8 };

            // Feedback quantization error
            // Error is (quantized - input) so positive error pushes integrators negative
            let quantized = if bit == 1 { 1.0 } else { -1.0 };
            let error = quantized - sample;
            self.noise_shaper.feedback(error, channel);

            // Accumulate bits (MSB first for standard DSD)
            self.bit_accumulators[channel] = (self.bit_accumulators[channel] << 1) | bit;
            self.bits_accumulated[channel] += 1;

            // Output byte when 8 bits accumulated
            if self.bits_accumulated[channel] >= 8 {
                bytes.push(self.bit_accumulators[channel]);
                self.bit_accumulators[channel] = 0;
                self.bits_accumulated[channel] = 0;
            }
        }

        bytes
    }

    /// Apply soft clipping to prevent harsh distortion
    #[inline]
    fn soft_clip(&self, sample: f64) -> f64 {
        let threshold = self.settings.soft_clip_threshold;

        if sample.abs() <= threshold {
            sample
        } else {
            // Soft knee compression above threshold
            let sign = sample.signum();
            let excess = sample.abs() - threshold;
            let compressed = threshold + (1.0 - threshold) * (1.0 - (-excess * 2.0).exp());
            sign * compressed.min(1.0)
        }
    }

    /// Generate dither using simple PRNG
    #[inline]
    fn generate_dither(&mut self) -> f64 {
        // Simple xorshift PRNG
        self.dither_state ^= self.dither_state << 13;
        self.dither_state ^= self.dither_state >> 17;
        self.dither_state ^= self.dither_state << 5;

        // Convert to -1.0 to 1.0 range
        (self.dither_state as f64 / u32::MAX as f64) * 2.0 - 1.0
    }

    /// Reset converter state
    pub fn reset(&mut self) {
        self.noise_shaper.reset();
        self.interp_last.fill(0.0);
        self.bit_accumulators.fill(0);
        self.bits_accumulated.fill(0);
    }

    /// Get current settings
    pub fn settings(&self) -> &DsdSettings {
        &self.settings
    }

    /// Get DSD output format
    pub fn format(&self) -> DsdFormat {
        self.settings.format
    }

    /// Get oversampling ratio
    pub fn oversampling_ratio(&self) -> u32 {
        self.oversampling_ratio
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Get input PCM sample rate
    pub fn pcm_sample_rate(&self) -> u32 {
        self.pcm_sample_rate
    }

    /// Get output DSD sample rate
    pub fn dsd_sample_rate(&self) -> u32 {
        self.settings.format.sample_rate()
    }

    /// Calculate output size in bytes for given PCM input size
    pub fn output_size_for_pcm(&self, pcm_samples: usize) -> usize {
        let frames = pcm_samples / self.channels;
        let bytes_per_frame = (self.oversampling_ratio / 8) as usize;
        frames * bytes_per_frame * self.channels
    }
}

impl Default for DsdConverter {
    fn default() -> Self {
        Self::new(DsdFormat::Dsd128, 44100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsd_format_rates() {
        assert_eq!(DsdFormat::Dsd64.multiplier(), 64);
        assert_eq!(DsdFormat::Dsd64.sample_rate(), 2_822_400);

        assert_eq!(DsdFormat::Dsd128.multiplier(), 128);
        assert_eq!(DsdFormat::Dsd128.sample_rate(), 5_644_800);

        assert_eq!(DsdFormat::Dsd256.multiplier(), 256);
        assert_eq!(DsdFormat::Dsd256.sample_rate(), 11_289_600);
    }

    #[test]
    fn test_dsd_format_display() {
        assert_eq!(DsdFormat::Dsd64.display_name(), "DSD64 (2.8 MHz)");
        assert_eq!(DsdFormat::Dsd128.display_name(), "DSD128 (5.6 MHz)");
    }

    #[test]
    fn test_converter_creation() {
        let converter = DsdConverter::new(DsdFormat::Dsd128, 44100);
        assert_eq!(converter.format(), DsdFormat::Dsd128);
        assert_eq!(converter.pcm_sample_rate(), 44100);
        assert_eq!(converter.dsd_sample_rate(), 5_644_800);
        assert_eq!(converter.oversampling_ratio(), 128);
    }

    #[test]
    fn test_converter_with_48khz() {
        let converter = DsdConverter::new(DsdFormat::Dsd64, 48000);
        // 2,822,400 / 48000 = 58.8, but we use integer division
        // Note: In practice, you'd want to resample to 44100 first
        // or use a rate that divides evenly
        assert_eq!(converter.pcm_sample_rate(), 48000);
    }

    #[test]
    fn test_process_silence() {
        let mut converter = DsdConverter::new(DsdFormat::Dsd64, 44100);

        // Process silent stereo samples (4 frames = 8 samples)
        let silence = vec![0.0f32; 8];
        let output = converter.process_pcm(&silence);

        // Should produce output
        assert!(!output.is_empty());

        // With silence input, output should be roughly balanced
        // (noise shaping might produce some variation)
    }

    #[test]
    fn test_process_sine() {
        // Generate a simple sine wave (mono for simplicity in verification)
        let mut converter = DsdConverter::with_settings(
            DsdSettings::for_format(DsdFormat::Dsd64),
            44100,
            1, // mono
        );

        let num_samples = 100;
        let pcm: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / 44100.0;
                let freq = 1000.0;
                (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5
            })
            .collect();

        let output = converter.process_pcm(&pcm);

        // Should produce output
        assert!(!output.is_empty());

        // Check expected output size
        // 100 samples * 64 DSD samples per PCM sample / 8 bits per byte = 800 bytes
        let expected_size = 100 * 64 / 8;
        assert_eq!(output.len(), expected_size);
    }

    #[test]
    fn test_output_size_calculation() {
        let converter = DsdConverter::new(DsdFormat::Dsd128, 44100);

        // 1000 stereo samples
        let size = converter.output_size_for_pcm(2000);

        // 1000 frames * 128 / 8 bytes per frame * 2 channels
        let expected = 1000 * (128 / 8) * 2;
        assert_eq!(size, expected);
    }

    #[test]
    fn test_soft_clipping() {
        let converter = DsdConverter::default();

        // Below threshold
        let clipped = converter.soft_clip(0.5);
        assert!((clipped - 0.5).abs() < 0.001);

        // Above threshold - should be compressed
        let clipped = converter.soft_clip(1.5);
        assert!(clipped < 1.5);
        assert!(clipped <= 1.0);
    }

    #[test]
    fn test_reset() {
        let mut converter = DsdConverter::default();

        // Process some audio
        let pcm = vec![0.5f32; 100];
        converter.process_pcm(&pcm);

        // Reset
        converter.reset();

        // Should be able to process again without issues
        let output = converter.process_pcm(&pcm);
        assert!(!output.is_empty());
    }

    #[test]
    fn test_settings() {
        let settings = DsdSettings {
            format: DsdFormat::Dsd256,
            noise_shaper_order: NoiseShaperOrder::Fifth,
            dither: false,
            soft_clip_threshold: 0.9,
        };

        let converter = DsdConverter::with_settings(settings.clone(), 44100, 2);
        assert_eq!(converter.format(), DsdFormat::Dsd256);
        assert!(!converter.settings().dither);
    }
}
