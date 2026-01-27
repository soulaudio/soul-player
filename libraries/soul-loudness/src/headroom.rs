//! Headroom management for preventing clipping
//!
//! Provides automatic and manual headroom attenuation to prevent
//! clipping before it happens. Applied before the DSP chain.
//!
//! # Signal Flow
//!
//! ```text
//! Source → ReplayGain → [Headroom Attenuation] → DSP Chain → Volume → Limiter → Output
//! ```
//!
//! # Modes
//!
//! - **Auto**: Calculate from ReplayGain + preamp + EQ boost estimates
//! - **Manual**: Fixed headroom reserve (e.g., -6 dB)
//! - **Disabled**: No headroom attenuation
//!
//! # Example
//!
//! ```
//! use soul_loudness::headroom::{HeadroomManager, HeadroomMode};
//!
//! let mut headroom = HeadroomManager::new();
//!
//! // Auto mode - calculates attenuation from cumulative gains
//! headroom.set_mode(HeadroomMode::Auto);
//! headroom.set_replaygain_db(5.0);     // +5 dB RG
//! headroom.set_preamp_db(3.0);          // +3 dB preamp
//! headroom.set_eq_max_boost_db(6.0);    // +6 dB EQ boost
//!
//! // Total potential gain: 14 dB, so headroom applies -14 dB
//! let attenuation = headroom.attenuation_db();
//! assert!((attenuation - (-14.0)).abs() < 0.01);
//! ```

/// Headroom mode for clipping prevention
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HeadroomMode {
    /// Automatic headroom calculation from DSP chain gains
    #[default]
    Auto,
    /// Fixed manual headroom reserve in dB (typically negative)
    Manual(f64),
    /// No headroom attenuation
    Disabled,
}

impl HeadroomMode {
    /// Parse from string for settings persistence
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" | "automatic" => Some(Self::Auto),
            "disabled" | "off" | "none" => Some(Self::Disabled),
            s if s.starts_with("manual:") || s.starts_with('-') => {
                // Parse "manual:-6" or just "-6"
                let value_str = s.trim_start_matches("manual:").trim();
                value_str.parse::<f64>().ok().map(Self::Manual)
            }
            _ => None,
        }
    }

    /// Convert to string for settings persistence
    pub fn as_str(&self) -> String {
        match self {
            Self::Auto => "auto".to_string(),
            Self::Manual(db) => format!("manual:{}", db),
            Self::Disabled => "disabled".to_string(),
        }
    }
}

/// Headroom manager for preventing clipping
///
/// Calculates and applies headroom attenuation based on cumulative
/// gain in the signal chain. This prevents clipping before it happens
/// rather than relying solely on the limiter to catch peaks.
///
/// # Coordination with LoudnessNormalizer
///
/// When the LoudnessNormalizer's `prevent_clipping` is enabled, it already
/// handles ReplayGain clipping prevention via peak-aware gain limiting.
/// In this case, set `exclude_replaygain_from_headroom(true)` to avoid
/// double-attenuation. The HeadroomManager will then only account for
/// DSP chain gains (EQ, effects) that the normalizer cannot anticipate.
#[derive(Debug)]
pub struct HeadroomManager {
    mode: HeadroomMode,
    /// ReplayGain value in dB
    replaygain_db: f64,
    /// Pre-amp gain in dB
    preamp_db: f64,
    /// Maximum EQ boost in dB (estimated from EQ settings)
    eq_max_boost_db: f64,
    /// Additional DSP gain in dB (from other effects)
    additional_gain_db: f64,
    /// Cached linear attenuation factor
    attenuation_linear: f32,
    /// Whether attenuation needs recalculation
    dirty: bool,
    /// Whether the headroom manager is enabled
    enabled: bool,
    /// Whether to exclude ReplayGain from headroom calculation
    /// (set true when LoudnessNormalizer's prevent_clipping is enabled)
    exclude_replaygain: bool,
}

impl HeadroomManager {
    /// Create a new headroom manager with default settings
    pub fn new() -> Self {
        Self {
            mode: HeadroomMode::Auto,
            replaygain_db: 0.0,
            preamp_db: 0.0,
            eq_max_boost_db: 0.0,
            additional_gain_db: 0.0,
            attenuation_linear: 1.0,
            dirty: true,
            enabled: true,
            exclude_replaygain: false,
        }
    }

    /// Set whether to exclude ReplayGain from headroom calculation
    ///
    /// When the LoudnessNormalizer's `prevent_clipping` is enabled, it already
    /// handles ReplayGain-related clipping prevention via peak-aware gain limiting.
    /// Setting this to `true` prevents double-attenuation by excluding RG and
    /// preamp from the headroom calculation - only DSP chain gains (EQ, effects)
    /// will be considered.
    ///
    /// # Arguments
    /// * `exclude` - If true, exclude ReplayGain and preamp from headroom calculation
    pub fn set_exclude_replaygain(&mut self, exclude: bool) {
        if self.exclude_replaygain != exclude {
            self.exclude_replaygain = exclude;
            self.dirty = true;
        }
    }

    /// Check if ReplayGain is excluded from headroom calculation
    pub fn excludes_replaygain(&self) -> bool {
        self.exclude_replaygain
    }

    /// Enable or disable the headroom manager
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if headroom manager is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set headroom mode
    pub fn set_mode(&mut self, mode: HeadroomMode) {
        if self.mode != mode {
            self.mode = mode;
            self.dirty = true;
        }
    }

    /// Get current headroom mode
    pub fn mode(&self) -> HeadroomMode {
        self.mode
    }

    /// Set ReplayGain value in dB
    pub fn set_replaygain_db(&mut self, gain_db: f64) {
        if (self.replaygain_db - gain_db).abs() > 0.001 {
            self.replaygain_db = gain_db;
            self.dirty = true;
        }
    }

    /// Set pre-amp gain in dB
    pub fn set_preamp_db(&mut self, preamp_db: f64) {
        if (self.preamp_db - preamp_db).abs() > 0.001 {
            self.preamp_db = preamp_db;
            self.dirty = true;
        }
    }

    /// Set maximum EQ boost in dB
    ///
    /// This should be the maximum positive gain from any EQ band.
    /// For example, if your EQ has bands at +3, -2, +6 dB,
    /// set this to 6.0.
    pub fn set_eq_max_boost_db(&mut self, boost_db: f64) {
        let clamped = boost_db.max(0.0); // Only positive boosts matter
        if (self.eq_max_boost_db - clamped).abs() > 0.001 {
            self.eq_max_boost_db = clamped;
            self.dirty = true;
        }
    }

    /// Set additional gain from other DSP effects
    pub fn set_additional_gain_db(&mut self, gain_db: f64) {
        if (self.additional_gain_db - gain_db).abs() > 0.001 {
            self.additional_gain_db = gain_db;
            self.dirty = true;
        }
    }

    /// Calculate total potential gain in dB
    ///
    /// When `exclude_replaygain` is true, ReplayGain and preamp are excluded
    /// from this calculation (they're handled by the LoudnessNormalizer's
    /// peak-aware clipping prevention).
    pub fn total_potential_gain_db(&self) -> f64 {
        if self.exclude_replaygain {
            // Only DSP chain gains (EQ, effects)
            self.eq_max_boost_db + self.additional_gain_db
        } else {
            // Full chain including RG and preamp
            self.replaygain_db + self.preamp_db + self.eq_max_boost_db + self.additional_gain_db
        }
    }

    /// Get the headroom attenuation in dB
    ///
    /// Returns negative dB value (attenuation) or 0 if no attenuation needed.
    pub fn attenuation_db(&mut self) -> f64 {
        self.update_attenuation();
        20.0 * (self.attenuation_linear as f64).log10()
    }

    /// Get the headroom attenuation as linear factor
    pub fn attenuation_linear(&mut self) -> f32 {
        self.update_attenuation();
        self.attenuation_linear
    }

    /// Update cached attenuation if dirty
    fn update_attenuation(&mut self) {
        if !self.dirty {
            return;
        }

        let attenuation_db = match self.mode {
            HeadroomMode::Disabled => 0.0,
            HeadroomMode::Manual(db) => db.min(0.0), // Manual is always specified value (negative)
            HeadroomMode::Auto => {
                let total_gain = self.total_potential_gain_db();
                if total_gain > 0.0 {
                    -total_gain // Attenuate by total positive gain
                } else {
                    0.0 // No attenuation needed
                }
            }
        };

        self.attenuation_linear = 10.0_f32.powf(attenuation_db as f32 / 20.0);
        self.dirty = false;
    }

    /// Apply headroom attenuation to audio buffer
    ///
    /// Call this BEFORE the DSP chain to prevent clipping.
    pub fn process(&mut self, samples: &mut [f32]) {
        // Skip if disabled
        if !self.enabled {
            return;
        }

        self.update_attenuation();

        // Skip if no attenuation needed
        if (self.attenuation_linear - 1.0).abs() < 0.0001 {
            return;
        }

        for sample in samples.iter_mut() {
            *sample *= self.attenuation_linear;
        }
    }

    /// Process with sample rate (for PipelineComponent compatibility)
    pub fn process_with_sample_rate(&mut self, samples: &mut [f32], _sample_rate: u32) {
        self.process(samples);
    }

    /// Reset all gain values (e.g., for new track)
    pub fn reset(&mut self) {
        self.replaygain_db = 0.0;
        self.preamp_db = 0.0;
        self.eq_max_boost_db = 0.0;
        self.additional_gain_db = 0.0;
        self.dirty = true;
    }

    /// Clear track-specific values (ReplayGain) but keep settings
    pub fn clear_track_gains(&mut self) {
        self.replaygain_db = 0.0;
        self.dirty = true;
    }
}

impl Default for HeadroomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate optimal headroom from EQ settings
///
/// Estimates the maximum potential boost from an EQ configuration.
/// This is a utility function for setting up the headroom manager.
///
/// # Arguments
/// * `band_gains_db` - Slice of EQ band gains in dB
///
/// # Returns
/// Maximum positive gain from any band
pub fn calculate_eq_headroom(band_gains_db: &[f64]) -> f64 {
    band_gains_db
        .iter()
        .copied()
        .filter(|&g| g > 0.0)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}

/// Calculate cumulative headroom for a DSP chain
///
/// # Arguments
/// * `replaygain_db` - ReplayGain value
/// * `preamp_db` - Pre-amp gain
/// * `eq_gains_db` - EQ band gains
/// * `other_gains_db` - Slice of other effect gains
///
/// # Returns
/// Recommended headroom attenuation in dB (negative value)
pub fn calculate_auto_headroom(
    replaygain_db: f64,
    preamp_db: f64,
    eq_gains_db: &[f64],
    other_gains_db: &[f64],
) -> f64 {
    let eq_boost = calculate_eq_headroom(eq_gains_db);
    let other_boost: f64 = other_gains_db.iter().copied().filter(|&g| g > 0.0).sum();

    let total = replaygain_db + preamp_db + eq_boost + other_boost;
    if total > 0.0 {
        -total
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headroom_manager_default() {
        let manager = HeadroomManager::new();
        assert_eq!(manager.mode(), HeadroomMode::Auto);
    }

    #[test]
    fn test_headroom_disabled() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Disabled);
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(5.0);

        // Should not attenuate
        let attenuation = manager.attenuation_linear();
        assert!((attenuation - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_manual() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        // Should attenuate by -6 dB regardless of gains
        let attenuation = manager.attenuation_db();
        assert!((attenuation - (-6.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_auto_positive_gain() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(5.0);
        manager.set_preamp_db(3.0);
        manager.set_eq_max_boost_db(6.0);

        // Total: 14 dB, should attenuate by -14 dB
        let attenuation = manager.attenuation_db();
        assert!((attenuation - (-14.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_auto_negative_gain() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(-5.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(0.0);

        // Total: -5 dB, no attenuation needed
        let attenuation = manager.attenuation_db();
        assert!((attenuation - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_headroom_process() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        let mut samples = vec![1.0_f32; 100];
        manager.process(&mut samples);

        // -6 dB = ~0.501 linear
        let expected = 10.0_f32.powf(-6.0 / 20.0);
        for &sample in &samples {
            assert!((sample - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_calculate_eq_headroom() {
        let gains = [3.0, -2.0, 6.0, 0.0, -1.0, 4.0];
        let headroom = calculate_eq_headroom(&gains);
        assert!((headroom - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_eq_headroom_all_negative() {
        let gains = [-3.0, -2.0, -6.0];
        let headroom = calculate_eq_headroom(&gains);
        assert!((headroom - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_auto_headroom() {
        let eq_gains = [3.0, 6.0, -2.0];
        let other_gains = [2.0, -1.0];
        let headroom = calculate_auto_headroom(5.0, 2.0, &eq_gains, &other_gains);

        // RG=5, preamp=2, EQ=6, other=2 -> total=15 -> headroom=-15
        assert!((headroom - (-15.0)).abs() < 0.001);
    }

    #[test]
    fn test_mode_parsing() {
        assert_eq!(HeadroomMode::from_str("auto"), Some(HeadroomMode::Auto));
        assert_eq!(
            HeadroomMode::from_str("disabled"),
            Some(HeadroomMode::Disabled)
        );
        assert_eq!(
            HeadroomMode::from_str("manual:-6"),
            Some(HeadroomMode::Manual(-6.0))
        );
        assert_eq!(
            HeadroomMode::from_str("-3.5"),
            Some(HeadroomMode::Manual(-3.5))
        );
    }

    #[test]
    fn test_mode_string_roundtrip() {
        let modes = [
            HeadroomMode::Auto,
            HeadroomMode::Disabled,
            HeadroomMode::Manual(-6.0),
        ];

        for mode in modes {
            let s = mode.as_str();
            let parsed = HeadroomMode::from_str(&s);
            assert_eq!(parsed, Some(mode));
        }
    }

    #[test]
    fn test_reset() {
        let mut manager = HeadroomManager::new();
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(5.0);
        manager.set_eq_max_boost_db(6.0);

        manager.reset();

        assert!((manager.total_potential_gain_db() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_clear_track_gains() {
        let mut manager = HeadroomManager::new();
        manager.set_replaygain_db(10.0);
        manager.set_eq_max_boost_db(6.0);

        manager.clear_track_gains();

        // ReplayGain cleared, but EQ boost preserved
        assert!((manager.total_potential_gain_db() - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_exclude_replaygain_from_headroom() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(2.0);
        manager.set_eq_max_boost_db(6.0);

        // Default: include RG in calculation
        assert!(!manager.excludes_replaygain());
        // Total: 10 + 2 + 6 = 18 dB
        assert!((manager.total_potential_gain_db() - 18.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-18.0)).abs() < 0.1);

        // Exclude RG: only DSP chain gains (EQ)
        manager.set_exclude_replaygain(true);
        assert!(manager.excludes_replaygain());
        // Total: 6 dB (only EQ, RG and preamp excluded)
        assert!((manager.total_potential_gain_db() - 6.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-6.0)).abs() < 0.1);
    }

    #[test]
    fn test_exclude_replaygain_prevents_double_attenuation() {
        // Scenario: LoudnessNormalizer with prevent_clipping=true handles RG,
        // HeadroomManager should only handle DSP chain gains
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(8.0); // +8 dB RG (would cause double-attenuation)
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(4.0); // +4 dB EQ boost
        manager.set_exclude_replaygain(true); // Coordinate with normalizer

        // Headroom should only account for EQ, not RG
        assert!((manager.total_potential_gain_db() - 4.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-4.0)).abs() < 0.1);

        // Test signal: apply headroom attenuation
        let mut samples: Vec<f32> = vec![1.0, -1.0, 0.5, -0.5];
        manager.process(&mut samples);

        // Should attenuate by -4 dB (not -12 dB which would be double)
        let expected_linear = 10.0_f32.powf(-4.0 / 20.0); // ~0.631
        for &sample in &samples[0..2] {
            assert!(
                (sample.abs() - expected_linear).abs() < 0.01,
                "Expected ~{:.3} but got {:.3}",
                expected_linear,
                sample.abs()
            );
        }
    }

    #[test]
    fn test_exclude_replaygain_no_eq_no_attenuation() {
        // When RG excluded and no EQ boost, no attenuation needed
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(12.0); // Would need -12 dB headroom normally
        manager.set_eq_max_boost_db(0.0); // Flat EQ
        manager.set_exclude_replaygain(true);

        // Total potential gain is 0 (no DSP chain boost)
        assert!((manager.total_potential_gain_db() - 0.0).abs() < 0.001);
        // No attenuation needed
        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }

    // ==================== Headroom Manager Coordination Tests ====================

    #[test]
    fn test_headroom_normalizer_coordination_scenario() {
        // Scenario: LoudnessNormalizer handles ReplayGain clipping prevention,
        // HeadroomManager only handles DSP chain gains

        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        // Simulate a track with +10 dB ReplayGain (loud track, needs attenuation normally)
        // and +6 dB EQ boost
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(6.0);

        // Without coordination: total = 16 dB, headroom = -16 dB
        assert!((manager.total_potential_gain_db() - 16.0).abs() < 0.001);

        // Enable coordination (normalizer handles RG)
        manager.set_exclude_replaygain(true);

        // With coordination: only EQ = 6 dB, headroom = -6 dB
        assert!((manager.total_potential_gain_db() - 6.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-6.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_preamp_coordination() {
        // When exclude_replaygain is true, preamp should also be excluded
        // (preamp is part of the ReplayGain chain, not the DSP chain)
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(5.0);
        manager.set_preamp_db(5.0);
        manager.set_eq_max_boost_db(3.0);

        // Full chain: 5 + 5 + 3 = 13 dB
        assert!((manager.total_potential_gain_db() - 13.0).abs() < 0.001);

        // Exclude RG + preamp
        manager.set_exclude_replaygain(true);

        // Only EQ: 3 dB
        assert!((manager.total_potential_gain_db() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_with_additional_dsp_gains() {
        // Test: Additional DSP gains (compressor makeup, etc.) are always included
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(5.0);
        manager.set_eq_max_boost_db(3.0);
        manager.set_additional_gain_db(4.0); // Compressor makeup gain

        // Full: 5 + 3 + 4 = 12 dB
        assert!((manager.total_potential_gain_db() - 12.0).abs() < 0.001);

        manager.set_exclude_replaygain(true);

        // Excluding RG: 3 + 4 = 7 dB (additional gain IS included)
        assert!((manager.total_potential_gain_db() - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_toggle_coordination() {
        // Test: Toggling coordination should update attenuation correctly
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(8.0);
        manager.set_eq_max_boost_db(4.0);

        // Start without coordination
        assert!(!manager.excludes_replaygain());
        let attn_without = manager.attenuation_db();
        assert!((attn_without - (-12.0)).abs() < 0.1);

        // Enable coordination
        manager.set_exclude_replaygain(true);
        assert!(manager.excludes_replaygain());
        let attn_with = manager.attenuation_db();
        assert!((attn_with - (-4.0)).abs() < 0.1);

        // Disable coordination
        manager.set_exclude_replaygain(false);
        let attn_again = manager.attenuation_db();
        assert!((attn_again - (-12.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_process_with_coordination() {
        // Test: Process should apply correct attenuation based on coordination state
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(12.0);
        manager.set_eq_max_boost_db(6.0);

        // Full attenuation: -18 dB
        // expected_linear = 10^(-18/20) ≈ 0.126
        let mut samples_full: Vec<f32> = vec![1.0, 1.0];
        manager.process(&mut samples_full);

        let expected_full = 10.0_f32.powf(-18.0 / 20.0);
        assert!(
            (samples_full[0] - expected_full).abs() < 0.01,
            "Full attenuation expected: sample={}, expected_linear={}",
            samples_full[0],
            expected_full
        );

        // With coordination: only -6 dB
        // expected_linear = 10^(-6/20) ≈ 0.501
        manager.set_exclude_replaygain(true);
        let mut samples_coord: Vec<f32> = vec![1.0, 1.0];
        manager.process(&mut samples_coord);

        let expected_coord = 10.0_f32.powf(-6.0 / 20.0);
        assert!(
            (samples_coord[0] - expected_coord).abs() < 0.01,
            "Coordinated attenuation: {} vs expected {}",
            samples_coord[0],
            expected_coord
        );
    }

    // ==================== Edge Cases for Headroom Manager ====================

    #[test]
    fn test_headroom_negative_gains_no_attenuation() {
        // Test: Negative gains (cuts) should not trigger headroom attenuation
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(-5.0); // Track is too loud, reduce
        manager.set_preamp_db(-3.0); // User wants it quieter
        manager.set_eq_max_boost_db(0.0); // Flat EQ

        // Total is negative, no attenuation needed
        assert!(manager.total_potential_gain_db() < 0.0);
        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_mixed_positive_negative_gains() {
        // Test: Mixed gains - only total positive matters
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(-5.0); // -5 dB
        manager.set_preamp_db(3.0); // +3 dB
        manager.set_eq_max_boost_db(4.0); // +4 dB

        // Total: -5 + 3 + 4 = 2 dB
        assert!((manager.total_potential_gain_db() - 2.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-2.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_manual_mode_ignores_gains() {
        // Test: Manual mode uses fixed value regardless of gains
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-3.0));

        manager.set_replaygain_db(20.0);
        manager.set_eq_max_boost_db(10.0);
        manager.set_additional_gain_db(5.0);

        // Manual mode: always -3 dB regardless of total gains
        assert!((manager.attenuation_db() - (-3.0)).abs() < 0.1);
    }

    #[test]
    fn test_headroom_disabled_mode() {
        // Test: Disabled mode applies no attenuation
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Disabled);

        manager.set_replaygain_db(20.0);
        manager.set_eq_max_boost_db(10.0);

        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_enabled_flag() {
        // Test: enabled flag bypasses processing
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(10.0);
        manager.set_eq_max_boost_db(6.0);

        // Enabled: attenuation applied
        let mut samples1 = vec![1.0f32; 10];
        manager.process(&mut samples1);
        assert!(samples1[0] < 1.0, "Should attenuate when enabled");

        // Disabled: bypass
        manager.set_enabled(false);
        let mut samples2 = vec![1.0f32; 10];
        manager.process(&mut samples2);
        assert!(
            (samples2[0] - 1.0).abs() < 0.001,
            "Should bypass when disabled"
        );
    }

    #[test]
    fn test_headroom_dirty_flag_optimization() {
        // Test: Dirty flag should prevent recalculation when no changes
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(5.0);
        manager.set_eq_max_boost_db(3.0);

        // First call calculates
        let attn1 = manager.attenuation_linear();

        // Second call without changes should return same value
        let attn2 = manager.attenuation_linear();
        assert_eq!(attn1, attn2);

        // Change a value
        manager.set_eq_max_boost_db(6.0);

        // Should recalculate
        let attn3 = manager.attenuation_linear();
        assert!(attn3 != attn1, "Should recalculate after change");
    }

    #[test]
    fn test_headroom_very_large_gains() {
        // Test: Very large gains should produce very small attenuation factors
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(30.0);
        manager.set_eq_max_boost_db(20.0);
        manager.set_additional_gain_db(10.0);

        // Total: 60 dB - very large
        let attn = manager.attenuation_linear();
        assert!(
            attn < 0.01,
            "Very large gain should have very small linear attenuation: {}",
            attn
        );
        assert!(attn.is_finite(), "Attenuation should be finite");
    }

    #[test]
    fn test_headroom_eq_only_clamps_negative() {
        // Test: EQ max boost only considers positive values
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        // Try to set negative EQ boost (nonsensical but should be clamped to 0)
        manager.set_eq_max_boost_db(-6.0);

        // Should be clamped to 0
        assert!(
            manager.total_potential_gain_db() >= 0.0 || manager.total_potential_gain_db() <= 0.0
        );
        // The internal value should be clamped to 0, so no attenuation from EQ
        manager.set_replaygain_db(0.0);
        manager.set_preamp_db(0.0);
        manager.set_additional_gain_db(0.0);
        assert!((manager.total_potential_gain_db() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_eq_headroom_empty() {
        let headroom = calculate_eq_headroom(&[]);
        assert!((headroom - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_eq_headroom_single_value() {
        assert!((calculate_eq_headroom(&[6.0]) - 6.0).abs() < 0.001);
        assert!((calculate_eq_headroom(&[-3.0]) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_auto_headroom_all_cuts() {
        // All negative gains should result in no headroom needed
        let headroom = calculate_auto_headroom(-5.0, -3.0, &[-2.0, -4.0], &[-1.0]);
        assert!((headroom - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_process_empty_buffer() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(10.0);

        let mut empty: Vec<f32> = vec![];
        manager.process(&mut empty); // Should not panic
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_headroom_process_single_sample() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        let mut single = vec![1.0f32];
        manager.process(&mut single);

        let expected = 10.0_f32.powf(-6.0 / 20.0);
        assert!(
            (single[0] - expected).abs() < 0.01,
            "Single sample should be attenuated: {} vs {}",
            single[0],
            expected
        );
    }

    #[test]
    fn test_headroom_process_with_sample_rate() {
        // Test the sample rate variant (should behave identically)
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        let mut samples = vec![1.0f32; 10];
        manager.process_with_sample_rate(&mut samples, 44100);

        let expected = 10.0_f32.powf(-6.0 / 20.0);
        for &sample in &samples {
            assert!((sample - expected).abs() < 0.01);
        }
    }
}
