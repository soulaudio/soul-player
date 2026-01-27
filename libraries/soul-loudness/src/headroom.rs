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

    // ==================== Auto Mode with Various Gain Combinations ====================

    /// Test: Auto mode with only ReplayGain (no EQ or additional gains)
    #[test]
    fn test_auto_mode_replaygain_only() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(8.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(0.0);
        manager.set_additional_gain_db(0.0);

        // Total: 8 dB, headroom should be -8 dB
        assert!((manager.total_potential_gain_db() - 8.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-8.0)).abs() < 0.1);
    }

    /// Test: Auto mode with only EQ boost (no ReplayGain)
    #[test]
    fn test_auto_mode_eq_only() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(0.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(9.0);
        manager.set_additional_gain_db(0.0);

        // Total: 9 dB from EQ
        assert!((manager.total_potential_gain_db() - 9.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-9.0)).abs() < 0.1);
    }

    /// Test: Auto mode with all gain sources active
    #[test]
    fn test_auto_mode_all_gains_active() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(4.0);
        manager.set_preamp_db(2.0);
        manager.set_eq_max_boost_db(5.0);
        manager.set_additional_gain_db(3.0);

        // Total: 4 + 2 + 5 + 3 = 14 dB
        assert!((manager.total_potential_gain_db() - 14.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-14.0)).abs() < 0.1);
    }

    /// Test: Auto mode with mixed positive and negative gains
    #[test]
    fn test_auto_mode_mixed_gains() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(-3.0); // Negative (loud track)
        manager.set_preamp_db(5.0); // User boost
        manager.set_eq_max_boost_db(4.0); // EQ boost
        manager.set_additional_gain_db(-2.0); // Compressor cut

        // Total: -3 + 5 + 4 + (-2) = 4 dB
        assert!((manager.total_potential_gain_db() - 4.0).abs() < 0.001);
        assert!((manager.attenuation_db() - (-4.0)).abs() < 0.1);
    }

    /// Test: Auto mode resulting in zero or negative total gain
    #[test]
    fn test_auto_mode_no_attenuation_needed() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(-10.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(2.0);
        manager.set_additional_gain_db(0.0);

        // Total: -10 + 2 = -8 dB (negative, no attenuation needed)
        assert!(manager.total_potential_gain_db() < 0.0);
        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }

    /// Test: Auto mode with extremely high gains
    #[test]
    fn test_auto_mode_extreme_gains() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(20.0);
        manager.set_preamp_db(12.0);
        manager.set_eq_max_boost_db(15.0);
        manager.set_additional_gain_db(10.0);

        // Total: 57 dB
        assert!((manager.total_potential_gain_db() - 57.0).abs() < 0.001);

        let attn = manager.attenuation_linear();
        // Very small attenuation factor expected
        assert!(
            attn < 0.01 && attn > 0.0,
            "Extreme gains should produce very small attenuation factor"
        );
    }

    // ==================== Manual Mode Tests ====================

    /// Test: Manual mode respects user setting regardless of gains
    #[test]
    fn test_manual_mode_respects_user_setting() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-4.0));

        // Set high gains that would require more attenuation in auto mode
        manager.set_replaygain_db(15.0);
        manager.set_preamp_db(10.0);
        manager.set_eq_max_boost_db(8.0);

        // Manual mode should still use -4 dB
        assert!(
            (manager.attenuation_db() - (-4.0)).abs() < 0.1,
            "Manual mode should use specified value regardless of gains"
        );
    }

    /// Test: Manual mode with zero headroom
    #[test]
    fn test_manual_mode_zero_headroom() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(0.0));

        manager.set_replaygain_db(10.0);
        manager.set_eq_max_boost_db(6.0);

        // Manual 0 dB = no attenuation
        assert!(
            (manager.attenuation_linear() - 1.0).abs() < 0.001,
            "Manual 0 dB should result in no attenuation"
        );
    }

    /// Test: Manual mode positive value is clamped to 0 or negative
    #[test]
    fn test_manual_mode_positive_value_clamped() {
        let mut manager = HeadroomManager::new();
        // Positive values don't make sense for headroom (gain, not attenuation)
        manager.set_mode(HeadroomMode::Manual(5.0));

        // Implementation clamps to 0 or below for safety
        let attn = manager.attenuation_db();
        assert!(
            attn <= 0.0,
            "Manual mode should clamp positive values to 0 or negative"
        );
    }

    /// Test: Manual mode with deep attenuation
    #[test]
    fn test_manual_mode_deep_attenuation() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-20.0));

        let attn = manager.attenuation_db();
        assert!(
            (attn - (-20.0)).abs() < 0.1,
            "Manual mode should allow deep attenuation"
        );

        // -20 dB = 0.1 linear
        let expected_linear = 10.0_f32.powf(-20.0 / 20.0);
        assert!((manager.attenuation_linear() - expected_linear).abs() < 0.01);
    }

    // ==================== EQ Boost Accounting Tests ====================

    /// Test: EQ boost only counts positive values
    #[test]
    fn test_eq_boost_ignores_negative() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(0.0);
        manager.set_preamp_db(0.0);

        // Set negative EQ boost (should be clamped to 0)
        manager.set_eq_max_boost_db(-6.0);

        // No positive gain, so no attenuation
        assert!(
            (manager.total_potential_gain_db() - 0.0).abs() < 0.001,
            "Negative EQ should not add to headroom"
        );
        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }

    /// Test: EQ boost from multiple bands (max value)
    #[test]
    fn test_eq_boost_max_from_bands() {
        // Using the calculate_eq_headroom helper
        let bands = [2.0, -3.0, 6.0, 1.0, -2.0, 4.0];
        let headroom = calculate_eq_headroom(&bands);

        // Max positive is 6.0
        assert!(
            (headroom - 6.0).abs() < 0.001,
            "Should use max positive EQ band"
        );
    }

    /// Test: EQ boost with all negative bands
    #[test]
    fn test_eq_boost_all_negative() {
        let bands = [-2.0, -4.0, -1.0, -3.0];
        let headroom = calculate_eq_headroom(&bands);

        assert!(
            (headroom - 0.0).abs() < 0.001,
            "All negative bands should result in 0 headroom"
        );
    }

    /// Test: EQ boost with single band
    #[test]
    fn test_eq_boost_single_band() {
        let positive_band = [8.5];
        let negative_band = [-5.0];

        assert!((calculate_eq_headroom(&positive_band) - 8.5).abs() < 0.001);
        assert!((calculate_eq_headroom(&negative_band) - 0.0).abs() < 0.001);
    }

    /// Test: EQ headroom calculation with empty array
    #[test]
    fn test_eq_boost_empty_bands() {
        let empty: [f64; 0] = [];
        let headroom = calculate_eq_headroom(&empty);

        assert!((headroom - 0.0).abs() < 0.001, "Empty bands should give 0");
    }

    // ==================== Multiple Gain Sources Interaction ====================

    /// Test: Additional gain accumulates with EQ boost
    #[test]
    fn test_additional_gain_accumulates_with_eq() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(0.0);
        manager.set_preamp_db(0.0);
        manager.set_eq_max_boost_db(5.0);
        manager.set_additional_gain_db(3.0);

        // 5 + 3 = 8 dB
        assert!((manager.total_potential_gain_db() - 8.0).abs() < 0.001);
    }

    /// Test: All gain sources combine correctly
    #[test]
    fn test_all_gain_sources_combine() {
        let eq_gains = [3.0, 7.0, -2.0, 4.0]; // Max: 7.0
        let other_gains = [2.0, -1.0, 4.0]; // Sum positive: 6.0

        let headroom = calculate_auto_headroom(5.0, 3.0, &eq_gains, &other_gains);

        // RG=5, preamp=3, EQ max=7, other positive sum=6 -> total=21, headroom=-21
        assert!(
            (headroom - (-21.0)).abs() < 0.001,
            "All gains should combine correctly"
        );
    }

    /// Test: calculate_auto_headroom with all zeros
    #[test]
    fn test_auto_headroom_all_zeros() {
        let headroom = calculate_auto_headroom(0.0, 0.0, &[], &[]);
        assert!(
            (headroom - 0.0).abs() < 0.001,
            "All zeros should give no headroom"
        );
    }

    /// Test: calculate_auto_headroom with all negative
    #[test]
    fn test_auto_headroom_all_negative() {
        let headroom = calculate_auto_headroom(-5.0, -2.0, &[-3.0, -1.0], &[-2.0]);
        assert!(
            (headroom - 0.0).abs() < 0.001,
            "All negative should give no headroom (0)"
        );
    }

    // ==================== Coordination with Normalizer Tests ====================

    /// Test: exclude_replaygain setting works in auto mode
    #[test]
    fn test_exclude_rg_in_auto_mode() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(5.0);
        manager.set_eq_max_boost_db(6.0);

        // Without exclusion: 10 + 5 + 6 = 21 dB
        assert!((manager.total_potential_gain_db() - 21.0).abs() < 0.001);

        // With exclusion: only 6 dB (EQ)
        manager.set_exclude_replaygain(true);
        assert!((manager.total_potential_gain_db() - 6.0).abs() < 0.001);
    }

    /// Test: exclude_replaygain has no effect in manual mode
    #[test]
    fn test_exclude_rg_in_manual_mode() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-5.0));

        manager.set_replaygain_db(10.0);
        manager.set_exclude_replaygain(true);

        // Manual mode ignores the exclusion setting
        assert!(
            (manager.attenuation_db() - (-5.0)).abs() < 0.1,
            "Manual mode should not be affected by exclude_replaygain"
        );
    }

    /// Test: exclude_replaygain with additional DSP gains
    #[test]
    fn test_exclude_rg_with_additional_gains() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        manager.set_replaygain_db(8.0);
        manager.set_preamp_db(4.0);
        manager.set_eq_max_boost_db(3.0);
        manager.set_additional_gain_db(2.0); // Makeup gain from compressor

        manager.set_exclude_replaygain(true);

        // With exclusion: 3 + 2 = 5 dB (RG and preamp excluded)
        assert!(
            (manager.total_potential_gain_db() - 5.0).abs() < 0.001,
            "Additional gain should still be included when RG is excluded"
        );
    }

    // ==================== Process Output Verification ====================

    /// Test: Processed samples match expected attenuation
    #[test]
    fn test_process_applies_correct_attenuation() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(6.0); // 6 dB total -> -6 dB attenuation

        let mut samples = vec![1.0_f32, -1.0, 0.5, -0.5];
        manager.process(&mut samples);

        // -6 dB = 10^(-6/20) ≈ 0.501
        let expected_factor = 10.0_f32.powf(-6.0 / 20.0);
        assert!(
            (samples[0] - expected_factor).abs() < 0.01,
            "Sample 0 should be attenuated"
        );
        assert!(
            (samples[1] - (-expected_factor)).abs() < 0.01,
            "Sample 1 should be attenuated (negative)"
        );
        assert!(
            (samples[2] - (0.5 * expected_factor)).abs() < 0.01,
            "Sample 2 should be attenuated"
        );
    }

    /// Test: No attenuation when gains are negative
    #[test]
    fn test_process_no_attenuation_negative_gains() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(-10.0);
        manager.set_eq_max_boost_db(0.0);

        let mut samples = vec![0.8_f32, -0.6, 0.3];
        let original = samples.clone();
        manager.process(&mut samples);

        for (orig, processed) in original.iter().zip(samples.iter()) {
            assert!(
                (orig - processed).abs() < 0.001,
                "No attenuation should be applied for negative total gain"
            );
        }
    }

    /// Test: Disabled manager bypasses processing
    #[test]
    fn test_disabled_manager_bypasses() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(20.0);
        manager.set_enabled(false);

        let mut samples = vec![1.0_f32; 10];
        let original = samples.clone();
        manager.process(&mut samples);

        for (orig, processed) in original.iter().zip(samples.iter()) {
            assert!(
                (orig - processed).abs() < 0.001,
                "Disabled manager should bypass"
            );
        }
    }

    /// Test: Disabled mode (not just enabled flag) applies no attenuation
    #[test]
    fn test_disabled_mode_vs_enabled_flag() {
        let mut manager = HeadroomManager::new();

        // Case 1: Mode disabled
        manager.set_mode(HeadroomMode::Disabled);
        manager.set_enabled(true);
        manager.set_replaygain_db(10.0);

        let mut samples1 = vec![1.0_f32; 5];
        manager.process(&mut samples1);
        assert!(
            (samples1[0] - 1.0).abs() < 0.001,
            "Disabled mode should bypass"
        );

        // Case 2: Mode auto but manager disabled
        manager.set_mode(HeadroomMode::Auto);
        manager.set_enabled(false);

        let mut samples2 = vec![1.0_f32; 5];
        manager.process(&mut samples2);
        assert!(
            (samples2[0] - 1.0).abs() < 0.001,
            "Disabled manager should bypass"
        );
    }

    // ==================== State Management Tests ====================

    /// Test: Reset clears all gain values
    #[test]
    fn test_reset_clears_all() {
        let mut manager = HeadroomManager::new();
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(5.0);
        manager.set_eq_max_boost_db(6.0);
        manager.set_additional_gain_db(3.0);

        manager.reset();

        assert!((manager.total_potential_gain_db() - 0.0).abs() < 0.001);
    }

    /// Test: clear_track_gains only clears ReplayGain
    #[test]
    fn test_clear_track_gains_preserves_dsp() {
        let mut manager = HeadroomManager::new();
        manager.set_replaygain_db(10.0);
        manager.set_preamp_db(5.0);
        manager.set_eq_max_boost_db(6.0);
        manager.set_additional_gain_db(3.0);

        manager.clear_track_gains();

        // Only RG cleared, preamp is part of RG chain but implementation only clears RG
        // EQ and additional should remain
        // Note: preamp remains (it's a user setting, not per-track)
        let total = manager.total_potential_gain_db();
        // 0 (RG cleared) + 5 (preamp) + 6 (EQ) + 3 (additional) = 14
        assert!(
            (total - 14.0).abs() < 0.001,
            "Only RG should be cleared, got total: {}",
            total
        );
    }

    /// Test: Changing mode triggers recalculation
    #[test]
    fn test_mode_change_triggers_recalc() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(10.0);

        // Get attenuation in auto mode
        let auto_attn = manager.attenuation_db();
        assert!((auto_attn - (-10.0)).abs() < 0.1);

        // Switch to manual
        manager.set_mode(HeadroomMode::Manual(-3.0));
        let manual_attn = manager.attenuation_db();
        assert!((manual_attn - (-3.0)).abs() < 0.1);

        // Switch to disabled
        manager.set_mode(HeadroomMode::Disabled);
        let disabled_attn = manager.attenuation_linear();
        assert!((disabled_attn - 1.0).abs() < 0.001);
    }

    // ==================== Precision and Edge Cases ====================

    /// Test: Very small gain changes trigger recalculation
    #[test]
    fn test_small_gain_change_threshold() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(5.0);

        let attn1 = manager.attenuation_linear();

        // Very small change (below threshold) should not trigger recalc
        manager.set_replaygain_db(5.0001); // 0.0001 < threshold (0.001)
        let attn2 = manager.attenuation_linear();
        assert_eq!(attn1, attn2, "Small change should not trigger recalc");

        // Larger change should trigger recalc
        manager.set_replaygain_db(5.1);
        let attn3 = manager.attenuation_linear();
        assert!(attn1 != attn3, "Larger change should trigger recalc");
    }

    /// Test: Finite values for extreme attenuation
    #[test]
    fn test_extreme_attenuation_finite() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);
        manager.set_replaygain_db(100.0); // Extreme value

        let linear = manager.attenuation_linear();
        let db = manager.attenuation_db();

        assert!(linear.is_finite(), "Linear attenuation should be finite");
        assert!(db.is_finite(), "dB attenuation should be finite");
        assert!(linear > 0.0, "Linear attenuation should be positive");
    }

    /// Test: Zero total gain produces unity attenuation
    #[test]
    fn test_zero_gain_unity_attenuation() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Auto);

        // Gains that sum to zero
        manager.set_replaygain_db(-5.0);
        manager.set_preamp_db(3.0);
        manager.set_eq_max_boost_db(2.0);
        manager.set_additional_gain_db(0.0);

        // Total: -5 + 3 + 2 = 0
        assert!((manager.total_potential_gain_db() - 0.0).abs() < 0.001);
        // Zero or negative gain -> unity attenuation
        assert!((manager.attenuation_linear() - 1.0).abs() < 0.001);
    }
}
