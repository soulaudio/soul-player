//! Noise shaper for sigma-delta modulation
//!
//! Provides various noise shaping filters to push quantization noise
//! out of the audible range during 1-bit conversion.

/// Noise shaper order (filter complexity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseShaperOrder {
    /// First order: Simple feedback, low CPU
    /// Good for very high DSD rates (DSD256+)
    First,

    /// Second order: Better noise shaping
    /// Good balance of quality and CPU
    #[default]
    Second,

    /// Third order: Even better noise shaping
    /// Recommended for DSD64/DSD128
    Third,

    /// Fifth order: Professional quality
    /// Best for mastering/high-quality conversion
    Fifth,
}

impl NoiseShaperOrder {
    /// Get the filter order (number of integrators)
    pub fn order(&self) -> usize {
        match self {
            NoiseShaperOrder::First => 1,
            NoiseShaperOrder::Second => 2,
            NoiseShaperOrder::Third => 3,
            NoiseShaperOrder::Fifth => 5,
        }
    }
}

/// Noise shaper for sigma-delta modulation
///
/// Implements multi-order noise shaping with configurable order.
/// Uses a MASH (Multi-stage noise shaping) inspired architecture.
pub struct NoiseShaper {
    /// Filter order
    order: NoiseShaperOrder,

    /// Integrator states (per channel, per order)
    /// Format: [channel][order]
    integrators: Vec<Vec<f64>>,

    /// Number of channels
    channels: usize,
}

impl NoiseShaper {
    /// Create a new noise shaper
    pub fn new(order: NoiseShaperOrder, channels: usize) -> Self {
        let order_count = order.order();
        let integrators = (0..channels).map(|_| vec![0.0; order_count]).collect();

        Self {
            order,
            integrators,
            channels,
        }
    }

    /// Process a single sample through the noise shaper
    ///
    /// Returns the shaped output value before quantization
    #[inline]
    pub fn process(&mut self, sample: f64, channel: usize) -> f64 {
        if channel >= self.channels {
            return sample;
        }

        let integrators = &mut self.integrators[channel];
        let order = self.order.order();

        // Input to noise shaper
        let mut x = sample;

        // Process through each integrator stage
        // Each stage adds the input to its state and passes to next
        for i in 0..order {
            integrators[i] += x;
            x = integrators[i];
        }

        x
    }

    /// Feedback quantization error into the noise shaper
    ///
    /// Call this after quantization with the error (input - quantized_output)
    #[inline]
    pub fn feedback(&mut self, error: f64, channel: usize) {
        if channel >= self.channels {
            return;
        }

        let integrators = &mut self.integrators[channel];
        let order = self.order.order();

        // Feedback error with coefficients for each order
        // Coefficients are carefully tuned for stability and noise shaping
        // Lower coefficients prevent divergence while still shaping noise
        match order {
            1 => {
                // First order: simple integrator feedback
                integrators[0] -= error * 0.9;
            }
            2 => {
                // Second-order: moderate shaping with stability
                integrators[0] -= error * 1.5;
                integrators[1] -= error * 0.5;
            }
            3 => {
                // Third-order: good shaping, stable coefficients
                integrators[0] -= error * 1.8;
                integrators[1] -= error * 1.2;
                integrators[2] -= error * 0.4;
            }
            5 => {
                // Fifth-order: aggressive but stable
                // Using smaller coefficients to prevent divergence
                integrators[0] -= error * 2.0;
                integrators[1] -= error * 1.5;
                integrators[2] -= error * 1.0;
                integrators[3] -= error * 0.5;
                integrators[4] -= error * 0.2;
            }
            _ => {
                // Fallback: conservative coefficients
                for (i, int) in integrators.iter_mut().enumerate() {
                    let coeff = 0.5 * (order - i) as f64 / order as f64;
                    *int -= error * coeff;
                }
            }
        }

        // Stability clamp: prevent integrator overflow
        for int in integrators.iter_mut() {
            *int = int.clamp(-100.0, 100.0);
        }
    }

    /// Reset the noise shaper state
    pub fn reset(&mut self) {
        for channel in &mut self.integrators {
            for state in channel.iter_mut() {
                *state = 0.0;
            }
        }
    }

    /// Get the noise shaper order
    pub fn order(&self) -> NoiseShaperOrder {
        self.order
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.channels
    }
}

impl Default for NoiseShaper {
    fn default() -> Self {
        Self::new(NoiseShaperOrder::Second, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_shaper_order() {
        assert_eq!(NoiseShaperOrder::First.order(), 1);
        assert_eq!(NoiseShaperOrder::Second.order(), 2);
        assert_eq!(NoiseShaperOrder::Third.order(), 3);
        assert_eq!(NoiseShaperOrder::Fifth.order(), 5);
    }

    #[test]
    fn test_noise_shaper_creation() {
        let shaper = NoiseShaper::new(NoiseShaperOrder::Third, 2);
        assert_eq!(shaper.order(), NoiseShaperOrder::Third);
        assert_eq!(shaper.channels(), 2);
    }

    #[test]
    fn test_noise_shaper_process() {
        let mut shaper = NoiseShaper::new(NoiseShaperOrder::First, 1);

        // Process a sample
        let output = shaper.process(0.5, 0);

        // First order integrator should accumulate
        assert!(output.abs() > 0.0);
    }

    #[test]
    fn test_noise_shaper_feedback() {
        let mut shaper = NoiseShaper::new(NoiseShaperOrder::Second, 1);

        // Process and feedback
        let input = 0.3;
        let _output = shaper.process(input, 0);
        let quantized = 1.0; // Assume quantized to +1
        let error = input - quantized;

        shaper.feedback(error, 0);

        // State should have changed
        // (verifying internals would require exposing state)
    }

    #[test]
    fn test_noise_shaper_reset() {
        let mut shaper = NoiseShaper::new(NoiseShaperOrder::Third, 2);

        // Process some samples
        for _ in 0..10 {
            shaper.process(0.5, 0);
            shaper.process(-0.5, 1);
        }

        // Reset
        shaper.reset();

        // State should be zero (process fresh input)
        let output = shaper.process(0.5, 0);

        // With reset state, output should be just the first integration
        assert!(output.abs() < 1.0);
    }

    #[test]
    fn test_invalid_channel() {
        let mut shaper = NoiseShaper::new(NoiseShaperOrder::First, 2);

        // Invalid channel should return input unchanged
        let output = shaper.process(0.5, 5);
        assert_eq!(output, 0.5);
    }
}
