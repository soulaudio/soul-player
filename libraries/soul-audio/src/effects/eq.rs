/// 3-Band Parametric Equalizer
///
/// Provides low, mid, and high frequency band control with adjustable gain.
/// Uses biquad filters for each band.

use super::chain::AudioEffect;

/// EQ band configuration
#[derive(Debug, Clone, Copy)]
pub struct EqBand {
    /// Center frequency in Hz
    pub frequency: f32,
    /// Gain in dB (-12 to +12)
    pub gain_db: f32,
    /// Q factor (0.1 to 10.0), controls bandwidth
    pub q: f32,
}

impl EqBand {
    /// Create a new EQ band
    pub fn new(frequency: f32, gain_db: f32, q: f32) -> Self {
        Self {
            frequency,
            gain_db: gain_db.clamp(-12.0, 12.0),
            q: q.clamp(0.1, 10.0),
        }
    }

    /// Create a low shelf filter (boosts/cuts below frequency)
    pub fn low_shelf(frequency: f32, gain_db: f32) -> Self {
        Self::new(frequency, gain_db, 0.707) // Butterworth Q
    }

    /// Create a peaking filter (boosts/cuts around frequency)
    pub fn peaking(frequency: f32, gain_db: f32, q: f32) -> Self {
        Self::new(frequency, gain_db, q)
    }

    /// Create a high shelf filter (boosts/cuts above frequency)
    pub fn high_shelf(frequency: f32, gain_db: f32) -> Self {
        Self::new(frequency, gain_db, 0.707)
    }
}

/// Biquad filter implementation
/// Used internally by the EQ for each band
#[derive(Debug, Clone)]
struct BiquadFilter {
    // Filter coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // State variables (per channel)
    x1_l: f32,
    x2_l: f32,
    y1_l: f32,
    y2_l: f32,

    x1_r: f32,
    x2_r: f32,
    y1_r: f32,
    y2_r: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter with neutral coefficients
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1_l: 0.0,
            x2_l: 0.0,
            y1_l: 0.0,
            y2_l: 0.0,
            x1_r: 0.0,
            x2_r: 0.0,
            y1_r: 0.0,
            y2_r: 0.0,
        }
    }

    /// Configure as peaking EQ filter
    fn set_peaking(&mut self, sample_rate: f32, frequency: f32, q: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0); // Amplitude
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha / a;

        // Normalize
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Configure as low shelf filter
    fn set_low_shelf(&mut self, sample_rate: f32, frequency: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / 2.0 * ((a + 1.0 / a) * (1.0 / 0.707 - 1.0) + 2.0).sqrt();
        let beta = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + beta);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - beta);
        let a0 = (a + 1.0) + (a - 1.0) * cos_omega + beta;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) + (a - 1.0) * cos_omega - beta;

        // Normalize
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Configure as high shelf filter
    fn set_high_shelf(&mut self, sample_rate: f32, frequency: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / 2.0 * ((a + 1.0 / a) * (1.0 / 0.707 - 1.0) + 2.0).sqrt();
        let beta = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + beta);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - beta);
        let a0 = (a + 1.0) - (a - 1.0) * cos_omega + beta;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) - (a - 1.0) * cos_omega - beta;

        // Normalize
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Process a stereo sample pair (left, right)
    #[inline]
    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Process left channel
        let out_l = self.b0 * left + self.b1 * self.x1_l + self.b2 * self.x2_l
            - self.a1 * self.y1_l
            - self.a2 * self.y2_l;

        self.x2_l = self.x1_l;
        self.x1_l = left;
        self.y2_l = self.y1_l;
        self.y1_l = out_l;

        // Process right channel
        let out_r = self.b0 * right + self.b1 * self.x1_r + self.b2 * self.x2_r
            - self.a1 * self.y1_r
            - self.a2 * self.y2_r;

        self.x2_r = self.x1_r;
        self.x1_r = right;
        self.y2_r = self.y1_r;
        self.y1_r = out_r;

        (out_l, out_r)
    }

    /// Reset filter state
    fn reset(&mut self) {
        self.x1_l = 0.0;
        self.x2_l = 0.0;
        self.y1_l = 0.0;
        self.y2_l = 0.0;
        self.x1_r = 0.0;
        self.x2_r = 0.0;
        self.y1_r = 0.0;
        self.y2_r = 0.0;
    }
}

/// 3-Band Parametric Equalizer
pub struct ParametricEq {
    // Filter banks
    low_filter: BiquadFilter,
    mid_filter: BiquadFilter,
    high_filter: BiquadFilter,

    // Band configurations
    low_band: EqBand,
    mid_band: EqBand,
    high_band: EqBand,

    // State
    enabled: bool,
    sample_rate: u32,
    needs_update: bool,
}

impl ParametricEq {
    /// Create a new 3-band parametric EQ with default settings
    ///
    /// Default bands:
    /// - Low: 80 Hz shelf
    /// - Mid: 1000 Hz peaking
    /// - High: 8000 Hz shelf
    pub fn new() -> Self {
        Self {
            low_filter: BiquadFilter::new(),
            mid_filter: BiquadFilter::new(),
            high_filter: BiquadFilter::new(),
            low_band: EqBand::low_shelf(80.0, 0.0),
            mid_band: EqBand::peaking(1000.0, 0.0, 1.0),
            high_band: EqBand::high_shelf(8000.0, 0.0),
            enabled: true,
            sample_rate: 44100,
            needs_update: true,
        }
    }

    /// Set low band parameters
    pub fn set_low_band(&mut self, band: EqBand) {
        self.low_band = band;
        self.needs_update = true;
    }

    /// Set mid band parameters
    pub fn set_mid_band(&mut self, band: EqBand) {
        self.mid_band = band;
        self.needs_update = true;
    }

    /// Set high band parameters
    pub fn set_high_band(&mut self, band: EqBand) {
        self.high_band = band;
        self.needs_update = true;
    }

    /// Get low band parameters
    pub fn low_band(&self) -> EqBand {
        self.low_band
    }

    /// Get mid band parameters
    pub fn mid_band(&self) -> EqBand {
        self.mid_band
    }

    /// Get high band parameters
    pub fn high_band(&self) -> EqBand {
        self.high_band
    }

    /// Update filter coefficients if needed
    fn update_filters(&mut self) {
        if self.needs_update {
            let sr = self.sample_rate as f32;

            self.low_filter
                .set_low_shelf(sr, self.low_band.frequency, self.low_band.gain_db);

            self.mid_filter.set_peaking(
                sr,
                self.mid_band.frequency,
                self.mid_band.q,
                self.mid_band.gain_db,
            );

            self.high_filter
                .set_high_shelf(sr, self.high_band.frequency, self.high_band.gain_db);

            self.needs_update = false;
        }
    }
}

impl Default for ParametricEq {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for ParametricEq {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        // Bypass if disabled
        if !self.enabled {
            return;
        }

        // Update sample rate if changed
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.needs_update = true;
        }

        // Update filters if parameters changed
        self.update_filters();

        // Process interleaved stereo buffer
        for chunk in buffer.chunks_exact_mut(2) {
            let left = chunk[0];
            let right = chunk[1];

            // Process through all three filters in series
            let (l, r) = self.low_filter.process_sample(left, right);
            let (l, r) = self.mid_filter.process_sample(l, r);
            let (l, r) = self.high_filter.process_sample(l, r);

            chunk[0] = l;
            chunk[1] = r;
        }
    }

    fn reset(&mut self) {
        self.low_filter.reset();
        self.mid_filter.reset();
        self.high_filter.reset();
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "3-Band Parametric EQ"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_eq() {
        let eq = ParametricEq::new();
        assert!(eq.is_enabled());
        assert_eq!(eq.name(), "3-Band Parametric EQ");
    }

    #[test]
    fn eq_band_clamping() {
        let band = EqBand::new(1000.0, 20.0, 0.01); // Out of range
        assert!(band.gain_db <= 12.0);
        assert!(band.q >= 0.1);
    }

    #[test]
    fn set_bands() {
        let mut eq = ParametricEq::new();

        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        eq.set_mid_band(EqBand::peaking(1500.0, -2.0, 2.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 4.0));

        assert_eq!(eq.low_band().frequency, 100.0);
        assert_eq!(eq.mid_band().frequency, 1500.0);
        assert_eq!(eq.high_band().frequency, 10000.0);
    }

    #[test]
    fn process_buffer() {
        let mut eq = ParametricEq::new();

        // Boost low band
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

        let mut buffer = crate::effects::tests::generate_sine(100.0, 44100, 0.1);
        let original = buffer.clone();

        eq.process(&mut buffer, 44100);

        // Signal should be modified
        assert_ne!(buffer, original);
    }

    #[test]
    fn reset_clears_state() {
        let mut eq = ParametricEq::new();

        // Process some samples
        let mut buffer = vec![1.0; 100];
        eq.process(&mut buffer, 44100);

        // Reset
        eq.reset();

        // Process again - should not have residual state
        let mut buffer2 = vec![1.0; 100];
        eq.process(&mut buffer2, 44100);

        // Results should be identical (deterministic)
        assert_eq!(buffer, buffer2);
    }

    #[test]
    fn disabled_eq_bypassed() {
        let mut eq = ParametricEq::new();
        eq.set_enabled(false);

        // Extreme boost
        eq.set_low_band(EqBand::low_shelf(100.0, 12.0));

        let mut buffer = vec![0.5; 100];
        let original = buffer.clone();

        eq.process(&mut buffer, 44100);

        // Should be unchanged (EQ disabled)
        assert_eq!(buffer, original);
    }

    #[test]
    fn eq_band_helpers() {
        let low = EqBand::low_shelf(80.0, 3.0);
        assert_eq!(low.frequency, 80.0);
        assert_eq!(low.gain_db, 3.0);

        let mid = EqBand::peaking(1000.0, -2.0, 2.0);
        assert_eq!(mid.frequency, 1000.0);
        assert_eq!(mid.q, 2.0);

        let high = EqBand::high_shelf(10000.0, 4.0);
        assert_eq!(high.frequency, 10000.0);
    }
}
