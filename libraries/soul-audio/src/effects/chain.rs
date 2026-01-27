/// Effect chain for processing audio
///
/// This module provides a trait-based architecture for chaining audio effects.
/// Effects are processed in order, and all operate on f32 samples in [-1.0, 1.0] range.
///
/// # Threading Model
///
/// `EffectChain` is NOT internally synchronized. The caller MUST provide external
/// synchronization when the chain is shared between threads. In this codebase:
///
/// - `PlaybackManager` contains `effect_chain: EffectChain`
/// - `DesktopPlayback` wraps `PlaybackManager` in `Arc<Mutex<PlaybackManager>>`
/// - Both audio callback and configuration code lock this mutex before access
///
/// This external mutex approach is preferred over internal locking because:
/// 1. Avoids lock contention overhead in the audio callback hot path
/// 2. Allows batch operations on multiple fields under a single lock
/// 3. Enables the audio callback to hold the lock for the entire `process()` call
///
/// # Parameter Updates During Processing
///
/// Individual effects (like `ParametricEq`) use coefficient smoothing to handle
/// parameter changes gracefully. When parameters are updated while audio is processing:
/// - New values are stored as "target" coefficients
/// - Active coefficients smoothly interpolate toward targets
/// - This prevents clicks, pops, and zipper noise
///
/// Effects that need smoothing implement it internally - the chain itself does not
/// provide parameter smoothing.
use std::any::Any;

/// Trait for audio effects that can be chained together
///
/// # Safety
/// - Must NOT allocate memory in `process()` (real-time constraint)
/// - Must be Send to allow multi-threaded audio processing
pub trait AudioEffect: Send {
    /// Process audio buffer in-place
    ///
    /// # Arguments
    /// * `buffer` - Interleaved stereo samples (L, R, L, R, ...)
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Real-Time Constraints
    /// - No allocations
    /// - No blocking operations
    /// - Deterministic execution time
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32);

    /// Reset effect state (e.g., when seeking or changing tracks)
    fn reset(&mut self);

    /// Enable/disable the effect
    fn set_enabled(&mut self, enabled: bool);

    /// Check if effect is enabled
    fn is_enabled(&self) -> bool;

    /// Get effect name (for debugging)
    fn name(&self) -> &str;

    /// Get a reference to self as Any for downcasting
    /// Required for in-place parameter updates without rebuilding
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to self as Any for downcasting
    /// Required for in-place parameter updates without rebuilding
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Notify effect of sample rate change
    ///
    /// Called when the audio device sample rate changes. Effects that cache
    /// sample rate should update their internal state and recalculate
    /// any sample-rate-dependent parameters (e.g., filter coefficients).
    ///
    /// Default implementation does nothing - effects that don't cache
    /// sample rate can ignore this.
    fn set_sample_rate(&mut self, _sample_rate: u32) {
        // Default: no-op for effects that get sample_rate from process()
    }
}

/// Chain of audio effects processed in order
///
/// # Thread Safety
///
/// This struct is `Send` but NOT `Sync`. It must be protected by external
/// synchronization (typically `Mutex` or `RwLock`) when accessed from multiple
/// threads. All methods (`add_effect`, `process`, `set_enabled`, etc.) require
/// `&mut self` to enforce exclusive access.
///
/// # Concurrent Modification Safety
///
/// When wrapped in a mutex (as in `DesktopPlayback`):
/// - `add_effect`/`clear`: Safe - mutex ensures exclusive access
/// - `process`: Safe - mutex held for entire audio callback
/// - `set_enabled` on effects: Safe - mutex held during modification
/// - Parameter updates via `get_effect_as_mut`: Safe - mutex held during update
///
/// The audio callback locks the mutex for the entire `process_audio()` call,
/// ensuring no other thread can modify the chain or effect parameters during
/// processing.
pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}

impl EffectChain {
    /// Create a new empty effect chain
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Add an effect to the end of the chain
    pub fn add_effect(&mut self, effect: Box<dyn AudioEffect>) {
        self.effects.push(effect);
    }

    /// Process audio through the entire effect chain
    ///
    /// # Arguments
    /// * `buffer` - Interleaved stereo samples (L, R, L, R, ...)
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Real-Time Safety
    /// - Safe for real-time audio threads
    /// - No allocations after setup
    pub fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        for effect in &mut self.effects {
            if effect.is_enabled() {
                effect.process(buffer, sample_rate);
            }
        }
    }

    /// Reset all effects in the chain
    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }

    /// Clear all effects from the chain
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Get number of effects in chain
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Get effect at index
    pub fn get_effect(&self, index: usize) -> Option<&dyn AudioEffect> {
        self.effects.get(index).map(|e| e.as_ref())
    }

    /// Get mutable effect at index
    pub fn get_effect_mut(&mut self, index: usize) -> Option<&mut dyn AudioEffect> {
        if let Some(effect) = self.effects.get_mut(index) {
            Some(effect.as_mut())
        } else {
            None
        }
    }

    /// Enable/disable all effects
    pub fn set_enabled(&mut self, enabled: bool) {
        for effect in &mut self.effects {
            effect.set_enabled(enabled);
        }
    }

    /// Replace effect at index, or add if index equals current length
    ///
    /// Returns the old effect if one was replaced, None otherwise.
    /// This preserves other effects in the chain (doesn't clear everything).
    pub fn replace_effect(
        &mut self,
        index: usize,
        effect: Box<dyn AudioEffect>,
    ) -> Option<Box<dyn AudioEffect>> {
        match index.cmp(&self.effects.len()) {
            std::cmp::Ordering::Less => Some(std::mem::replace(&mut self.effects[index], effect)),
            std::cmp::Ordering::Equal => {
                self.effects.push(effect);
                None
            }
            std::cmp::Ordering::Greater => {
                // Index out of bounds - just append the effect
                // This shouldn't normally happen in practice
                self.effects.push(effect);
                None
            }
        }
    }

    /// Get effect at index, downcasted to specific type
    ///
    /// Use this for in-place parameter updates:
    /// ```ignore
    /// if let Some(eq) = chain.get_effect_as_mut::<ParametricEq>(0) {
    ///     eq.set_band(0, new_band);
    /// }
    /// ```
    pub fn get_effect_as<T: 'static>(&self, index: usize) -> Option<&T> {
        self.effects
            .get(index)
            .and_then(|e| e.as_any().downcast_ref::<T>())
    }

    /// Get mutable effect at index, downcasted to specific type
    pub fn get_effect_as_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.effects
            .get_mut(index)
            .and_then(|e| e.as_any_mut().downcast_mut::<T>())
    }

    /// Notify all effects of sample rate change
    ///
    /// Call this when the audio device sample rate changes. This propagates
    /// the sample rate to all effects in the chain, allowing them to update
    /// any cached sample-rate-dependent parameters (e.g., filter coefficients).
    ///
    /// Note: Effects also receive sample_rate via process(), but this method
    /// allows proactive updates when the device changes before processing resumes.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        for effect in &mut self.effects {
            effect.set_sample_rate(sample_rate);
        }
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock effect for testing
    struct GainEffect {
        gain: f32,
        enabled: bool,
    }

    impl AudioEffect for GainEffect {
        fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
            for sample in buffer.iter_mut() {
                *sample *= self.gain;
            }
        }

        fn reset(&mut self) {
            // Nothing to reset for gain
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn name(&self) -> &str {
            "Gain"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn empty_chain() {
        let chain = EffectChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn add_effects() {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 2.0,
            enabled: true,
        }));

        assert_eq!(chain.len(), 2);
        assert!(!chain.is_empty());
    }

    #[test]
    fn process_chain() {
        let mut chain = EffectChain::new();

        // Add gain of 0.5, then gain of 2.0
        // Result: 0.5 * 2.0 = 1.0 (no change)
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 2.0,
            enabled: true,
        }));

        let mut buffer = vec![1.0; 100]; // 50 stereo samples
        chain.process(&mut buffer, 44100);

        // Should be unchanged (0.5 * 2.0 = 1.0)
        for sample in &buffer {
            assert!((sample - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn disabled_effect_bypassed() {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(GainEffect {
            gain: 0.0,      // Would zero the signal
            enabled: false, // But it's disabled
        }));

        let mut buffer = vec![1.0; 100];
        chain.process(&mut buffer, 44100);

        // Should be unchanged (effect disabled)
        for sample in &buffer {
            assert!((sample - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn reset_chain() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        chain.reset(); // Should not panic
    }

    #[test]
    fn clear_chain() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        assert_eq!(chain.len(), 1);

        chain.clear();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn get_effect() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        let effect = chain.get_effect(0).unwrap();
        assert_eq!(effect.name(), "Gain");

        assert!(chain.get_effect(1).is_none());
    }

    #[test]
    fn enable_disable_all() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        chain.set_enabled(false);

        let mut buffer = vec![1.0; 100];
        chain.process(&mut buffer, 44100);

        // Should be unchanged (all effects disabled)
        for sample in &buffer {
            assert!((sample - 1.0).abs() < 0.0001);
        }
    }

    // ==================== Multiple Effects Enable/Disable Tests ====================

    #[test]
    fn enable_disable_multiple_effects_simultaneously() {
        // Test: Enabling/disabling all effects at once should work correctly
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.25,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 2.0,
            enabled: true,
        }));

        // All enabled: 0.5 * 0.25 * 2.0 = 0.25
        let mut buffer = vec![1.0; 10];
        chain.process(&mut buffer, 44100);
        for sample in &buffer {
            assert!(
                (sample - 0.25).abs() < 0.0001,
                "Expected 0.25, got {}",
                sample
            );
        }

        // Disable all simultaneously
        chain.set_enabled(false);

        let mut buffer2 = vec![1.0; 10];
        chain.process(&mut buffer2, 44100);
        for sample in &buffer2 {
            assert!((sample - 1.0).abs() < 0.0001, "Should bypass when disabled");
        }

        // Re-enable all
        chain.set_enabled(true);

        let mut buffer3 = vec![1.0; 10];
        chain.process(&mut buffer3, 44100);
        for sample in &buffer3 {
            assert!(
                (sample - 0.25).abs() < 0.0001,
                "Should process when re-enabled"
            );
        }
    }

    #[test]
    fn mixed_enable_disable_states() {
        // Test: Chain processes correctly when effects have mixed enabled states
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.1, // Would significantly reduce if enabled
            enabled: false,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 2.0,
            enabled: true,
        }));

        // Only first and third effects: 0.5 * 2.0 = 1.0
        let mut buffer = vec![0.5; 10];
        chain.process(&mut buffer, 44100);
        for sample in &buffer {
            assert!(
                (sample - 0.5).abs() < 0.0001,
                "Expected 0.5, got {}",
                sample
            );
        }
    }

    #[test]
    fn toggle_individual_effects_during_processing() {
        // Test: Toggling individual effects mid-stream
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        // Process first buffer: 0.5 * 0.5 = 0.25
        let mut buffer1 = vec![1.0; 10];
        chain.process(&mut buffer1, 44100);

        // Disable first effect
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Process second buffer: only second effect: 0.5
        let mut buffer2 = vec![1.0; 10];
        chain.process(&mut buffer2, 44100);
        for sample in &buffer2 {
            assert!((sample - 0.5).abs() < 0.0001);
        }
    }

    // ==================== Odd Buffer Size Tests ====================

    #[test]
    fn chain_with_odd_buffer_size() {
        // Test: Chain should handle odd buffer sizes gracefully
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        // Process odd-sized buffer
        let mut buffer = vec![1.0; 101]; // Odd length
        chain.process(&mut buffer, 44100);

        // All samples should be processed
        for sample in &buffer {
            assert!((sample - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn chain_with_very_small_buffer() {
        // Test: Very small buffers should still work
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 2.0,
            enabled: true,
        }));

        // Process single sample
        let mut buffer = vec![0.8];
        chain.process(&mut buffer, 44100);
        assert!((buffer[0] - 0.8).abs() < 0.0001); // 0.8 * 0.5 * 2.0 = 0.8

        // Process two samples (one stereo pair)
        let mut buffer2 = vec![0.8, 0.6];
        chain.process(&mut buffer2, 44100);
        assert!((buffer2[0] - 0.8).abs() < 0.0001);
        assert!((buffer2[1] - 0.6).abs() < 0.0001);
    }

    #[test]
    fn chain_with_empty_buffer() {
        // Test: Empty buffer should not crash
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        let mut buffer: Vec<f32> = vec![];
        chain.process(&mut buffer, 44100);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn chain_with_prime_number_buffer_size() {
        // Test: Prime number buffer sizes to catch edge cases
        let prime_sizes = [7, 11, 13, 17, 23, 29, 31, 37, 41];

        for &size in &prime_sizes {
            let mut chain = EffectChain::new();
            chain.add_effect(Box::new(GainEffect {
                gain: 0.5,
                enabled: true,
            }));

            let mut buffer = vec![1.0; size];
            chain.process(&mut buffer, 44100);

            for sample in &buffer {
                assert!(
                    (sample - 0.5).abs() < 0.0001,
                    "Buffer size {} failed: {}",
                    size,
                    sample
                );
            }
        }
    }

    // ==================== Effect Chain Modification During Processing ====================

    #[test]
    fn replace_effect_preserves_chain_order() {
        // Test: Replacing an effect should not affect other effects
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        // Initial: 0.5^3 = 0.125
        let mut buffer1 = vec![1.0; 10];
        chain.process(&mut buffer1, 44100);
        for sample in &buffer1 {
            assert!((sample - 0.125).abs() < 0.001);
        }

        // Replace middle effect with 2x gain
        chain.replace_effect(
            1,
            Box::new(GainEffect {
                gain: 2.0,
                enabled: true,
            }),
        );

        // New: 0.5 * 2.0 * 0.5 = 0.5
        let mut buffer2 = vec![1.0; 10];
        chain.process(&mut buffer2, 44100);
        for sample in &buffer2 {
            assert!((sample - 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn get_effect_as_mut_allows_parameter_changes() {
        // Test: get_effect_as_mut should allow modifying effect parameters
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect {
            gain: 0.5,
            enabled: true,
        }));

        // Modify the effect's gain
        if let Some(gain_effect) = chain.get_effect_as_mut::<GainEffect>(0) {
            gain_effect.gain = 0.25;
        }

        // Process with new gain
        let mut buffer = vec![1.0; 10];
        chain.process(&mut buffer, 44100);
        for sample in &buffer {
            assert!((sample - 0.25).abs() < 0.0001);
        }
    }

    #[test]
    fn sample_rate_change_propagates_to_all_effects() {
        // Test: Sample rate change should propagate to all effects
        // Create a mock effect that tracks sample rate changes
        struct SampleRateTrackingEffect {
            last_sample_rate: u32,
            enabled: bool,
        }

        impl AudioEffect for SampleRateTrackingEffect {
            fn process(&mut self, _buffer: &mut [f32], sample_rate: u32) {
                self.last_sample_rate = sample_rate;
            }

            fn reset(&mut self) {}

            fn set_enabled(&mut self, enabled: bool) {
                self.enabled = enabled;
            }

            fn is_enabled(&self) -> bool {
                self.enabled
            }

            fn name(&self) -> &str {
                "SampleRateTracker"
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn set_sample_rate(&mut self, sample_rate: u32) {
                self.last_sample_rate = sample_rate;
            }
        }

        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(SampleRateTrackingEffect {
            last_sample_rate: 0,
            enabled: true,
        }));
        chain.add_effect(Box::new(SampleRateTrackingEffect {
            last_sample_rate: 0,
            enabled: true,
        }));

        // Set sample rate via chain method
        chain.set_sample_rate(96000);

        // Verify both effects received the sample rate
        if let Some(effect) = chain.get_effect_as::<SampleRateTrackingEffect>(0) {
            assert_eq!(effect.last_sample_rate, 96000);
        }
        if let Some(effect) = chain.get_effect_as::<SampleRateTrackingEffect>(1) {
            assert_eq!(effect.last_sample_rate, 96000);
        }
    }

    #[test]
    fn reset_propagates_to_all_effects() {
        // Test: Reset should propagate to all effects
        struct ResetTrackingEffect {
            reset_count: usize,
            enabled: bool,
        }

        impl AudioEffect for ResetTrackingEffect {
            fn process(&mut self, _buffer: &mut [f32], _sample_rate: u32) {}

            fn reset(&mut self) {
                self.reset_count += 1;
            }

            fn set_enabled(&mut self, enabled: bool) {
                self.enabled = enabled;
            }

            fn is_enabled(&self) -> bool {
                self.enabled
            }

            fn name(&self) -> &str {
                "ResetTracker"
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ResetTrackingEffect {
            reset_count: 0,
            enabled: true,
        }));
        chain.add_effect(Box::new(ResetTrackingEffect {
            reset_count: 0,
            enabled: true,
        }));
        chain.add_effect(Box::new(ResetTrackingEffect {
            reset_count: 0,
            enabled: true,
        }));

        // Reset the chain
        chain.reset();

        // Verify all effects were reset
        for i in 0..3 {
            if let Some(effect) = chain.get_effect_as::<ResetTrackingEffect>(i) {
                assert_eq!(effect.reset_count, 1, "Effect {} should be reset once", i);
            }
        }
    }
}
