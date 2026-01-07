//! Volume control with logarithmic scaling
//!
//! Provides human-perceptual volume control using dB scaling.
//! Volume range is 0-100%, mapped to -60 dB to 0 dB internally.

/// Volume controller with logarithmic scaling
///
/// Uses dB-based scaling to match human hearing perception.
/// Spotify-style: 0% = -60 dB (near silence), 100% = 0 dB (unity gain)
#[derive(Debug, Clone)]
pub struct Volume {
    /// Volume level (0-100)
    level: u8,

    /// Mute state (preserves volume level)
    muted: bool,

    /// Cached linear gain multiplier
    linear_gain: f32,
}

impl Volume {
    /// Create new volume controller
    ///
    /// # Arguments
    /// * `level` - Initial volume (0-100, default: 80)
    pub fn new(level: u8) -> Self {
        let level = level.min(100);
        let linear_gain = Self::calculate_linear_gain(level);

        Self {
            level,
            muted: false,
            linear_gain,
        }
    }

    /// Set volume level (0-100)
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(100);
        self.linear_gain = Self::calculate_linear_gain(self.level);
    }

    /// Get current volume level (0-100)
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Mute audio (preserves volume level)
    pub fn mute(&mut self) {
        self.muted = true;
    }

    /// Unmute audio (restores previous volume)
    pub fn unmute(&mut self) {
        self.muted = false;
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Get linear gain multiplier for audio processing
    ///
    /// Returns 0.0 if muted, otherwise logarithmic gain based on level
    pub fn gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.linear_gain
        }
    }

    /// Apply volume to audio buffer (in-place)
    ///
    /// Multiplies all samples by the current gain
    pub fn apply(&self, buffer: &mut [f32]) {
        let gain = self.gain();

        if gain == 0.0 {
            // Muted - fill with zeros
            buffer.fill(0.0);
        } else if gain != 1.0 {
            // Apply gain
            for sample in buffer.iter_mut() {
                *sample *= gain;
            }
        }
        // If gain == 1.0, no processing needed
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
    pub fn to_db(&self) -> f32 {
        if self.level == 0 || self.muted {
            -60.0
        } else {
            20.0 * self.linear_gain.log10()
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

        let mut buffer = vec![0.5, 0.8, -0.3, -0.9];
        vol.apply(&mut buffer);

        // All samples should be zero
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
}
