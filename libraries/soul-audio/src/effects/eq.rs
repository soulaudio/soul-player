/// Parametric Equalizer
///
/// Provides flexible frequency band control with adjustable gain.
/// Uses biquad filters for each band. Supports 1-8 bands dynamically.
use super::chain::AudioEffect;

/// Maximum number of bands supported by the dynamic EQ
pub const MAX_EQ_BANDS: usize = 8;

/// Filter type for EQ bands
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    /// Low shelf - boosts/cuts below frequency
    LowShelf,
    /// Peaking - boosts/cuts around frequency with Q bandwidth
    Peaking,
    /// High shelf - boosts/cuts above frequency
    HighShelf,
}

impl Default for FilterType {
    fn default() -> Self {
        Self::Peaking
    }
}

/// EQ band configuration
#[derive(Debug, Clone, Copy)]
pub struct EqBand {
    /// Center frequency in Hz
    pub frequency: f32,
    /// Gain in dB (-24 to +24) - Made private to enforce validation
    gain_db: f32,
    /// Q factor (0.1 to 10.0), controls bandwidth - Made private to enforce validation
    q: f32,
    /// Filter type (shelf or peaking)
    filter_type: FilterType,
}

impl EqBand {
    /// Create a new EQ band (defaults to peaking filter)
    pub fn new(frequency: f32, gain_db: f32, q: f32) -> Self {
        Self {
            frequency,
            gain_db: gain_db.clamp(-24.0, 24.0),
            q: q.clamp(0.1, 10.0),
            filter_type: FilterType::Peaking,
        }
    }

    /// Get the gain in dB
    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    /// Set the gain in dB (clamped to -24 to +24)
    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db.clamp(-24.0, 24.0);
    }

    /// Get the Q factor
    pub fn q(&self) -> f32 {
        self.q
    }

    /// Set the Q factor (clamped to 0.1 to 10.0)
    pub fn set_q(&mut self, q: f32) {
        self.q = q.clamp(0.1, 10.0);
    }

    /// Get the filter type
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    /// Create a low shelf filter (boosts/cuts below frequency)
    pub fn low_shelf(frequency: f32, gain_db: f32) -> Self {
        Self {
            frequency,
            gain_db: gain_db.clamp(-24.0, 24.0),
            q: 0.707, // Butterworth Q
            filter_type: FilterType::LowShelf,
        }
    }

    /// Create a peaking filter (boosts/cuts around frequency)
    pub fn peaking(frequency: f32, gain_db: f32, q: f32) -> Self {
        Self::new(frequency, gain_db, q)
    }

    /// Create a high shelf filter (boosts/cuts above frequency)
    pub fn high_shelf(frequency: f32, gain_db: f32) -> Self {
        Self {
            frequency,
            gain_db: gain_db.clamp(-24.0, 24.0),
            q: 0.707, // Butterworth Q
            filter_type: FilterType::HighShelf,
        }
    }
}

/// Smoothing coefficient for exponential coefficient interpolation.
/// This controls how fast coefficients approach their target.
/// Value of 0.001 at 44.1kHz gives ~3ms time constant (smooth but responsive).
/// Lower values = slower/smoother, higher = faster/more responsive.
const SMOOTH_COEFF: f32 = 0.002;

/// Biquad filter implementation
/// Used internally by the EQ for each band
///
/// Implements coefficient smoothing to prevent audio artifacts (clicks, pops,
/// zipper noise) when parameters change at runtime. Uses exponential smoothing
/// for natural transitions that handle continuous parameter changes (like dragging).
#[derive(Debug, Clone)]
struct BiquadFilter {
    // Target filter coefficients (set by set_* methods)
    target_b0: f32,
    target_b1: f32,
    target_b2: f32,
    target_a1: f32,
    target_a2: f32,

    // Active coefficients (used for processing, smoothed toward target)
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
            // Target coefficients (where we're smoothing to)
            target_b0: 1.0,
            target_b1: 0.0,
            target_b2: 0.0,
            target_a1: 0.0,
            target_a2: 0.0,
            // Active coefficients (start at neutral/bypass)
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            // State variables
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

    /// Smoothly update coefficients toward target values using exponential smoothing
    /// Called once per sample during processing
    ///
    /// Uses exponential (one-pole) smoothing which naturally handles continuous
    /// parameter changes. Each sample moves coefficients closer to target by a
    /// fixed proportion, providing smooth transitions regardless of how often
    /// parameters change.
    #[inline]
    fn smooth_coefficients(&mut self) {
        // Exponential smoothing: new = old + alpha * (target - old)
        // This naturally handles continuous parameter changes without needing
        // to track a "smoothing window" that can restart
        self.b0 += SMOOTH_COEFF * (self.target_b0 - self.b0);
        self.b1 += SMOOTH_COEFF * (self.target_b1 - self.b1);
        self.b2 += SMOOTH_COEFF * (self.target_b2 - self.b2);
        self.a1 += SMOOTH_COEFF * (self.target_a1 - self.a1);
        self.a2 += SMOOTH_COEFF * (self.target_a2 - self.a2);
    }

    /// Set target coefficients for exponential smoothing
    ///
    /// The exponential smoothing in smooth_coefficients() will gradually
    /// move the active coefficients toward these targets. This naturally
    /// handles both single changes and continuous parameter updates.
    ///
    /// Note: Always uses smooth transitions to prevent audio artifacts when
    /// adding/removing bands during playback. New filters start at neutral
    /// and smoothly transition to target values.
    fn set_target_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
        // Update targets - exponential smoothing in smooth_coefficients() handles transitions
        // We always smooth, even from neutral state, to prevent clicks when adding bands
        self.target_b0 = b0;
        self.target_b1 = b1;
        self.target_b2 = b2;
        self.target_a1 = a1;
        self.target_a2 = a2;
    }

    /// Configure as peaking EQ filter
    fn set_peaking(&mut self, sample_rate: f32, frequency: f32, q: f32, gain_db: f32) {
        // Bug fix: Early return if sample rate is invalid to prevent division by zero
        if sample_rate < 1.0 {
            return;
        }

        let a = 10.0_f32.powf(gain_db / 40.0); // Amplitude
        // Bug fix: Clamp frequency to 45% of sample rate to prevent near-Nyquist instability
        let clamped_freq = frequency.min(sample_rate * 0.45);
        let omega = 2.0 * std::f32::consts::PI * clamped_freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha / a;

        // Normalize and set as target (smoothed transition)
        self.set_target_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    /// Configure as low shelf filter
    fn set_low_shelf(&mut self, sample_rate: f32, frequency: f32, q: f32, gain_db: f32) {
        // Bug fix: Early return if sample rate is invalid to prevent division by zero
        if sample_rate < 1.0 {
            return;
        }

        let a = 10.0_f32.powf(gain_db / 40.0);
        // Bug fix: Clamp frequency to 45% of sample rate to prevent near-Nyquist instability
        let clamped_freq = frequency.min(sample_rate * 0.45);
        let omega = 2.0 * std::f32::consts::PI * clamped_freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        // Bug fix: Use q parameter instead of hardcoded 0.707
        let alpha = sin_omega / 2.0 * ((a + 1.0 / a) * (1.0 / q - 1.0) + 2.0).sqrt();
        let beta = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + beta);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - beta);
        let a0 = (a + 1.0) + (a - 1.0) * cos_omega + beta;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) + (a - 1.0) * cos_omega - beta;

        // Normalize and set as target (smoothed transition)
        self.set_target_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    /// Configure as high shelf filter
    fn set_high_shelf(&mut self, sample_rate: f32, frequency: f32, q: f32, gain_db: f32) {
        // Bug fix: Early return if sample rate is invalid to prevent division by zero
        if sample_rate < 1.0 {
            return;
        }

        let a = 10.0_f32.powf(gain_db / 40.0);
        // Bug fix: Clamp frequency to 45% of sample rate to prevent near-Nyquist instability
        let clamped_freq = frequency.min(sample_rate * 0.45);
        let omega = 2.0 * std::f32::consts::PI * clamped_freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        // Bug fix: Use q parameter instead of hardcoded 0.707
        let alpha = sin_omega / 2.0 * ((a + 1.0 / a) * (1.0 / q - 1.0) + 2.0).sqrt();
        let beta = 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + beta);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - beta);
        let a0 = (a + 1.0) - (a - 1.0) * cos_omega + beta;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) - (a - 1.0) * cos_omega - beta;

        // Normalize and set as target (smoothed transition)
        self.set_target_coefficients(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    }

    /// Process a stereo sample pair (left, right)
    #[inline]
    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Smooth coefficient transitions to prevent clicks/pops
        self.smooth_coefficients();

        // Process left channel
        let mut out_l = self.b0 * left + self.b1 * self.x1_l + self.b2 * self.x2_l
            - self.a1 * self.y1_l
            - self.a2 * self.y2_l;

        // Bug fix: Flush denormal numbers to zero to prevent CPU performance issues
        if out_l.abs() < 1e-15 {
            out_l = 0.0;
        }

        self.x2_l = self.x1_l;
        self.x1_l = left;
        self.y2_l = self.y1_l;
        self.y1_l = out_l;

        // Process right channel
        let mut out_r = self.b0 * right + self.b1 * self.x1_r + self.b2 * self.x2_r
            - self.a1 * self.y1_r
            - self.a2 * self.y2_r;

        // Bug fix: Flush denormal numbers to zero to prevent CPU performance issues
        if out_r.abs() < 1e-15 {
            out_r = 0.0;
        }

        self.x2_r = self.x1_r;
        self.x1_r = right;
        self.y2_r = self.y1_r;
        self.y1_r = out_r;

        (out_l, out_r)
    }

    /// Reset filter state (but preserve coefficients)
    fn reset(&mut self) {
        self.x1_l = 0.0;
        self.x2_l = 0.0;
        self.y1_l = 0.0;
        self.y2_l = 0.0;
        self.x1_r = 0.0;
        self.x2_r = 0.0;
        self.y1_r = 0.0;
        self.y2_r = 0.0;
        // Snap to target coefficients when resetting
        self.b0 = self.target_b0;
        self.b1 = self.target_b1;
        self.b2 = self.target_b2;
        self.a1 = self.target_a1;
        self.a2 = self.target_a2;
    }

    /// Set filter to neutral (bypass) state without clearing filter state
    ///
    /// This is used when adding new bands during playback. The filter starts
    /// at neutral (pass-through) and smoothly transitions to target coefficients.
    /// Filter state is preserved so there's no transient from zeroed state.
    fn set_to_neutral(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
        self.target_b0 = 1.0;
        self.target_b1 = 0.0;
        self.target_b2 = 0.0;
        self.target_a1 = 0.0;
        self.target_a2 = 0.0;
    }
}

/// Dynamic Parametric Equalizer supporting 1-8 bands
///
/// This EQ supports a variable number of bands (1 to MAX_EQ_BANDS).
/// All filters are pre-allocated to avoid allocations during audio processing.
/// Bands are treated as peaking filters by default.
pub struct ParametricEq {
    /// Pre-allocated filters (all MAX_EQ_BANDS slots)
    /// Only the first `band_count` filters are active
    filters: [BiquadFilter; MAX_EQ_BANDS],

    /// Pre-allocated band configurations
    bands: [EqBand; MAX_EQ_BANDS],

    /// Number of active bands (1 to MAX_EQ_BANDS)
    band_count: usize,

    /// Whether the EQ is enabled
    enabled: bool,

    /// Current sample rate
    sample_rate: u32,

    /// Flag to recalculate filter coefficients
    needs_update: bool,
}

impl ParametricEq {
    /// Create a new parametric EQ with default 3-band settings
    ///
    /// Default bands:
    /// - Low: 80 Hz shelf
    /// - Mid: 1000 Hz peaking
    /// - High: 8000 Hz shelf
    pub fn new() -> Self {
        // Pre-allocate all filters with neutral coefficients
        let filters = [
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
            BiquadFilter::new(),
        ];

        // Default band configurations (3 bands for backward compatibility)
        let bands = [
            EqBand::low_shelf(80.0, 0.0),
            EqBand::peaking(1000.0, 0.0, 1.0),
            EqBand::high_shelf(8000.0, 0.0),
            EqBand::peaking(250.0, 0.0, 1.0),
            EqBand::peaking(500.0, 0.0, 1.0),
            EqBand::peaking(2000.0, 0.0, 1.0),
            EqBand::peaking(4000.0, 0.0, 1.0),
            EqBand::peaking(16000.0, 0.0, 1.0),
        ];

        Self {
            filters,
            bands,
            band_count: 3,
            enabled: true,
            sample_rate: 44100,
            needs_update: true,
        }
    }

    /// Create a new parametric EQ with the specified number of bands
    ///
    /// # Arguments
    /// * `num_bands` - Number of bands (clamped to 1..=MAX_EQ_BANDS)
    pub fn with_band_count(num_bands: usize) -> Self {
        let mut eq = Self::new();
        eq.band_count = num_bands.clamp(1, MAX_EQ_BANDS);
        eq
    }

    /// Get the number of active bands
    pub fn band_count(&self) -> usize {
        self.band_count
    }

    /// Set the number of active bands
    ///
    /// # Arguments
    /// * `count` - Number of bands (clamped to 1..=MAX_EQ_BANDS)
    ///
    /// When increasing the band count, the new bands will have their
    /// pre-existing configurations (default or previously set).
    /// New bands start with neutral coefficients and smoothly transition
    /// to their target values to prevent clicks.
    pub fn set_band_count(&mut self, count: usize) {
        let new_count = count.clamp(1, MAX_EQ_BANDS);

        // For newly added bands, ensure they start at neutral coefficients
        // Don't reset filter state - let coefficient smoothing handle the transition
        for i in self.band_count..new_count {
            self.filters[i].set_to_neutral();
        }

        self.band_count = new_count;
        self.needs_update = true;
    }

    /// Set low band parameters (band index 0)
    /// For backward compatibility with the 3-band API
    pub fn set_low_band(&mut self, band: EqBand) {
        self.set_band(0, band);
    }

    /// Set mid band parameters (band index 1)
    /// For backward compatibility with the 3-band API
    pub fn set_mid_band(&mut self, band: EqBand) {
        self.set_band(1, band);
    }

    /// Set high band parameters (band index 2)
    /// For backward compatibility with the 3-band API
    pub fn set_high_band(&mut self, band: EqBand) {
        self.set_band(2, band);
    }

    /// Set a specific band's parameters
    ///
    /// # Arguments
    /// * `index` - Band index (0 to band_count-1)
    /// * `band` - New band configuration
    ///
    /// If index >= band_count, the band_count is automatically increased.
    /// If index >= MAX_EQ_BANDS, the band is not set.
    ///
    /// Note: Filter state is preserved during parameter changes to prevent
    /// audio artifacts. Coefficients are smoothly interpolated to the new values.
    pub fn set_band(&mut self, index: usize, band: EqBand) {
        if index >= MAX_EQ_BANDS {
            return;
        }

        self.bands[index] = band;

        // For newly added bands, set to neutral and let smoothing transition to target
        // For existing bands, preserve state - coefficient smoothing handles the transition
        if index >= self.band_count {
            self.filters[index].set_to_neutral();
            self.band_count = index + 1;
        }

        self.needs_update = true;
    }

    /// Get a band's parameters
    ///
    /// # Arguments
    /// * `index` - Band index
    ///
    /// Returns None if index is out of range
    pub fn get_band(&self, index: usize) -> Option<EqBand> {
        if index < self.band_count {
            Some(self.bands[index])
        } else {
            None
        }
    }

    /// Set all bands at once
    ///
    /// # Arguments
    /// * `bands` - Vector of EQ bands (1 to MAX_EQ_BANDS)
    ///
    /// The band count is automatically adjusted to match the input length.
    /// If the vector is empty, a single flat band is used.
    /// If the vector exceeds MAX_EQ_BANDS, excess bands are ignored.
    ///
    /// Note: Filter states are preserved for existing bands to prevent audio
    /// artifacts. New bands start neutral and smoothly transition to targets.
    pub fn set_bands(&mut self, bands: Vec<EqBand>) {
        if bands.is_empty() {
            // Set a single flat band - preserve filter state
            self.bands[0] = EqBand::peaking(1000.0, 0.0, 1.0);
            self.band_count = 1;
            // Don't reset - let coefficient smoothing handle it
        } else {
            let new_count = bands.len().min(MAX_EQ_BANDS);

            for (i, band) in bands.into_iter().take(MAX_EQ_BANDS).enumerate() {
                self.bands[i] = band;
                // For newly added bands, set to neutral and let smoothing transition
                if i >= self.band_count {
                    self.filters[i].set_to_neutral();
                }
            }

            self.band_count = new_count;
        }

        self.needs_update = true;
    }

    /// Get all active bands
    pub fn get_bands(&self) -> Vec<EqBand> {
        self.bands[..self.band_count].to_vec()
    }

    /// Get low band parameters (band index 0)
    /// For backward compatibility with the 3-band API
    pub fn low_band(&self) -> EqBand {
        self.bands[0]
    }

    /// Get mid band parameters (band index 1)
    /// For backward compatibility with the 3-band API
    pub fn mid_band(&self) -> EqBand {
        if self.band_count > 1 {
            self.bands[1]
        } else {
            EqBand::peaking(1000.0, 0.0, 1.0)
        }
    }

    /// Get high band parameters (band index 2)
    /// For backward compatibility with the 3-band API
    pub fn high_band(&self) -> EqBand {
        if self.band_count > 2 {
            self.bands[2]
        } else {
            EqBand::high_shelf(8000.0, 0.0)
        }
    }

    /// Add a new band to the EQ
    ///
    /// # Arguments
    /// * `band` - Band configuration to add
    ///
    /// # Returns
    /// The index of the added band, or None if at max capacity
    ///
    /// Note: New bands start with neutral coefficients and smoothly
    /// transition to their target values to prevent audio artifacts.
    pub fn add_band(&mut self, band: EqBand) -> Option<usize> {
        if self.band_count >= MAX_EQ_BANDS {
            return None;
        }

        let index = self.band_count;
        self.bands[index] = band;
        // Set to neutral - smoothing will transition to target coefficients
        self.filters[index].set_to_neutral();
        self.band_count += 1;
        self.needs_update = true;

        Some(index)
    }

    /// Remove a band from the EQ
    ///
    /// # Arguments
    /// * `index` - Index of the band to remove
    ///
    /// # Returns
    /// true if the band was removed, false if index was invalid or
    /// if removing would leave zero bands
    pub fn remove_band(&mut self, index: usize) -> bool {
        if index >= self.band_count || self.band_count <= 1 {
            return false;
        }

        // Shift bands and filters down
        for i in index..(self.band_count - 1) {
            self.bands[i] = self.bands[i + 1];
            // Copy filter state to maintain continuity where possible
            self.filters[i] = self.filters[i + 1].clone();
        }

        // Reset the last slot
        self.bands[self.band_count - 1] = EqBand::peaking(1000.0, 0.0, 1.0);
        self.filters[self.band_count - 1].reset();

        self.band_count -= 1;
        self.needs_update = true;

        true
    }

    /// Update filter coefficients if needed
    fn update_filters(&mut self) {
        if !self.needs_update {
            return;
        }

        let sr = self.sample_rate as f32;

        for i in 0..self.band_count {
            let band = &self.bands[i];
            // Use the appropriate filter type based on band configuration
            match band.filter_type {
                FilterType::LowShelf => {
                    self.filters[i].set_low_shelf(sr, band.frequency, band.q, band.gain_db);
                }
                FilterType::HighShelf => {
                    self.filters[i].set_high_shelf(sr, band.frequency, band.q, band.gain_db);
                }
                FilterType::Peaking => {
                    self.filters[i].set_peaking(sr, band.frequency, band.q, band.gain_db);
                }
            }
        }

        self.needs_update = false;
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
            // Reset all filter states when sample rate changes
            for filter in &mut self.filters {
                filter.reset();
            }
            self.needs_update = true;
        }

        // Update filters if parameters changed
        self.update_filters();

        // Process interleaved stereo buffer
        // Process through all active bands in series
        for chunk in buffer.chunks_exact_mut(2) {
            let mut left = chunk[0];
            let mut right = chunk[1];

            // Process through all active filters
            for i in 0..self.band_count {
                let (l, r) = self.filters[i].process_sample(left, right);
                left = l;
                right = r;
            }

            chunk[0] = left;
            chunk[1] = right;
        }
    }

    fn reset(&mut self) {
        // Force update_filters to run even if it already ran before
        // This ensures coefficients are set to target and smoothing is re-triggered
        self.needs_update = true;
        self.update_filters();

        for filter in &mut self.filters {
            filter.reset();
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Parametric EQ"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_eq() {
        let eq = ParametricEq::new();
        assert!(eq.is_enabled());
        assert_eq!(eq.name(), "Parametric EQ");
        assert_eq!(eq.band_count(), 3); // Default is 3 bands
    }

    #[test]
    fn eq_band_clamping() {
        let band = EqBand::new(1000.0, 30.0, 0.01); // Out of range
        assert!(band.gain_db() <= 24.0);
        assert!(band.q() >= 0.1);
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

        // Results should be very close (allowing for floating point precision)
        // Minor differences can occur due to coefficient smoothing timing
        for (a, b) in buffer.iter().zip(buffer2.iter()) {
            assert!(
                (a - b).abs() < 1e-5,
                "Samples should be nearly identical after reset: {} vs {}",
                a,
                b
            );
        }
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
        assert_eq!(low.gain_db(), 3.0);

        let mid = EqBand::peaking(1000.0, -2.0, 2.0);
        assert_eq!(mid.frequency, 1000.0);
        assert_eq!(mid.q(), 2.0);

        let high = EqBand::high_shelf(10000.0, 4.0);
        assert_eq!(high.frequency, 10000.0);
    }

    // ==================== Dynamic Band Tests ====================

    #[test]
    fn dynamic_band_count() {
        let mut eq = ParametricEq::new();
        assert_eq!(eq.band_count(), 3);

        // Increase band count
        eq.set_band_count(5);
        assert_eq!(eq.band_count(), 5);

        // Decrease band count
        eq.set_band_count(2);
        assert_eq!(eq.band_count(), 2);

        // Test clamping to max
        eq.set_band_count(100);
        assert_eq!(eq.band_count(), MAX_EQ_BANDS);

        // Test clamping to min
        eq.set_band_count(0);
        assert_eq!(eq.band_count(), 1);
    }

    #[test]
    fn add_bands_dynamically() {
        let mut eq = ParametricEq::new();
        assert_eq!(eq.band_count(), 3);

        // Add a new band
        let index = eq.add_band(EqBand::peaking(4000.0, 3.0, 1.5));
        assert_eq!(index, Some(3));
        assert_eq!(eq.band_count(), 4);

        // Verify the band was added correctly
        let band = eq.get_band(3).unwrap();
        assert_eq!(band.frequency, 4000.0);
        assert_eq!(band.gain_db(), 3.0);
        assert_eq!(band.q(), 1.5);
    }

    #[test]
    fn add_bands_to_max_capacity() {
        let mut eq = ParametricEq::with_band_count(1);
        assert_eq!(eq.band_count(), 1);

        // Add bands up to max
        for i in 1..MAX_EQ_BANDS {
            let index = eq.add_band(EqBand::peaking(1000.0 * (i as f32), 0.0, 1.0));
            assert_eq!(index, Some(i));
        }

        assert_eq!(eq.band_count(), MAX_EQ_BANDS);

        // Trying to add one more should fail
        let result = eq.add_band(EqBand::peaking(20000.0, 0.0, 1.0));
        assert_eq!(result, None);
        assert_eq!(eq.band_count(), MAX_EQ_BANDS);
    }

    #[test]
    fn remove_bands() {
        let mut eq = ParametricEq::new();
        assert_eq!(eq.band_count(), 3);

        // Add a band first
        eq.add_band(EqBand::peaking(4000.0, 5.0, 2.0));
        assert_eq!(eq.band_count(), 4);

        // Remove band at index 1
        let removed = eq.remove_band(1);
        assert!(removed);
        assert_eq!(eq.band_count(), 3);

        // Verify the bands shifted correctly
        // Band 0 should still be at 80 Hz (original low band)
        assert_eq!(eq.get_band(0).unwrap().frequency, 80.0);
        // Band 1 should now be the old band 2 (8000 Hz high shelf)
        assert_eq!(eq.get_band(1).unwrap().frequency, 8000.0);
        // Band 2 should be the added 4000 Hz band
        assert_eq!(eq.get_band(2).unwrap().frequency, 4000.0);
    }

    #[test]
    fn cannot_remove_last_band() {
        let mut eq = ParametricEq::with_band_count(1);

        // Try to remove the only band
        let removed = eq.remove_band(0);
        assert!(!removed);
        assert_eq!(eq.band_count(), 1);
    }

    #[test]
    fn set_bands_vector() {
        let mut eq = ParametricEq::new();

        // Set 5 bands
        let bands = vec![
            EqBand::peaking(100.0, 1.0, 1.0),
            EqBand::peaking(500.0, 2.0, 1.0),
            EqBand::peaking(1000.0, 3.0, 1.0),
            EqBand::peaking(5000.0, 4.0, 1.0),
            EqBand::peaking(10000.0, 5.0, 1.0),
        ];

        eq.set_bands(bands);
        assert_eq!(eq.band_count(), 5);

        // Verify all bands
        assert_eq!(eq.get_band(0).unwrap().frequency, 100.0);
        assert_eq!(eq.get_band(1).unwrap().frequency, 500.0);
        assert_eq!(eq.get_band(2).unwrap().frequency, 1000.0);
        assert_eq!(eq.get_band(3).unwrap().frequency, 5000.0);
        assert_eq!(eq.get_band(4).unwrap().frequency, 10000.0);
    }

    #[test]
    fn set_bands_empty_vector() {
        let mut eq = ParametricEq::new();

        // Set empty vector - should result in 1 flat band
        eq.set_bands(vec![]);
        assert_eq!(eq.band_count(), 1);
        assert_eq!(eq.get_band(0).unwrap().gain_db(), 0.0);
    }

    #[test]
    fn set_bands_exceeds_max() {
        let mut eq = ParametricEq::new();

        // Create more bands than allowed
        let bands: Vec<EqBand> = (0..20)
            .map(|i| EqBand::peaking(100.0 * (i as f32 + 1.0), 0.0, 1.0))
            .collect();

        eq.set_bands(bands);

        // Should be clamped to MAX_EQ_BANDS
        assert_eq!(eq.band_count(), MAX_EQ_BANDS);
    }

    #[test]
    fn process_with_dynamic_bands() {
        let mut eq = ParametricEq::new();

        // Start with 3 bands
        let mut buffer = crate::effects::tests::generate_sine(1000.0, 44100, 0.1);
        eq.process(&mut buffer, 44100);

        // Add more bands and process again - should not crash
        eq.add_band(EqBand::peaking(2000.0, 3.0, 1.0));
        eq.add_band(EqBand::peaking(4000.0, -2.0, 1.5));

        let mut buffer2 = crate::effects::tests::generate_sine(1000.0, 44100, 0.1);
        eq.process(&mut buffer2, 44100);

        // Both should process without panic
        assert_eq!(eq.band_count(), 5);
    }

    #[test]
    fn audio_continues_after_adding_band() {
        let mut eq = ParametricEq::new();

        // Process initial buffer
        let mut buffer1 = vec![0.5f32; 200];
        eq.process(&mut buffer1, 44100);

        // Add a band mid-stream
        eq.add_band(EqBand::peaking(2000.0, 6.0, 2.0));

        // Process another buffer - should not crash or produce invalid output
        let mut buffer2 = vec![0.5f32; 200];
        eq.process(&mut buffer2, 44100);

        // Verify output is valid (not NaN or infinite)
        for sample in &buffer2 {
            assert!(sample.is_finite(), "Sample should be finite after adding band");
        }
    }

    #[test]
    fn audio_continues_after_removing_band() {
        let mut eq = ParametricEq::new();

        // Add bands
        eq.add_band(EqBand::peaking(2000.0, 3.0, 1.0));
        eq.add_band(EqBand::peaking(4000.0, 3.0, 1.0));

        // Process buffer
        let mut buffer1 = vec![0.5f32; 200];
        eq.process(&mut buffer1, 44100);

        // Remove a band mid-stream
        eq.remove_band(2);

        // Process another buffer - should not crash or produce invalid output
        let mut buffer2 = vec![0.5f32; 200];
        eq.process(&mut buffer2, 44100);

        // Verify output is valid
        for sample in &buffer2 {
            assert!(sample.is_finite(), "Sample should be finite after removing band");
        }
    }

    #[test]
    fn get_bands_returns_active_only() {
        let mut eq = ParametricEq::new();
        eq.set_band_count(2);

        let bands = eq.get_bands();
        assert_eq!(bands.len(), 2);
    }

    #[test]
    fn with_band_count_constructor() {
        let eq = ParametricEq::with_band_count(5);
        assert_eq!(eq.band_count(), 5);

        // Test clamping
        let eq_max = ParametricEq::with_band_count(100);
        assert_eq!(eq_max.band_count(), MAX_EQ_BANDS);

        let eq_min = ParametricEq::with_band_count(0);
        assert_eq!(eq_min.band_count(), 1);
    }

    #[test]
    fn set_band_auto_expands() {
        let mut eq = ParametricEq::with_band_count(2);
        assert_eq!(eq.band_count(), 2);

        // Setting band at index 4 should expand band_count to 5
        eq.set_band(4, EqBand::peaking(5000.0, 3.0, 1.0));
        assert_eq!(eq.band_count(), 5);

        // Verify the band was set
        assert_eq!(eq.get_band(4).unwrap().frequency, 5000.0);
    }

    #[test]
    fn set_band_beyond_max_ignored() {
        let mut eq = ParametricEq::new();

        // Try to set a band beyond MAX_EQ_BANDS
        eq.set_band(MAX_EQ_BANDS + 1, EqBand::peaking(20000.0, 10.0, 1.0));

        // Band count should not exceed MAX_EQ_BANDS
        assert!(eq.band_count() <= MAX_EQ_BANDS);
    }
}
