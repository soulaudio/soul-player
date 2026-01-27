//! Volume control with logarithmic scaling and click-free ramping
//!
//! Provides human-perceptual volume control using dB scaling.
//! Volume range is 0-100%, mapped to -60 dB to 0 dB internally.
//!
//! # Click-Free Volume Changes
//!
//! Volume changes are smoothly ramped to prevent audible clicks/pops.
//! When `set_level()` or `set_muted()` is called while a ramp is in progress,
//! the new ramp starts from the current interpolated gain, ensuring smooth
//! transitions even with rapid changes.
//!
//! # Thread Safety
//!
//! This struct is NOT thread-safe and should only be accessed from the
//! audio processing thread.

/// Default ramp duration in milliseconds
///
/// 10ms is fast enough to feel instant but long enough to prevent clicks.
const RAMP_DURATION_MS: u32 = 10;

/// Minimum gain change threshold to trigger a ramp
///
/// Changes smaller than this are applied instantly to avoid accumulated error
/// from many tiny ramps. Value is in linear gain units.
const MIN_RAMP_THRESHOLD: f32 = 0.001;

/// Maximum allowed volume level
pub const MAX_VOLUME_LEVEL: u8 = 100;

/// Minimum allowed sample rate in Hz
pub const MIN_SAMPLE_RATE: u32 = 8000;

/// Maximum allowed sample rate in Hz
pub const MAX_SAMPLE_RATE: u32 = 384000;

/// Volume controller with logarithmic scaling and click-free ramping
///
/// Uses dB-based scaling to match human hearing perception.
/// Spotify-style: 0% = -60 dB (near silence), 100% = 0 dB (unity gain)
///
/// # Ramping Behavior
///
/// - New ramps always start from `current_gain`, not `target_gain`
/// - Overlapping ramps are handled smoothly by updating target mid-ramp
/// - Tiny gain changes (< 0.001) skip ramping to prevent accumulated error
#[derive(Debug, Clone)]
pub struct Volume {
    /// Volume level (0-100)
    level: u8,

    /// Mute state (preserves volume level)
    muted: bool,

    /// Target linear gain multiplier (what we're ramping towards)
    target_gain: f32,

    /// Current linear gain multiplier (actual gain being applied)
    current_gain: f32,

    /// Ramp state: samples remaining in current ramp
    ramp_samples_remaining: usize,

    /// Ramp state: total samples for current ramp (for interpolation)
    ramp_samples_total: usize,

    /// Ramp state: gain at start of current ramp
    ramp_start_gain: f32,

    /// Sample rate for ramp duration calculation (default: 44100)
    sample_rate: u32,

    /// Cached ramp duration in samples (stereo samples)
    ramp_duration_samples: usize,
}

impl Volume {
    /// Create new volume controller
    ///
    /// # Arguments
    /// * `level` - Initial volume (0-100, default: 80)
    pub fn new(level: u8) -> Self {
        Self::with_sample_rate(level, 44100)
    }

    /// Create new volume controller with specific sample rate
    ///
    /// # Arguments
    /// * `level` - Initial volume (0-100, clamped if out of range)
    /// * `sample_rate` - Audio sample rate in Hz (clamped to 8000-384000 Hz)
    pub fn with_sample_rate(level: u8, sample_rate: u32) -> Self {
        let level = level.min(MAX_VOLUME_LEVEL);
        let clamped_rate = sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
        let linear_gain = Self::calculate_linear_gain(level);
        let ramp_duration_samples = Self::calculate_ramp_samples(clamped_rate);

        Self {
            level,
            muted: false,
            target_gain: linear_gain,
            current_gain: linear_gain,
            ramp_samples_remaining: 0,
            ramp_samples_total: 0,
            ramp_start_gain: linear_gain,
            sample_rate: clamped_rate,
            ramp_duration_samples,
        }
    }

    /// Calculate ramp duration in stereo samples from sample rate
    #[inline]
    fn calculate_ramp_samples(sample_rate: u32) -> usize {
        // stereo samples = sample_rate * duration_ms / 1000 * 2
        ((sample_rate as u64 * RAMP_DURATION_MS as u64 * 2) / 1000) as usize
    }

    /// Update sample rate and recalculate ramp duration
    ///
    /// Should be called when audio output format changes.
    /// Sample rate is clamped to valid range (8000 - 384000 Hz).
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        let clamped_rate = sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
        if self.sample_rate != clamped_rate {
            self.sample_rate = clamped_rate;
            self.ramp_duration_samples = Self::calculate_ramp_samples(clamped_rate);
        }
    }

    /// Start a new ramp to the specified target gain
    ///
    /// If a ramp is already in progress, the new ramp starts from the current
    /// interpolated gain position, ensuring smooth transitions.
    ///
    /// Tiny gain changes (< MIN_RAMP_THRESHOLD) are applied instantly to
    /// prevent accumulated error from many small ramps.
    #[inline]
    fn start_ramp(&mut self, new_target: f32) {
        let gain_delta = (new_target - self.current_gain).abs();

        if gain_delta < MIN_RAMP_THRESHOLD {
            // Change is too small - apply instantly to avoid accumulated error
            self.current_gain = new_target;
            self.target_gain = new_target;
            self.ramp_samples_remaining = 0;
            return;
        }

        // Start new ramp from current position (handles overlapping ramps)
        self.ramp_start_gain = self.current_gain;
        self.target_gain = new_target;
        self.ramp_samples_total = self.ramp_duration_samples;
        self.ramp_samples_remaining = self.ramp_duration_samples;
    }

    /// Set volume level (0-100) with smooth ramping
    ///
    /// If a ramp is already in progress, the new ramp starts from the current
    /// gain position, ensuring smooth transitions even with rapid changes.
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(100);
        let new_target = if self.muted {
            0.0
        } else {
            Self::calculate_linear_gain(self.level)
        };
        self.start_ramp(new_target);
    }

    /// Get current volume level (0-100)
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Mute audio with smooth fade-out (preserves volume level)
    pub fn mute(&mut self) {
        if !self.muted {
            self.muted = true;
            self.start_ramp(0.0);
        }
    }

    /// Unmute audio with smooth fade-in (restores previous volume)
    pub fn unmute(&mut self) {
        if self.muted {
            self.muted = false;
            let new_target = Self::calculate_linear_gain(self.level);
            self.start_ramp(new_target);
        }
    }

    /// Toggle mute state with smooth ramping
    pub fn toggle_mute(&mut self) {
        if self.muted {
            self.unmute();
        } else {
            self.mute();
        }
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Get target linear gain multiplier
    ///
    /// Returns 0.0 if muted, otherwise logarithmic gain based on level.
    /// Note: This returns the target gain, not the current ramped gain.
    pub fn gain(&self) -> f32 {
        self.target_gain
    }

    /// Get current linear gain multiplier (accounting for ramp)
    ///
    /// This is the actual gain being applied during audio processing.
    pub fn current_gain(&self) -> f32 {
        self.current_gain
    }

    /// Check if a ramp is currently in progress
    pub fn is_ramping(&self) -> bool {
        self.ramp_samples_remaining > 0
    }

    /// Apply volume to audio buffer with smooth ramping (in-place)
    ///
    /// # Ramping Behavior
    ///
    /// - If no ramp is active and gain is 0.0: fills buffer with zeros
    /// - If no ramp is active and gain is 1.0: no processing
    /// - If no ramp is active: applies constant gain
    /// - If ramp is active: smoothly interpolates gain per sample
    ///
    /// # Audio Safety
    ///
    /// This method is allocation-free and suitable for real-time audio callbacks.
    pub fn apply(&mut self, buffer: &mut [f32]) {
        if self.ramp_samples_remaining == 0 {
            // No ramp in progress - apply constant gain
            if self.current_gain == 0.0 {
                buffer.fill(0.0);
            } else if self.current_gain != 1.0 {
                for sample in buffer.iter_mut() {
                    *sample *= self.current_gain;
                }
            }
            // If gain == 1.0, no processing needed
            return;
        }

        // Ramp in progress - interpolate gain per sample
        for sample in buffer.iter_mut() {
            if self.ramp_samples_remaining > 0 {
                // Linear interpolation from start to target
                let progress =
                    1.0 - (self.ramp_samples_remaining as f32 / self.ramp_samples_total as f32);
                self.current_gain =
                    self.ramp_start_gain + (self.target_gain - self.ramp_start_gain) * progress;
                self.ramp_samples_remaining -= 1;
            } else {
                // Ramp complete - snap to target
                self.current_gain = self.target_gain;
            }

            *sample *= self.current_gain;
        }

        // Ensure we snap to target when ramp completes
        if self.ramp_samples_remaining == 0 {
            self.current_gain = self.target_gain;
        }
    }

    /// Convert volume percentage to linear gain
    ///
    /// Formula: gain = 10^((level% - 100) * 0.6 / 20)
    /// - 0%   → -60 dB → 0.001 gain (near silence)
    /// - 50%  → -30 dB → 0.0316 gain
    /// - 80%  → -12 dB → 0.251 gain (default)
    /// - 100% →   0 dB → 1.0 gain (unity)
    fn calculate_linear_gain(level: u8) -> f32 {
        if level == 0 {
            return 0.0;
        }

        // Map 0-100% to -60 dB to 0 dB
        let db = (level as f32 - 100.0) * 0.6; // 0.6 = 60/100

        // Convert dB to linear gain: gain = 10^(dB/20)
        10.0_f32.powf(db / 20.0)
    }

    /// Convert linear gain to dB
    ///
    /// Useful for debugging and display
    #[allow(dead_code)]
    pub fn to_db(&self) -> f32 {
        if self.level == 0 || self.muted {
            -60.0
        } else {
            20.0 * self.target_gain.log10()
        }
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::new(80) // Default to 80%
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_volume() {
        let vol = Volume::new(80);
        assert_eq!(vol.level(), 80);
        assert!(!vol.is_muted());
    }

    #[test]
    fn set_volume_level() {
        let mut vol = Volume::new(50);
        assert_eq!(vol.level(), 50);

        vol.set_level(75);
        assert_eq!(vol.level(), 75);

        // Clamp to 100
        vol.set_level(150);
        assert_eq!(vol.level(), 100);
    }

    #[test]
    fn mute_unmute() {
        let mut vol = Volume::new(80);
        assert!(!vol.is_muted());

        vol.mute();
        assert!(vol.is_muted());
        assert_eq!(vol.level(), 80); // Level preserved

        vol.unmute();
        assert!(!vol.is_muted());
        assert_eq!(vol.level(), 80);
    }

    #[test]
    fn toggle_mute() {
        let mut vol = Volume::new(80);
        assert!(!vol.is_muted());

        vol.toggle_mute();
        assert!(vol.is_muted());

        vol.toggle_mute();
        assert!(!vol.is_muted());
    }

    #[test]
    fn gain_calculation() {
        // 0% should be near silence
        let vol = Volume::new(0);
        assert_eq!(vol.gain(), 0.0);

        // 100% should be unity gain
        let vol = Volume::new(100);
        assert!((vol.gain() - 1.0).abs() < 0.001);

        // 50% should be -30 dB (0.0316)
        let vol = Volume::new(50);
        assert!((vol.gain() - 0.0316).abs() < 0.001);

        // 80% should be -12 dB (0.251)
        let vol = Volume::new(80);
        assert!((vol.gain() - 0.251).abs() < 0.01);
    }

    #[test]
    fn muted_gain_is_zero() {
        let mut vol = Volume::new(80);
        assert!(vol.gain() > 0.0);

        vol.mute();
        assert_eq!(vol.gain(), 0.0);
    }

    #[test]
    fn apply_to_buffer() {
        let mut vol = Volume::new(100); // Unity gain
        let mut buffer = vec![0.5, 0.8, -0.3, -0.9];

        vol.apply(&mut buffer);

        // Should be unchanged at 100%
        assert!((buffer[0] - 0.5).abs() < 0.001);
        assert!((buffer[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn apply_muted() {
        let mut vol = Volume::new(80);
        vol.mute();

        // Process enough samples to complete the ramp (10ms at 44100Hz = 882 stereo samples)
        let mut ramp_buffer = vec![1.0f32; 1000];
        vol.apply(&mut ramp_buffer);

        // Now the mute ramp should be complete
        let mut buffer = vec![0.5, 0.8, -0.3, -0.9];
        vol.apply(&mut buffer);

        // All samples should be zero (muted)
        assert_eq!(buffer, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn apply_reduced_volume() {
        let mut vol = Volume::new(50); // -30 dB
        let mut buffer = vec![1.0];

        vol.apply(&mut buffer);

        // Should be approximately 0.0316
        assert!((buffer[0] - 0.0316).abs() < 0.001);
    }

    #[test]
    fn db_conversion() {
        let vol = Volume::new(100);
        assert!((vol.to_db() - 0.0).abs() < 0.1); // 100% ≈ 0 dB

        let vol = Volume::new(0);
        assert!((vol.to_db() + 60.0).abs() < 0.1); // 0% ≈ -60 dB

        let mut vol = Volume::new(80);
        vol.mute();
        assert!((vol.to_db() + 60.0).abs() < 0.1); // Muted ≈ -60 dB
    }

    #[test]
    fn logarithmic_scaling() {
        // Verify that volume feels linear to human perception
        let vol_25 = Volume::new(25);
        let vol_50 = Volume::new(50);
        let vol_75 = Volume::new(75);

        // Each step should feel like equal volume change
        // Verify dB scale is used (not linear)
        assert!(vol_25.gain() < 0.01); // Much quieter than 25% linear
        assert!(vol_50.gain() < 0.1); // Much quieter than 50% linear
        assert!(vol_75.gain() < 0.5); // Quieter than 75% linear
    }

    // ========================================
    // Ramping Tests
    // ========================================

    #[test]
    fn ramp_on_volume_change() {
        let mut vol = Volume::new(100);
        assert!(!vol.is_ramping());

        // Change volume - should start a ramp
        vol.set_level(50);
        assert!(vol.is_ramping());

        // Current gain should still be at the old value initially
        assert!((vol.current_gain() - 1.0).abs() < 0.001);

        // Target gain should be the new value
        assert!((vol.gain() - 0.0316).abs() < 0.001);
    }

    #[test]
    fn ramp_completes_after_processing() {
        let mut vol = Volume::new(100);
        vol.set_level(50);

        // Process enough samples to complete the ramp
        // 10ms at 44100Hz = 882 stereo samples
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Ramp should be complete
        assert!(!vol.is_ramping());

        // Current gain should match target
        assert!((vol.current_gain() - vol.gain()).abs() < 0.001);
    }

    #[test]
    fn ramp_is_smooth() {
        let mut vol = Volume::new(100);
        vol.set_level(0); // Ramp from 1.0 to 0.0

        // Process in small chunks and verify monotonic decrease
        let mut prev_gain = vol.current_gain();
        for _ in 0..20 {
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);

            // Gain should be decreasing (or equal at end)
            assert!(
                vol.current_gain() <= prev_gain + 0.001,
                "Gain should be monotonically decreasing"
            );
            prev_gain = vol.current_gain();
        }
    }

    #[test]
    fn overlapping_ramp_starts_from_current() {
        let mut vol = Volume::new(100);

        // Start a ramp
        vol.set_level(50);

        // Process partway through the ramp
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);

        // Capture the current gain mid-ramp
        let mid_gain = vol.current_gain();
        assert!(
            mid_gain > 0.0316 && mid_gain < 1.0,
            "Should be mid-ramp: {}",
            mid_gain
        );

        // Start a new ramp while the first is in progress
        vol.set_level(100);

        // The new ramp should start from the mid-ramp position, not from 0.0316
        assert!(
            (vol.current_gain() - mid_gain).abs() < 0.01,
            "New ramp should start from current position"
        );
    }

    #[test]
    fn tiny_change_skips_ramp() {
        let mut vol = Volume::new(100);

        // Make a tiny change that's below the threshold
        // 100 -> 99 at high volume is a small change
        let initial_gain = vol.gain();
        vol.set_level(99);
        let new_gain = vol.gain();

        // If the change is small enough, it should apply instantly
        if (new_gain - initial_gain).abs() < MIN_RAMP_THRESHOLD {
            assert!(
                !vol.is_ramping(),
                "Tiny changes should skip ramping to avoid accumulated error"
            );
        }
    }

    #[test]
    fn mute_creates_ramp() {
        let mut vol = Volume::new(80);
        vol.mute();

        // Should start ramping to 0.0
        assert!(vol.is_ramping());
        assert_eq!(vol.gain(), 0.0);
    }

    #[test]
    fn unmute_creates_ramp() {
        let mut vol = Volume::new(80);
        vol.mute();

        // Complete the mute ramp
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Unmute - should start ramping back up
        vol.unmute();
        assert!(vol.is_ramping());
        assert!((vol.gain() - 0.251).abs() < 0.01); // Target should be 80% gain
    }

    #[test]
    fn sample_rate_affects_ramp_duration() {
        let mut vol_44k = Volume::with_sample_rate(100, 44100);
        let mut vol_96k = Volume::with_sample_rate(100, 96000);

        vol_44k.set_level(50);
        vol_96k.set_level(50);

        // 96kHz should need more samples to complete the same duration
        // Process 500 samples - should complete 44.1k but not 96k
        let mut buffer = vec![1.0f32; 500];
        vol_44k.apply(&mut buffer);
        vol_96k.apply(&mut buffer);

        // 44.1k should be closer to completion (or done)
        // 96k should still have more ramping to do
        assert!(
            vol_44k.ramp_samples_remaining < vol_96k.ramp_samples_remaining,
            "Higher sample rate should require more samples for same duration"
        );
    }

    #[test]
    fn set_sample_rate_updates_ramp_duration() {
        let mut vol = Volume::new(80);
        let initial_samples = vol.ramp_duration_samples;

        vol.set_sample_rate(96000);

        assert!(
            vol.ramp_duration_samples > initial_samples,
            "Higher sample rate should have longer ramp in samples"
        );
    }

    #[test]
    fn rapid_changes_remain_smooth() {
        let mut vol = Volume::new(100);

        // Rapidly change volume multiple times
        for i in 0..5 {
            let level = if i % 2 == 0 { 20 } else { 80 };
            vol.set_level(level);

            // Process a small amount
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);

            // Verify no sudden jumps (buffer samples should be monotonic within each call)
            for j in 1..buffer.len() {
                let delta = (buffer[j] - buffer[j - 1]).abs();
                assert!(
                    delta < 0.1,
                    "Sample-to-sample change should be smooth: delta={}",
                    delta
                );
            }
        }
    }

    // ========================================
    // Volume Ramping Edge Case Tests
    // ========================================

    #[test]
    fn overlapping_ramp_interrupted_by_new_volume_change() {
        // Test: When a ramp is in progress and a new volume change is requested,
        // the new ramp should start from the current interpolated position.
        let mut vol = Volume::new(100);

        // Start first ramp: 100% -> 20%
        vol.set_level(20);
        assert!(vol.is_ramping());

        // Process halfway through the ramp
        let mut buffer = vec![1.0f32; 400];
        vol.apply(&mut buffer);

        // Capture the intermediate gain
        let mid_gain_first_ramp = vol.current_gain();
        assert!(
            mid_gain_first_ramp < 1.0 && mid_gain_first_ramp > 0.01,
            "Should be mid-ramp: {}",
            mid_gain_first_ramp
        );

        // Start second ramp: current position -> 80%
        vol.set_level(80);
        assert!(vol.is_ramping());

        // The new ramp should start from mid_gain_first_ramp
        assert!(
            (vol.current_gain() - mid_gain_first_ramp).abs() < 0.01,
            "Overlapping ramp should start from current position: {} vs {}",
            vol.current_gain(),
            mid_gain_first_ramp
        );

        // Process enough samples for the new ramp to clearly be in effect
        // and verify the final gain is closer to the new target (80%)
        let mut buffer2 = vec![1.0f32; 500];
        vol.apply(&mut buffer2);
        let gain_after_processing = vol.current_gain();

        // The target for 80% is about 0.251
        // The gain should have moved toward the new target
        let target_80_percent = 0.251;
        let distance_to_new_target = (gain_after_processing - target_80_percent).abs();
        let distance_from_old_start = (mid_gain_first_ramp - target_80_percent).abs();

        assert!(
            distance_to_new_target < distance_from_old_start,
            "Gain should move toward new target (80%): current={}, mid_ramp_start={}, target={}",
            gain_after_processing,
            mid_gain_first_ramp,
            target_80_percent
        );
    }

    #[test]
    fn multiple_overlapping_ramps_in_quick_succession() {
        // Test: Multiple rapid volume changes should all start from current position
        let mut vol = Volume::new(100);

        // Start ramp, interrupt, repeat
        let levels = [20, 80, 50, 100, 30, 70];
        let mut prev_gain = vol.current_gain();

        for &level in &levels {
            vol.set_level(level);

            // Verify new ramp starts from previous gain
            let current = vol.current_gain();
            assert!(
                (current - prev_gain).abs() < 0.02,
                "Ramp should start from previous position: {} vs {}",
                current,
                prev_gain
            );

            // Process a small amount and capture the new gain
            let mut buffer = vec![1.0f32; 100];
            vol.apply(&mut buffer);
            prev_gain = vol.current_gain();
        }
    }

    #[test]
    fn mute_during_active_volume_ramp() {
        // Test: Muting while a volume ramp is in progress should smoothly
        // transition from current gain to 0.0
        let mut vol = Volume::new(100);

        // Start ramp from 100% -> 50%
        vol.set_level(50);
        assert!(vol.is_ramping());

        // Process partway through the volume ramp
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);

        // Capture current gain mid-ramp
        let mid_ramp_gain = vol.current_gain();
        assert!(
            mid_ramp_gain > 0.0316 && mid_ramp_gain < 1.0,
            "Should be between 50% and 100% gain: {}",
            mid_ramp_gain
        );

        // Mute while ramp is in progress
        vol.mute();
        assert!(vol.is_ramping());

        // The mute ramp should start from the mid-ramp position
        assert!(
            (vol.current_gain() - mid_ramp_gain).abs() < 0.01,
            "Mute ramp should start from current gain: {} vs {}",
            vol.current_gain(),
            mid_ramp_gain
        );

        // Process and verify we're ramping toward 0.0
        let mut buffer2 = vec![1.0f32; 50];
        vol.apply(&mut buffer2);
        let gain_during_mute = vol.current_gain();

        assert!(
            gain_during_mute < mid_ramp_gain,
            "Gain should be decreasing toward mute: {} < {}",
            gain_during_mute,
            mid_ramp_gain
        );

        // Complete the mute ramp
        let mut buffer3 = vec![1.0f32; 1000];
        vol.apply(&mut buffer3);

        assert!(!vol.is_ramping());
        assert!(
            vol.current_gain() < 0.001,
            "Should be near zero: {}",
            vol.current_gain()
        );
    }

    #[test]
    fn unmute_during_active_mute_ramp() {
        // Test: Unmuting while mute ramp is in progress should reverse direction
        let mut vol = Volume::new(80);

        // Start mute
        vol.mute();

        // Process partway through the mute ramp
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);

        let mid_mute_gain = vol.current_gain();
        assert!(mid_mute_gain > 0.0 && mid_mute_gain < 0.251);

        // Unmute mid-ramp
        vol.unmute();
        assert!(vol.is_ramping());

        // Should start ramping back up from current position
        let gain_after_unmute = vol.current_gain();
        assert!(
            (gain_after_unmute - mid_mute_gain).abs() < 0.01,
            "Unmute should start from current position"
        );

        // Process more and verify gain is increasing
        let mut buffer2 = vec![1.0f32; 100];
        vol.apply(&mut buffer2);

        assert!(
            vol.current_gain() > mid_mute_gain,
            "Gain should be increasing after unmute"
        );
    }

    #[test]
    fn volume_change_while_muted_defers_until_unmute() {
        // Test: Volume changes while muted should update the level,
        // but the gain should stay at 0.0 until unmute
        let mut vol = Volume::new(80);
        vol.mute();

        // Complete the mute ramp
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert!(vol.current_gain() < 0.001);

        // Change volume while muted
        vol.set_level(50);
        assert_eq!(vol.level(), 50);

        // Gain should still be 0 (muted)
        assert!(vol.current_gain() < 0.001);

        // Unmute - should ramp to 50% gain
        vol.unmute();
        assert!(vol.is_ramping());

        // Process the unmute ramp
        let mut buffer2 = vec![1.0f32; 1000];
        vol.apply(&mut buffer2);

        // Should reach 50% gain (about 0.0316)
        assert!(
            (vol.current_gain() - 0.0316).abs() < 0.01,
            "Should be at 50% gain: {}",
            vol.current_gain()
        );
    }

    #[test]
    fn ramp_handles_extreme_sample_rates() {
        // Test: Verify ramping works correctly at extreme sample rates
        let sample_rates = [8000, 22050, 44100, 48000, 88200, 96000, 192000];

        for &rate in &sample_rates {
            let mut vol = Volume::with_sample_rate(100, rate);
            vol.set_level(50);

            // Process 50ms worth of samples (should complete any ramp)
            let samples_to_process = (rate as f32 * 0.05 * 2.0) as usize;
            let mut buffer = vec![1.0f32; samples_to_process];
            vol.apply(&mut buffer);

            // Ramp should be complete
            assert!(!vol.is_ramping(), "Ramp should complete at {}Hz", rate);

            // Verify final gain is correct
            assert!(
                (vol.current_gain() - 0.0316).abs() < 0.01,
                "Final gain should be correct at {}Hz: {}",
                rate,
                vol.current_gain()
            );
        }
    }

    #[test]
    fn ramp_with_buffer_smaller_than_ramp_duration() {
        // Test: Processing with very small buffers should still result in smooth ramping
        let mut vol = Volume::new(100);
        vol.set_level(50);

        // Process with tiny buffers (2 samples at a time)
        let mut all_gains = Vec::new();
        for _ in 0..500 {
            all_gains.push(vol.current_gain());
            let mut buffer = vec![1.0f32; 2];
            vol.apply(&mut buffer);
        }

        // Verify monotonic decrease
        for i in 1..all_gains.len() {
            assert!(
                all_gains[i] <= all_gains[i - 1] + 0.001,
                "Gain should be monotonically decreasing: {} vs {}",
                all_gains[i],
                all_gains[i - 1]
            );
        }
    }

    #[test]
    fn ramp_produces_no_audible_clicks() {
        // Test: Verify that the maximum sample-to-sample change is below
        // the audible click threshold (typically < 0.01 per sample is safe)
        let mut vol = Volume::new(100);
        vol.set_level(0); // Maximum possible ramp

        // Process the entire ramp
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Check sample-to-sample deltas
        let mut max_delta = 0.0f32;
        for i in 1..buffer.len() {
            let delta = (buffer[i] - buffer[i - 1]).abs();
            max_delta = max_delta.max(delta);
        }

        // Max delta should be small enough to avoid clicks
        assert!(
            max_delta < 0.05,
            "Max sample-to-sample delta too large: {}",
            max_delta
        );
    }

    // ========================================
    // Configuration Validation Tests
    // ========================================

    #[test]
    fn volume_level_clamped_to_max() {
        // Volume above 100 should be clamped
        let vol = Volume::new(150);
        assert_eq!(vol.level(), MAX_VOLUME_LEVEL);
    }

    #[test]
    fn volume_set_level_clamps_to_max() {
        let mut vol = Volume::new(50);
        vol.set_level(200);
        assert_eq!(vol.level(), MAX_VOLUME_LEVEL);
    }

    #[test]
    fn sample_rate_zero_clamped_to_min() {
        // Sample rate of 0 should be clamped to minimum
        let vol = Volume::with_sample_rate(80, 0);
        assert_eq!(vol.sample_rate, MIN_SAMPLE_RATE);
    }

    #[test]
    fn sample_rate_very_low_clamped_to_min() {
        let vol = Volume::with_sample_rate(80, 100);
        assert_eq!(vol.sample_rate, MIN_SAMPLE_RATE);
    }

    #[test]
    fn sample_rate_very_high_clamped_to_max() {
        let vol = Volume::with_sample_rate(80, 1000000);
        assert_eq!(vol.sample_rate, MAX_SAMPLE_RATE);
    }

    #[test]
    fn sample_rate_valid_passes_through() {
        let vol = Volume::with_sample_rate(80, 48000);
        assert_eq!(vol.sample_rate, 48000);
    }

    #[test]
    fn set_sample_rate_clamps_low() {
        let mut vol = Volume::new(80);
        vol.set_sample_rate(100);
        assert_eq!(vol.sample_rate, MIN_SAMPLE_RATE);
    }

    #[test]
    fn set_sample_rate_clamps_high() {
        let mut vol = Volume::new(80);
        vol.set_sample_rate(1000000);
        assert_eq!(vol.sample_rate, MAX_SAMPLE_RATE);
    }

    #[test]
    fn set_sample_rate_valid_value() {
        let mut vol = Volume::new(80);
        vol.set_sample_rate(96000);
        assert_eq!(vol.sample_rate, 96000);
    }

    #[test]
    fn volume_boundary_levels() {
        // Volume 0 should have 0 gain
        let vol = Volume::new(0);
        assert_eq!(vol.gain(), 0.0);

        // Volume 100 should have unity gain
        let vol = Volume::new(100);
        assert!((vol.gain() - 1.0).abs() < 0.001);
    }

    #[test]
    fn volume_gain_curve_is_logarithmic() {
        // Verify that 50% volume is much less than 50% linear gain
        // because of logarithmic scaling (human hearing perception)
        let vol = Volume::new(50);
        let gain = vol.gain();

        // At 50%, gain should be approximately -30dB which is ~0.0316
        // Linear would give 0.5, so logarithmic should be much less
        assert!(
            gain < 0.1,
            "50% volume should have logarithmic gain < 0.1, got {}",
            gain
        );
        assert!(
            gain > 0.01,
            "50% volume should have logarithmic gain > 0.01, got {}",
            gain
        );
    }

    // ========================================
    // Volume Boundary Edge Case Tests
    // ========================================

    #[test]
    fn volume_boundary_exact_zero() {
        let vol = Volume::new(0);
        assert_eq!(vol.level(), 0);
        assert_eq!(vol.gain(), 0.0);
        assert_eq!(vol.current_gain(), 0.0);

        // Verify applying to buffer produces silence
        let mut vol = Volume::new(0);
        let mut buffer = vec![0.5, 0.8, -0.3, -0.9, 1.0];
        vol.apply(&mut buffer);
        assert!(
            buffer.iter().all(|&s| s == 0.0),
            "All samples should be zero at volume 0"
        );
    }

    #[test]
    fn volume_boundary_exact_one() {
        let vol = Volume::new(1);
        assert_eq!(vol.level(), 1);
        // At 1%, gain should be very small but non-zero
        let gain = vol.gain();
        assert!(gain > 0.0, "Volume 1 should have non-zero gain");
        assert!(
            gain < 0.01,
            "Volume 1 should have very small gain: {}",
            gain
        );
    }

    #[test]
    fn volume_boundary_exact_fifty() {
        let vol = Volume::new(50);
        assert_eq!(vol.level(), 50);
        // 50% should be -30 dB = 10^(-30/20) = 0.0316
        let expected_gain = 0.0316;
        assert!(
            (vol.gain() - expected_gain).abs() < 0.001,
            "50% volume should be ~0.0316, got {}",
            vol.gain()
        );
    }

    #[test]
    fn volume_boundary_exact_ninety_nine() {
        let vol = Volume::new(99);
        assert_eq!(vol.level(), 99);
        // 99% should be -0.6 dB = 10^(-0.6/20) = 0.933
        let expected_gain = 0.933;
        assert!(
            (vol.gain() - expected_gain).abs() < 0.01,
            "99% volume should be ~0.933, got {}",
            vol.gain()
        );
    }

    #[test]
    fn volume_boundary_exact_hundred() {
        let vol = Volume::new(100);
        assert_eq!(vol.level(), 100);
        // 100% should be 0 dB = 1.0 unity gain
        assert!(
            (vol.gain() - 1.0).abs() < 0.001,
            "100% volume should be unity gain, got {}",
            vol.gain()
        );

        // Verify applying to buffer doesn't modify samples
        let mut vol = Volume::new(100);
        let original = vec![0.5, 0.8, -0.3, -0.9, 1.0];
        let mut buffer = original.clone();
        vol.apply(&mut buffer);
        for (i, (&orig, &result)) in original.iter().zip(buffer.iter()).enumerate() {
            assert!(
                (orig - result).abs() < 0.001,
                "Sample {} should be unchanged at 100%: {} vs {}",
                i,
                orig,
                result
            );
        }
    }

    #[test]
    fn volume_transitions_between_boundaries() {
        let mut vol = Volume::new(0);

        // 0 -> 100
        vol.set_level(100);
        assert!(vol.is_ramping());
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);
        assert!((vol.current_gain() - 1.0).abs() < 0.001);

        // 100 -> 0
        vol.set_level(0);
        assert!(vol.is_ramping());
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);
        assert_eq!(vol.current_gain(), 0.0);

        // 0 -> 50 -> 100
        vol.set_level(50);
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);
        assert!((vol.current_gain() - 0.0316).abs() < 0.001);

        vol.set_level(100);
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);
        assert!((vol.current_gain() - 1.0).abs() < 0.001);
    }

    // ========================================
    // Rapid Volume Change Tests
    // ========================================

    #[test]
    fn rapid_volume_change_every_sample() {
        // Simulate changing volume on every sample - stress test for click prevention
        let mut vol = Volume::new(50);

        let mut prev_gain = vol.current_gain();
        let mut max_delta = 0.0f32;

        // Alternate between two different volumes each sample
        for i in 0..1000 {
            let level = if i % 2 == 0 { 30 } else { 70 };
            vol.set_level(level);

            // Process one sample at a time
            let mut buffer = vec![1.0f32; 1];
            vol.apply(&mut buffer);

            let delta = (vol.current_gain() - prev_gain).abs();
            max_delta = max_delta.max(delta);
            prev_gain = vol.current_gain();
        }

        // Despite rapid changes, per-sample delta should be small enough to prevent clicks
        assert!(
            max_delta < 0.02,
            "Rapid changes should not cause large gain jumps: max_delta={}",
            max_delta
        );
    }

    #[test]
    fn rapid_random_volume_changes() {
        let mut vol = Volume::new(50);

        let levels = [0, 100, 25, 75, 50, 10, 90, 33, 67, 100, 0, 50];
        let mut all_gains = Vec::new();

        for &level in &levels {
            vol.set_level(level);

            // Process small amounts between each change
            for _ in 0..10 {
                all_gains.push(vol.current_gain());
                let mut buffer = vec![1.0f32; 10];
                vol.apply(&mut buffer);
            }
        }

        // Check for smooth transitions - no large jumps between consecutive gains
        let mut max_jump = 0.0f32;
        for i in 1..all_gains.len() {
            let jump = (all_gains[i] - all_gains[i - 1]).abs();
            max_jump = max_jump.max(jump);
        }

        assert!(
            max_jump < 0.1,
            "Random volume changes should not cause large gain jumps: max_jump={}",
            max_jump
        );
    }

    #[test]
    fn rapid_mute_unmute_cycles() {
        let mut vol = Volume::new(80);
        let mut prev_gain = vol.current_gain();
        let mut max_delta = 0.0f32;

        for _ in 0..100 {
            vol.toggle_mute();

            // Process a few samples
            for _ in 0..5 {
                let mut buffer = vec![1.0f32; 2];
                vol.apply(&mut buffer);

                let delta = (vol.current_gain() - prev_gain).abs();
                max_delta = max_delta.max(delta);
                prev_gain = vol.current_gain();
            }
        }

        assert!(
            max_delta < 0.02,
            "Rapid mute/unmute should not cause clicks: max_delta={}",
            max_delta
        );
    }

    // ========================================
    // Mute During Volume Ramp Tests
    // ========================================

    #[test]
    fn mute_during_volume_increase_ramp() {
        let mut vol = Volume::new(20);

        // Start ramping up
        vol.set_level(80);
        assert!(vol.is_ramping());

        // Process partway
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);
        let mid_ramp_gain = vol.current_gain();

        // Mute while ramping up
        vol.mute();
        assert!(vol.is_ramping());
        assert!(vol.is_muted());

        // Should start from current position and ramp to 0
        assert!(
            (vol.current_gain() - mid_ramp_gain).abs() < 0.01,
            "Mute should start from current gain"
        );

        // Process and verify decreasing
        let mut buffer = vec![1.0f32; 100];
        vol.apply(&mut buffer);
        assert!(
            vol.current_gain() < mid_ramp_gain,
            "Gain should decrease during mute ramp"
        );
    }

    #[test]
    fn mute_during_volume_decrease_ramp() {
        let mut vol = Volume::new(80);

        // Start ramping down
        vol.set_level(20);
        assert!(vol.is_ramping());

        // Process partway
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);
        let _mid_ramp_gain = vol.current_gain();

        // Mute while already ramping down
        vol.mute();
        assert!(vol.is_ramping());

        // Mute ramp should continue from current gain to 0
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert!(
            vol.current_gain() < 0.001,
            "Should reach near-zero after mute"
        );
    }

    #[test]
    fn mute_preserves_level_during_ramp() {
        let mut vol = Volume::new(50);

        // Start a ramp
        vol.set_level(80);

        // Mute mid-ramp
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);
        vol.mute();

        // Level should be preserved as 80
        assert_eq!(vol.level(), 80);

        // Complete the mute
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Unmute should restore to 80%
        vol.unmute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        let expected_gain = Volume::new(80).gain();
        assert!(
            (vol.current_gain() - expected_gain).abs() < 0.01,
            "Should restore to 80% gain: {} vs {}",
            vol.current_gain(),
            expected_gain
        );
    }

    // ========================================
    // Unmute to Different Volume Tests
    // ========================================

    #[test]
    fn unmute_to_different_volume_than_before_mute() {
        let mut vol = Volume::new(80);
        let gain_before_mute = vol.gain();

        // Mute
        vol.mute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Change volume while muted
        vol.set_level(30);
        assert_eq!(vol.level(), 30);
        assert!(
            vol.current_gain() < 0.001,
            "Gain should stay zero while muted"
        );

        // Unmute - should go to 30%, not 80%
        vol.unmute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        let expected_gain = Volume::new(30).gain();
        assert!(
            (vol.current_gain() - expected_gain).abs() < 0.01,
            "Should unmute to new volume (30%), not original (80%): {} vs {}",
            vol.current_gain(),
            expected_gain
        );
        assert!(
            (vol.current_gain() - gain_before_mute).abs() > 0.1,
            "Should be different from original gain"
        );
    }

    #[test]
    fn unmute_to_zero_volume() {
        let mut vol = Volume::new(80);

        vol.mute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Set volume to 0 while muted
        vol.set_level(0);

        // Unmute - should stay at 0
        vol.unmute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert_eq!(
            vol.current_gain(),
            0.0,
            "Unmuting to 0% should result in 0 gain"
        );
    }

    #[test]
    fn unmute_to_max_volume() {
        let mut vol = Volume::new(30);

        vol.mute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Set volume to 100 while muted
        vol.set_level(100);

        // Unmute - should go to unity gain
        vol.unmute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert!(
            (vol.current_gain() - 1.0).abs() < 0.001,
            "Unmuting to 100% should result in unity gain"
        );
    }

    // ========================================
    // Volume Change During Mute Tests
    // ========================================

    #[test]
    fn volume_change_while_muted_does_not_affect_current_gain() {
        let mut vol = Volume::new(80);

        vol.mute();
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert!(vol.current_gain() < 0.001);

        // Change volume multiple times while muted
        for level in [50, 30, 90, 10, 70] {
            vol.set_level(level);

            // Gain should remain zero
            assert!(
                vol.current_gain() < 0.001,
                "Gain should stay zero while muted even after setting level to {}",
                level
            );

            // Process some samples
            let mut buffer = vec![1.0f32; 100];
            vol.apply(&mut buffer);

            // Still zero
            assert!(
                vol.current_gain() < 0.001,
                "Gain should stay zero after processing while muted"
            );
        }

        // Final level should be preserved
        assert_eq!(vol.level(), 70);
    }

    #[test]
    fn volume_change_during_mute_ramp() {
        let mut vol = Volume::new(80);

        // Start mute
        vol.mute();

        // Process partway through mute ramp
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);
        let _mid_mute_gain = vol.current_gain();

        // Change volume during mute ramp
        vol.set_level(30);
        assert_eq!(vol.level(), 30);

        // Should continue ramping to 0 (mute takes precedence)
        assert_eq!(vol.gain(), 0.0, "Target should still be 0 (muted)");

        // Complete the mute
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        assert!(
            vol.current_gain() < 0.001,
            "Should reach zero despite volume change"
        );
    }

    // ========================================
    // Sample Rate Change During Ramp Tests
    // ========================================

    #[test]
    fn sample_rate_change_during_active_ramp() {
        let mut vol = Volume::with_sample_rate(100, 44100);

        // Start a ramp
        vol.set_level(50);
        assert!(vol.is_ramping());

        // Process partway
        let mut buffer = vec![1.0f32; 200];
        vol.apply(&mut buffer);
        let mid_ramp_gain = vol.current_gain();

        // Change sample rate while ramping
        let initial_remaining = vol.ramp_samples_remaining;
        vol.set_sample_rate(96000);

        // The ramp samples remaining should NOT change mid-ramp
        // (sample rate affects future ramps, not current one)
        assert_eq!(
            vol.ramp_samples_remaining, initial_remaining,
            "Changing sample rate should not affect current ramp"
        );

        // Current gain should not jump
        assert!(
            (vol.current_gain() - mid_ramp_gain).abs() < 0.001,
            "Gain should not jump on sample rate change"
        );

        // Complete the current ramp
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        // Start a NEW ramp - this should use the new sample rate
        vol.set_level(100);
        assert!(vol.is_ramping());

        // The new ramp should have more samples due to higher sample rate
        let expected_samples_96k = (96000 * RAMP_DURATION_MS * 2 / 1000) as usize;
        assert_eq!(
            vol.ramp_samples_total, expected_samples_96k,
            "New ramp should use updated sample rate"
        );
    }

    #[test]
    fn sample_rate_change_between_ramps() {
        let mut vol = Volume::with_sample_rate(100, 44100);

        // Complete a ramp at 44.1kHz
        vol.set_level(50);
        let initial_total = vol.ramp_samples_total;
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);
        assert!(!vol.is_ramping());

        // Change sample rate
        vol.set_sample_rate(192000);

        // Start a new ramp - should have more samples
        vol.set_level(100);
        let new_total = vol.ramp_samples_total;

        assert!(
            new_total > initial_total,
            "192kHz ramp should have more samples than 44.1kHz: {} vs {}",
            new_total,
            initial_total
        );

        // Verify the ratio is approximately correct
        let expected_ratio = 192000.0 / 44100.0;
        let actual_ratio = new_total as f32 / initial_total as f32;
        assert!(
            (actual_ratio - expected_ratio).abs() < 0.1,
            "Ramp sample ratio should match sample rate ratio: {} vs {}",
            actual_ratio,
            expected_ratio
        );
    }

    // ========================================
    // High Sample Rate (192kHz) Tests
    // ========================================

    #[test]
    fn high_sample_rate_192khz_ramp_duration() {
        let vol = Volume::with_sample_rate(80, 192000);

        // 10ms at 192kHz = 1920 samples, stereo = 3840
        let expected_samples = (192000 * RAMP_DURATION_MS * 2 / 1000) as usize;
        assert_eq!(
            vol.ramp_duration_samples, expected_samples,
            "192kHz should have correct ramp duration: {} vs {}",
            vol.ramp_duration_samples, expected_samples
        );
    }

    #[test]
    fn high_sample_rate_192khz_smooth_ramp() {
        let mut vol = Volume::with_sample_rate(100, 192000);
        vol.set_level(0);

        let mut prev_gain = vol.current_gain();
        let mut max_delta = 0.0f32;

        // Process the entire ramp in small chunks
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 10];
            vol.apply(&mut buffer);

            let delta = (vol.current_gain() - prev_gain).abs();
            max_delta = max_delta.max(delta);
            prev_gain = vol.current_gain();
        }

        // High sample rate should have very smooth transitions
        assert!(
            max_delta < 0.01,
            "192kHz should have very smooth ramping: max_delta={}",
            max_delta
        );
    }

    #[test]
    fn high_sample_rate_192khz_ramp_completes_in_10ms() {
        let mut vol = Volume::with_sample_rate(100, 192000);
        vol.set_level(50);

        // 10ms at 192kHz = 1920 mono samples = 3840 stereo samples
        let samples_for_10ms = (192000.0 * 0.01 * 2.0) as usize;
        let mut buffer = vec![1.0f32; samples_for_10ms];
        vol.apply(&mut buffer);

        assert!(
            !vol.is_ramping(),
            "Ramp should complete within 10ms worth of samples at 192kHz"
        );
    }

    #[test]
    fn high_sample_rate_no_clicks_on_volume_change() {
        let mut vol = Volume::with_sample_rate(100, 192000);
        vol.set_level(0);

        // Collect all samples during the ramp
        let mut all_samples = Vec::new();
        while vol.is_ramping() || all_samples.is_empty() {
            let mut buffer = vec![1.0f32; 100];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        // Check that maximum sample-to-sample change is small
        let mut max_sample_delta = 0.0f32;
        for i in 1..all_samples.len() {
            let delta = (all_samples[i] - all_samples[i - 1]).abs();
            max_sample_delta = max_sample_delta.max(delta);
        }

        assert!(
            max_sample_delta < 0.01,
            "No sample should change by more than 0.01 at 192kHz: max={}",
            max_sample_delta
        );
    }

    // ========================================
    // Low Sample Rate (8kHz) Tests
    // ========================================

    #[test]
    fn low_sample_rate_8khz_ramp_duration() {
        let vol = Volume::with_sample_rate(80, 8000);

        // 10ms at 8kHz = 80 samples, stereo = 160
        let expected_samples = (8000 * RAMP_DURATION_MS * 2 / 1000) as usize;
        assert_eq!(
            vol.ramp_duration_samples, expected_samples,
            "8kHz should have correct ramp duration: {} vs {}",
            vol.ramp_duration_samples, expected_samples
        );
    }

    #[test]
    fn low_sample_rate_8khz_still_smooth() {
        let mut vol = Volume::with_sample_rate(100, 8000);
        vol.set_level(0);

        let mut prev_gain = vol.current_gain();
        let mut max_delta = 0.0f32;

        // Process the entire ramp
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 2];
            vol.apply(&mut buffer);

            let delta = (vol.current_gain() - prev_gain).abs();
            max_delta = max_delta.max(delta);
            prev_gain = vol.current_gain();
        }

        // Even at low sample rates, ramping should be reasonably smooth
        // Delta will be larger due to fewer samples, but still should avoid clicks
        assert!(
            max_delta < 0.02,
            "8kHz should still have acceptably smooth ramping: max_delta={}",
            max_delta
        );
    }

    #[test]
    fn low_sample_rate_8khz_ramp_completes_in_10ms() {
        let mut vol = Volume::with_sample_rate(100, 8000);
        vol.set_level(50);

        // 10ms at 8kHz = 80 mono samples = 160 stereo samples
        let samples_for_10ms = (8000.0 * 0.01 * 2.0) as usize;
        let mut buffer = vec![1.0f32; samples_for_10ms];
        vol.apply(&mut buffer);

        assert!(
            !vol.is_ramping(),
            "Ramp should complete within 10ms worth of samples at 8kHz"
        );
    }

    #[test]
    fn low_sample_rate_8khz_no_clicks() {
        let mut vol = Volume::with_sample_rate(100, 8000);
        vol.set_level(0);

        // Process the entire ramp
        let mut all_samples = Vec::new();
        while vol.is_ramping() || all_samples.is_empty() {
            let mut buffer = vec![1.0f32; 10];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        // Check sample-to-sample changes
        let mut max_sample_delta = 0.0f32;
        for i in 1..all_samples.len() {
            let delta = (all_samples[i] - all_samples[i - 1]).abs();
            max_sample_delta = max_sample_delta.max(delta);
        }

        // At 8kHz, deltas will be larger but should still be below click threshold
        assert!(
            max_sample_delta < 0.02,
            "Sample changes should not cause clicks at 8kHz: max={}",
            max_sample_delta
        );
    }

    #[test]
    fn low_sample_rate_handles_tiny_buffers() {
        // At 8kHz with 160 samples for 10ms, using 1-sample buffers should still work
        let mut vol = Volume::with_sample_rate(100, 8000);
        vol.set_level(50);

        let mut samples_processed = 0;
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 1];
            vol.apply(&mut buffer);
            samples_processed += 1;

            // Safety: prevent infinite loop
            if samples_processed > 200 {
                break;
            }
        }

        assert!(
            !vol.is_ramping(),
            "Should complete ramp even with 1-sample buffers"
        );
        assert!(
            samples_processed <= 160,
            "Should not require more than ramp duration samples: {}",
            samples_processed
        );
    }

    // ========================================
    // Comprehensive Click Prevention Tests
    // ========================================

    #[test]
    fn click_test_extreme_volume_jump_high_to_zero() {
        // Most likely to cause clicks: 100% -> 0%
        let mut vol = Volume::new(100);
        vol.set_level(0);

        let mut all_samples = Vec::new();
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        verify_no_clicks(&all_samples, "100% to 0%");
    }

    #[test]
    fn click_test_extreme_volume_jump_zero_to_high() {
        // Also likely to cause clicks: 0% -> 100%
        let mut vol = Volume::new(0);
        vol.set_level(100);

        let mut all_samples = Vec::new();
        // Initial samples at 0 gain
        let mut buffer = vec![1.0f32; 50];
        vol.apply(&mut buffer);
        all_samples.extend(buffer);

        // Continue processing
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        verify_no_clicks(&all_samples, "0% to 100%");
    }

    #[test]
    fn click_test_mute_at_full_volume() {
        let mut vol = Volume::new(100);
        vol.mute();

        let mut all_samples = Vec::new();
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        verify_no_clicks(&all_samples, "mute from 100%");
    }

    #[test]
    fn click_test_unmute_to_full_volume() {
        let mut vol = Volume::new(100);
        vol.mute();

        // Complete mute
        let mut buffer = vec![1.0f32; 1000];
        vol.apply(&mut buffer);

        vol.unmute();

        let mut all_samples = Vec::new();
        while vol.is_ramping() {
            let mut buffer = vec![1.0f32; 50];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        verify_no_clicks(&all_samples, "unmute to 100%");
    }

    #[test]
    fn click_test_rapid_mute_unmute() {
        let mut vol = Volume::new(80);

        let mut all_samples = Vec::new();

        // Rapid mute/unmute without completing ramps
        for _ in 0..20 {
            vol.toggle_mute();
            let mut buffer = vec![1.0f32; 20];
            vol.apply(&mut buffer);
            all_samples.extend(buffer);
        }

        verify_no_clicks(&all_samples, "rapid mute/unmute");
    }

    /// Helper function to verify no audible clicks in a sample buffer
    fn verify_no_clicks(samples: &[f32], test_name: &str) {
        // Click threshold: maximum acceptable sample-to-sample change
        // Values above ~0.05 per sample are typically audible as clicks
        const CLICK_THRESHOLD: f32 = 0.05;

        let mut max_delta = 0.0f32;
        let mut click_positions = Vec::new();

        for i in 1..samples.len() {
            let delta = (samples[i] - samples[i - 1]).abs();
            if delta > CLICK_THRESHOLD {
                click_positions.push((i, delta));
            }
            max_delta = max_delta.max(delta);
        }

        assert!(
            click_positions.is_empty(),
            "{}: Found {} potential clicks (max delta: {}). Positions: {:?}",
            test_name,
            click_positions.len(),
            max_delta,
            click_positions.iter().take(5).collect::<Vec<_>>()
        );
    }

    // ========================================
    // Gain Calculation Verification Tests
    // ========================================

    #[test]
    fn verify_gain_at_all_boundary_levels() {
        let test_cases = [
            (0, 0.0),     // Silence
            (1, 0.00105), // Near minimum (-59.4 dB)
            (50, 0.0316), // -30 dB
            (99, 0.933),  // -0.6 dB
            (100, 1.0),   // Unity gain
        ];

        for (level, expected_gain) in test_cases {
            let vol = Volume::new(level);
            let actual_gain = vol.gain();

            // Use relative tolerance for non-zero values
            if expected_gain == 0.0 {
                assert_eq!(actual_gain, 0.0, "Level {} should have 0 gain", level);
            } else {
                let tolerance = expected_gain * 0.05; // 5% tolerance
                assert!(
                    (actual_gain - expected_gain).abs() < tolerance,
                    "Level {} should have gain ~{}, got {}",
                    level,
                    expected_gain,
                    actual_gain
                );
            }
        }
    }

    #[test]
    fn verify_db_conversion_accuracy() {
        let test_cases = [
            (0, -60.0),  // Minimum
            (50, -30.0), // Half
            (100, 0.0),  // Unity
        ];

        for (level, expected_db) in test_cases {
            let vol = Volume::new(level);
            let actual_db = vol.to_db();

            assert!(
                (actual_db - expected_db).abs() < 0.5,
                "Level {} should be ~{} dB, got {}",
                level,
                expected_db,
                actual_db
            );
        }
    }

    #[test]
    fn verify_gain_monotonic_increase() {
        // Verify that gain increases monotonically with level
        let mut prev_gain = 0.0f32;

        for level in 0..=100 {
            let vol = Volume::new(level);
            let gain = vol.gain();

            assert!(
                gain >= prev_gain,
                "Gain should increase monotonically: level {} has gain {} < previous {}",
                level,
                gain,
                prev_gain
            );

            prev_gain = gain;
        }
    }

    // ========================================
    // Smooth Transition Verification Tests
    // ========================================

    #[test]
    fn verify_ramp_is_linear_interpolation() {
        let mut vol = Volume::new(100);
        vol.set_level(0);

        let start_gain = vol.current_gain();
        let target_gain = vol.gain();
        let total_samples = vol.ramp_samples_total;

        // Collect gains after processing each sample
        // The apply() function updates current_gain during processing
        let mut gains = Vec::new();
        for _ in 0..total_samples {
            let mut buffer = vec![1.0f32; 1];
            vol.apply(&mut buffer);
            gains.push(vol.current_gain());
        }

        // Verify linear interpolation
        // After processing sample i, progress should be (i+1)/total_samples
        for (i, &gain) in gains.iter().enumerate() {
            let progress = (i + 1) as f32 / total_samples as f32;
            let expected_gain = start_gain + (target_gain - start_gain) * progress;

            // Allow small tolerance due to floating point
            assert!(
                (gain - expected_gain).abs() < 0.01,
                "Sample {}: expected gain {}, got {}",
                i,
                expected_gain,
                gain
            );
        }
    }

    #[test]
    fn verify_ramp_ends_exactly_at_target() {
        let mut vol = Volume::new(100);
        let target_level = 50;
        vol.set_level(target_level);

        // Process enough to complete ramp
        let mut buffer = vec![1.0f32; 2000];
        vol.apply(&mut buffer);

        let expected_gain = Volume::new(target_level).gain();
        assert!(
            (vol.current_gain() - expected_gain).abs() < 0.0001,
            "Ramp should end exactly at target gain: {} vs {}",
            vol.current_gain(),
            expected_gain
        );

        // Process more - gain should stay stable
        let gain_after = vol.current_gain();
        let mut buffer = vec![1.0f32; 100];
        vol.apply(&mut buffer);

        assert_eq!(
            vol.current_gain(),
            gain_after,
            "Gain should remain stable after ramp completes"
        );
    }
}
