//! Stereo Enhancement Effects
//!
//! Provides stereo field manipulation including:
//! - Width control (0-200%)
//! - Mid/Side processing
//! - Balance adjustment
//! - Mono compatibility checking

use super::chain::AudioEffect;
use std::f32::consts::PI;

/// Stereo enhancer settings
#[derive(Debug, Clone)]
pub struct StereoSettings {
    /// Stereo width (0.0 = mono, 1.0 = normal, 2.0 = extra wide)
    pub width: f32,

    /// Mid gain in dB (-12 to +12)
    pub mid_gain_db: f32,

    /// Side gain in dB (-12 to +12)
    pub side_gain_db: f32,

    /// Balance (-1.0 = full left, 0.0 = center, 1.0 = full right)
    pub balance: f32,
}

impl Default for StereoSettings {
    fn default() -> Self {
        Self {
            width: 1.0,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        }
    }
}

impl StereoSettings {
    /// Create settings with specific width
    pub fn with_width(width: f32) -> Self {
        Self {
            width: width.clamp(0.0, 2.0),
            ..Default::default()
        }
    }

    /// Create mono settings
    pub fn mono() -> Self {
        Self {
            width: 0.0,
            ..Default::default()
        }
    }

    /// Create wide stereo settings
    pub fn wide() -> Self {
        Self {
            width: 1.5,
            ..Default::default()
        }
    }

    /// Create extra wide settings
    pub fn extra_wide() -> Self {
        Self {
            width: 2.0,
            ..Default::default()
        }
    }
}

/// Stereo enhancer effect
///
/// Provides stereo field manipulation using Mid/Side processing:
/// - Mid = (L + R) / 2 (center/mono content)
/// - Side = (L - R) / 2 (stereo content)
///
/// Width control:
/// - 0% = mono (only mid)
/// - 100% = normal stereo
/// - 200% = double the stereo width (emphasized side)
///
/// # Parameter Smoothing
/// All gains are smoothed over 64 samples (~1.5ms) to prevent audible clicks
/// when parameters are adjusted during playback.

/// Smoothing coefficient for exponential parameter interpolation.
/// Value of 0.003 at 44.1kHz gives ~1.5ms time constant.
/// This ensures ~95% convergence within typical buffer sizes while remaining smooth.
const SMOOTH_COEFF: f32 = 0.003;

pub struct StereoEnhancer {
    /// Current settings
    settings: StereoSettings,

    /// Target mid gain (linear) - what we're smoothing toward
    target_mid_gain: f32,
    /// Target side gain (linear)
    target_side_gain: f32,
    /// Target width
    target_width: f32,
    /// Target balance
    target_balance: f32,

    /// Active mid gain (linear) - used for processing
    mid_gain: f32,
    /// Active side gain (linear)
    side_gain: f32,
    /// Active width
    width: f32,
    /// Active balance (smoothed)
    balance: f32,

    /// Effect enabled state
    enabled: bool,

    /// Settings need recalculation
    needs_update: bool,
}

impl StereoEnhancer {
    /// Create a new stereo enhancer with default settings
    pub fn new() -> Self {
        Self {
            settings: StereoSettings::default(),
            target_mid_gain: 1.0,
            target_side_gain: 1.0,
            target_width: 1.0,
            target_balance: 0.0,
            mid_gain: 1.0,
            side_gain: 1.0,
            width: 1.0,
            balance: 0.0,
            enabled: true,
            needs_update: true,
        }
    }

    /// Create with specific settings
    pub fn with_settings(settings: StereoSettings) -> Self {
        let mid_gain = 10.0_f32.powf(settings.mid_gain_db / 20.0);
        let side_gain = 10.0_f32.powf(settings.side_gain_db / 20.0);
        let balance = settings.balance;
        Self {
            target_mid_gain: mid_gain,
            target_side_gain: side_gain,
            target_width: settings.width,
            target_balance: balance,
            mid_gain,
            side_gain,
            width: settings.width,
            balance,
            settings,
            enabled: true,
            needs_update: false, // Already initialized
        }
    }

    /// Set stereo width (0.0 = mono, 1.0 = normal, 2.0 = extra wide)
    pub fn set_width(&mut self, width: f32) {
        self.settings.width = width.clamp(0.0, 2.0);
        self.target_width = self.settings.width;
        self.needs_update = true;
    }

    /// Smooth parameters toward target values using exponential smoothing
    ///
    /// Uses constant-rate exponential smoothing which naturally handles continuous
    /// parameter changes without needing to track a smoothing window.
    #[inline]
    fn smooth_parameters(&mut self) {
        // Exponential smoothing: new = old + alpha * (target - old)
        self.mid_gain += SMOOTH_COEFF * (self.target_mid_gain - self.mid_gain);
        self.side_gain += SMOOTH_COEFF * (self.target_side_gain - self.side_gain);
        self.width += SMOOTH_COEFF * (self.target_width - self.width);
        self.balance += SMOOTH_COEFF * (self.target_balance - self.balance);
    }

    /// Get current width setting
    pub fn width(&self) -> f32 {
        self.settings.width
    }

    /// Set mid gain in dB
    pub fn set_mid_gain_db(&mut self, gain_db: f32) {
        self.settings.mid_gain_db = gain_db.clamp(-12.0, 12.0);
        self.target_mid_gain = 10.0_f32.powf(self.settings.mid_gain_db / 20.0);
        self.needs_update = true;
    }

    /// Set side gain in dB
    pub fn set_side_gain_db(&mut self, gain_db: f32) {
        self.settings.side_gain_db = gain_db.clamp(-12.0, 12.0);
        self.target_side_gain = 10.0_f32.powf(self.settings.side_gain_db / 20.0);
        self.needs_update = true;
    }

    /// Set balance (-1.0 = full left, 0.0 = center, 1.0 = full right)
    pub fn set_balance(&mut self, balance: f32) {
        self.settings.balance = balance.clamp(-1.0, 1.0);
        self.target_balance = self.settings.balance;
        self.needs_update = true;
    }

    /// Get current balance
    pub fn balance(&self) -> f32 {
        self.settings.balance
    }

    /// Get current settings
    pub fn settings(&self) -> &StereoSettings {
        &self.settings
    }

    /// Apply settings directly
    pub fn apply_settings(&mut self, settings: StereoSettings) {
        // Extract values before moving settings
        self.target_mid_gain = 10.0_f32.powf(settings.mid_gain_db / 20.0);
        self.target_side_gain = 10.0_f32.powf(settings.side_gain_db / 20.0);
        self.target_width = settings.width;
        self.target_balance = settings.balance;
        self.settings = settings;
        self.needs_update = true;
    }

    /// Update derived parameters (sets targets, smoothing handles the transition)
    fn update_parameters(&mut self) {
        if self.needs_update {
            // Targets are already set by the setters
            // The actual smoothing happens in smooth_parameters()
            self.needs_update = false;
        }
    }

    /// Process a single stereo sample using Mid/Side technique
    #[inline]
    fn process_sample(&self, left: f32, right: f32) -> (f32, f32) {
        // Convert to Mid/Side
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;

        // Apply width and gains (using smoothed values)
        // Width < 1.0 reduces side (toward mono)
        // Width > 1.0 increases side (wider stereo)
        let processed_mid = mid * self.mid_gain;
        let processed_side = side * self.side_gain * self.width;

        // Convert back to L/R
        let mut new_left = processed_mid + processed_side;
        let mut new_right = processed_mid - processed_side;

        // Prevent clipping when width > 1.0 causes output to exceed [-1, 1]
        // This can happen with wide stereo content at high width settings
        let max_sample = new_left.abs().max(new_right.abs());
        if max_sample > 1.0 {
            let scale = 1.0 / max_sample;
            new_left *= scale;
            new_right *= scale;
        }

        // Apply balance using constant-power panning to preserve perceived loudness
        // Linear pan causes ~3dB drop at hard pan positions
        // Use smoothed balance value to prevent clicks
        let balance = self.balance;
        if balance.abs() > 0.001 {
            // Map balance [-1, 1] to angle [0, PI/2]
            // At balance=0: angle=PI/4 (equal power to both channels)
            // At balance=-1: angle=0 (full left)
            // At balance=1: angle=PI/2 (full right)
            let pan_angle = (balance + 1.0) * 0.5 * (PI * 0.5);
            let left_gain = pan_angle.cos();
            let right_gain = pan_angle.sin();
            new_left *= left_gain;
            new_right *= right_gain;
        }

        (new_left, new_right)
    }
}

impl Default for StereoEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for StereoEnhancer {
    fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
        if !self.enabled {
            return;
        }

        self.update_parameters();

        // Quick bypass if all active values are neutral (targets and current)
        // Check both targets and current values to ensure no smoothing in progress
        let targets_neutral = (self.target_width - 1.0).abs() < 0.001
            && (self.target_mid_gain - 1.0).abs() < 0.001
            && (self.target_side_gain - 1.0).abs() < 0.001
            && self.target_balance.abs() < 0.001;

        let current_neutral = (self.width - 1.0).abs() < 0.001
            && (self.mid_gain - 1.0).abs() < 0.001
            && (self.side_gain - 1.0).abs() < 0.001
            && self.balance.abs() < 0.001;

        if targets_neutral && current_neutral {
            return;
        }

        // Process interleaved stereo buffer
        for chunk in buffer.chunks_exact_mut(2) {
            // Smooth parameters to prevent clicks during parameter changes
            self.smooth_parameters();

            let (new_left, new_right) = self.process_sample(chunk[0], chunk[1]);
            chunk[0] = new_left;
            chunk[1] = new_right;
        }
    }

    fn reset(&mut self) {
        // Snap to target when resetting
        self.mid_gain = self.target_mid_gain;
        self.side_gain = self.target_side_gain;
        self.width = self.target_width;
        self.balance = self.target_balance;
        // No state to reset for stereo enhancer
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Stereo Enhancer"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Utility function to check mono compatibility
///
/// Returns the correlation coefficient between left and right channels:
/// - 1.0 = perfectly correlated (mono compatible)
/// - 0.0 = uncorrelated
/// - -1.0 = perfectly anti-correlated (will cancel in mono)
pub fn mono_compatibility(buffer: &[f32]) -> f32 {
    if buffer.len() < 2 {
        return 1.0;
    }

    let mut sum_l = 0.0f64;
    let mut sum_r = 0.0f64;
    let mut sum_ll = 0.0f64;
    let mut sum_rr = 0.0f64;
    let mut sum_lr = 0.0f64;
    let mut count = 0;

    for chunk in buffer.chunks_exact(2) {
        let l = chunk[0] as f64;
        let r = chunk[1] as f64;

        sum_l += l;
        sum_r += r;
        sum_ll += l * l;
        sum_rr += r * r;
        sum_lr += l * r;
        count += 1;
    }

    if count == 0 {
        return 1.0;
    }

    let n = count as f64;
    let mean_l = sum_l / n;
    let mean_r = sum_r / n;

    let var_l = (sum_ll / n) - (mean_l * mean_l);
    let var_r = (sum_rr / n) - (mean_r * mean_r);
    let cov = (sum_lr / n) - (mean_l * mean_r);

    let std_l = var_l.sqrt();
    let std_r = var_r.sqrt();

    if std_l < 1e-10 || std_r < 1e-10 {
        return 1.0; // Avoid division by zero
    }

    (cov / (std_l * std_r)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = StereoSettings::default();
        assert_eq!(settings.width, 1.0);
        assert_eq!(settings.mid_gain_db, 0.0);
        assert_eq!(settings.side_gain_db, 0.0);
        assert_eq!(settings.balance, 0.0);
    }

    #[test]
    fn test_presets() {
        let mono = StereoSettings::mono();
        assert_eq!(mono.width, 0.0);

        let wide = StereoSettings::wide();
        assert_eq!(wide.width, 1.5);

        let extra_wide = StereoSettings::extra_wide();
        assert_eq!(extra_wide.width, 2.0);
    }

    #[test]
    fn test_width_clamping() {
        let mut enhancer = StereoEnhancer::new();

        enhancer.set_width(-0.5);
        assert_eq!(enhancer.width(), 0.0);

        enhancer.set_width(3.0);
        assert_eq!(enhancer.width(), 2.0);
    }

    #[test]
    fn test_mono_conversion() {
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings::mono());

        // Stereo signal
        let mut buffer = vec![1.0, -1.0, 1.0, -1.0];

        enhancer.process(&mut buffer, 44100);

        // Should be mono (L == R)
        assert!(
            (buffer[0] - buffer[1]).abs() < 0.001,
            "Mono mode should produce identical channels"
        );
    }

    #[test]
    fn test_wide_stereo() {
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());

        // Stereo signal with some difference
        let mut buffer: Vec<f32> = vec![0.8, 0.2, 0.8, 0.2];
        let original_diff = (buffer[0] - buffer[1]).abs();

        enhancer.process(&mut buffer, 44100);

        // Stereo difference should be larger
        let new_diff = (buffer[0] - buffer[1]).abs();
        assert!(
            new_diff > original_diff,
            "Wide mode should increase stereo separation"
        );
    }

    #[test]
    fn test_balance_left() {
        // Use with_settings to initialize with target values (no smoothing needed)
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
            balance: -1.0,
            ..Default::default()
        });

        let mut buffer = vec![0.5, 0.5, 0.5, 0.5];
        enhancer.process(&mut buffer, 44100);

        // With constant-power panning at hard left (balance=-1.0):
        // pan_angle = 0, left_gain = cos(0) = 1.0, right_gain = sin(0) = 0.0
        // Right channel should be silent
        assert!(
            buffer[1].abs() < 0.001,
            "Full left balance should silence right channel, got {}",
            buffer[1]
        );
        // Left channel should have full signal
        assert!(
            buffer[0].abs() > 0.4,
            "Full left balance should preserve left channel, got {}",
            buffer[0]
        );
    }

    #[test]
    fn test_balance_right() {
        // Use with_settings to initialize with target values (no smoothing needed)
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
            balance: 1.0,
            ..Default::default()
        });

        let mut buffer = vec![0.5, 0.5, 0.5, 0.5];
        enhancer.process(&mut buffer, 44100);

        // With constant-power panning at hard right (balance=1.0):
        // pan_angle = PI/2, left_gain = cos(PI/2) = 0.0, right_gain = sin(PI/2) = 1.0
        // Left channel should be silent
        assert!(
            buffer[0].abs() < 0.001,
            "Full right balance should silence left channel, got {}",
            buffer[0]
        );
        // Right channel should have full signal
        assert!(
            buffer[1].abs() > 0.4,
            "Full right balance should preserve right channel, got {}",
            buffer[1]
        );
    }

    #[test]
    fn test_neutral_settings_bypass() {
        let mut enhancer = StereoEnhancer::new();

        let mut buffer = vec![0.5, 0.3, -0.2, 0.8];
        let original = buffer.clone();

        enhancer.process(&mut buffer, 44100);

        // Neutral settings should not change the signal
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_disabled_bypass() {
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());
        enhancer.set_enabled(false);

        let mut buffer = vec![0.5, 0.3];
        let original = buffer.clone();

        enhancer.process(&mut buffer, 44100);

        assert_eq!(buffer, original);
    }

    #[test]
    fn test_mono_compatibility_correlated() {
        // Perfectly correlated (mono)
        let buffer = vec![0.5, 0.5, -0.3, -0.3, 0.8, 0.8];
        let compat = mono_compatibility(&buffer);
        assert!(compat > 0.99, "Mono signal should have correlation ~1.0");
    }

    #[test]
    fn test_mono_compatibility_anticorrelated() {
        // Perfectly anti-correlated
        let buffer = vec![0.5, -0.5, -0.3, 0.3, 0.8, -0.8];
        let compat = mono_compatibility(&buffer);
        assert!(
            compat < -0.99,
            "Anti-correlated signal should have correlation ~-1.0"
        );
    }

    #[test]
    fn test_name() {
        let enhancer = StereoEnhancer::new();
        assert_eq!(enhancer.name(), "Stereo Enhancer");
    }
}
