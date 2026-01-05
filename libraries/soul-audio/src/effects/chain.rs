/// Effect chain for processing audio
///
/// This module provides a trait-based architecture for chaining audio effects.
/// Effects are processed in order, and all operate on f32 samples in [-1.0, 1.0] range.

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
}

/// Chain of audio effects processed in order
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
            gain: 0.0, // Would zero the signal
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
}
