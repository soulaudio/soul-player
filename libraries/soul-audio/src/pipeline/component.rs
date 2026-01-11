//! Pipeline Component Trait
//!
//! Base interface that all audio pipeline components must implement.

use std::any::Any;

/// Information about a pipeline component for introspection
#[derive(Debug, Clone)]
pub struct PipelineComponentInfo {
    /// Unique type identifier (e.g., "parametric_eq", "compressor")
    pub type_id: &'static str,
    /// Human-readable name
    pub display_name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Whether this component supports in-place parameter updates
    pub supports_in_place_update: bool,
}

/// Base trait for all audio pipeline components
///
/// # Safety
/// - `process()` must NOT allocate memory (real-time constraint)
/// - Components must be Send to allow multi-threaded audio processing
///
/// # Implementation Requirements
/// All pipeline components must:
/// 1. Implement `process()` for audio processing (real-time safe)
/// 2. Implement `reset()` for state clearing (track changes, seeking)
/// 3. Implement `info()` for component metadata
/// 4. Implement `as_any()` / `as_any_mut()` for downcasting
/// 5. Optionally implement `update_parameters()` for in-place updates
pub trait PipelineComponent: Send {
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

    /// Reset component state (e.g., when seeking or changing tracks)
    fn reset(&mut self);

    /// Enable/disable the component
    fn set_enabled(&mut self, enabled: bool);

    /// Check if component is enabled
    fn is_enabled(&self) -> bool;

    /// Get component information
    fn info(&self) -> PipelineComponentInfo;

    /// Get a reference to self as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to self as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Update parameters in-place from a generic parameters object
    ///
    /// Returns true if update was successful, false if type mismatch or unsupported.
    /// Default implementation returns false (rebuild required).
    ///
    /// # Arguments
    /// * `params` - Type-erased parameters that should match this component's parameter type
    fn update_parameters(&mut self, _params: &dyn Any) -> bool {
        false
    }
}

/// Helper macro to implement PipelineComponent boilerplate for effects
///
/// This reduces repetitive code when implementing PipelineComponent for effects.
#[macro_export]
macro_rules! impl_pipeline_component {
    (
        $type:ty,
        type_id: $type_id:literal,
        display_name: $display_name:literal,
        description: $desc:literal,
        supports_in_place: $supports:expr,
        params_type: $params:ty
    ) => {
        impl $crate::pipeline::PipelineComponent for $type {
            fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
                <Self as $crate::effects::AudioEffect>::process(self, buffer, sample_rate)
            }

            fn reset(&mut self) {
                <Self as $crate::effects::AudioEffect>::reset(self)
            }

            fn set_enabled(&mut self, enabled: bool) {
                <Self as $crate::effects::AudioEffect>::set_enabled(self, enabled)
            }

            fn is_enabled(&self) -> bool {
                <Self as $crate::effects::AudioEffect>::is_enabled(self)
            }

            fn info(&self) -> $crate::pipeline::PipelineComponentInfo {
                $crate::pipeline::PipelineComponentInfo {
                    type_id: $type_id,
                    display_name: $display_name,
                    description: $desc,
                    supports_in_place_update: $supports,
                }
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn update_parameters(&mut self, params: &dyn std::any::Any) -> bool {
                if let Some(p) = params.downcast_ref::<$params>() {
                    self.apply_parameters(p);
                    true
                } else {
                    false
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestComponent {
        enabled: bool,
        gain: f32,
    }

    impl PipelineComponent for TestComponent {
        fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
            if self.enabled {
                for sample in buffer.iter_mut() {
                    *sample *= self.gain;
                }
            }
        }

        fn reset(&mut self) {
            // Nothing to reset
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn info(&self) -> PipelineComponentInfo {
            PipelineComponentInfo {
                type_id: "test",
                display_name: "Test Component",
                description: "A test component",
                supports_in_place_update: true,
            }
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn update_parameters(&mut self, params: &dyn Any) -> bool {
            if let Some(gain) = params.downcast_ref::<f32>() {
                self.gain = *gain;
                true
            } else {
                false
            }
        }
    }

    #[test]
    fn test_component_info() {
        let comp = TestComponent {
            enabled: true,
            gain: 1.0,
        };
        let info = comp.info();
        assert_eq!(info.type_id, "test");
        assert!(info.supports_in_place_update);
    }

    #[test]
    fn test_in_place_update() {
        let mut comp = TestComponent {
            enabled: true,
            gain: 1.0,
        };

        let new_gain: f32 = 0.5;
        assert!(comp.update_parameters(&new_gain));
        assert_eq!(comp.gain, 0.5);
    }
}
