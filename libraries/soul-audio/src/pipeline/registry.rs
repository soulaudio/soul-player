//! Effect Registry - Factory Pattern for Audio Effects
//!
//! Provides a centralized registry for creating and managing audio effects.
//! This eliminates the need for hardcoded switch statements when creating
//! or updating effects.

use super::component::PipelineComponent;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Unique identifier for effect types
pub type EffectTypeId = &'static str;

/// Factory function type for creating effects
pub type CreateFn = Arc<dyn Fn(&dyn Any) -> Option<Box<dyn PipelineComponent>> + Send + Sync>;

/// Factory function type for updating effect parameters in-place
pub type UpdateFn = Arc<dyn Fn(&mut dyn PipelineComponent, &dyn Any) -> bool + Send + Sync>;

/// Factory for a specific effect type
#[derive(Clone)]
pub struct EffectFactory {
    /// Effect type identifier
    pub type_id: EffectTypeId,
    /// Human-readable name
    pub display_name: &'static str,
    /// Create a new instance of this effect from parameters
    pub create: CreateFn,
    /// Update an existing effect's parameters in-place
    pub update: UpdateFn,
    /// Whether this effect supports in-place parameter updates
    pub supports_in_place_update: bool,
}

impl Debug for EffectFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectFactory")
            .field("type_id", &self.type_id)
            .field("display_name", &self.display_name)
            .field("supports_in_place_update", &self.supports_in_place_update)
            .finish()
    }
}

/// Registry of available effect types
///
/// The registry provides a factory pattern for effect creation and management.
/// Effects register themselves with the registry, and the playback system
/// uses the registry to create/update effects without knowing concrete types.
///
/// # Example
///
/// ```ignore
/// let mut registry = EffectRegistry::new();
///
/// // Register an effect type
/// registry.register(EffectFactory {
///     type_id: "parametric_eq",
///     display_name: "Parametric EQ",
///     create: Arc::new(|params| {
///         let settings = params.downcast_ref::<EqSettings>()?;
///         Some(Box::new(ParametricEq::from_settings(settings)))
///     }),
///     update: Arc::new(|effect, params| {
///         effect.update_parameters(params)
///     }),
///     supports_in_place_update: true,
/// });
///
/// // Create an effect
/// let eq = registry.create("parametric_eq", &my_settings);
/// ```
#[derive(Debug, Default)]
pub struct EffectRegistry {
    factories: HashMap<EffectTypeId, EffectFactory>,
}

impl EffectRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Create a registry with all built-in effects registered
    pub fn with_builtin_effects() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_effects();
        registry
    }

    /// Register a new effect factory
    pub fn register(&mut self, factory: EffectFactory) {
        self.factories.insert(factory.type_id, factory);
    }

    /// Get a factory by type ID
    pub fn get_factory(&self, type_id: EffectTypeId) -> Option<&EffectFactory> {
        self.factories.get(type_id)
    }

    /// Create an effect instance
    ///
    /// # Arguments
    /// * `type_id` - The effect type identifier
    /// * `params` - Type-erased parameters for the effect
    ///
    /// # Returns
    /// A boxed effect instance, or None if the type is not registered or params are invalid
    pub fn create(&self, type_id: EffectTypeId, params: &dyn Any) -> Option<Box<dyn PipelineComponent>> {
        let factory = self.factories.get(type_id)?;
        (factory.create)(params)
    }

    /// Update an effect's parameters in-place
    ///
    /// # Arguments
    /// * `type_id` - The effect type identifier
    /// * `effect` - The effect instance to update
    /// * `params` - New parameters
    ///
    /// # Returns
    /// true if update was successful, false otherwise
    pub fn update_in_place(
        &self,
        type_id: EffectTypeId,
        effect: &mut dyn PipelineComponent,
        params: &dyn Any,
    ) -> bool {
        if let Some(factory) = self.factories.get(type_id) {
            if factory.supports_in_place_update {
                return (factory.update)(effect, params);
            }
        }
        false
    }

    /// Check if a type is registered
    pub fn is_registered(&self, type_id: EffectTypeId) -> bool {
        self.factories.contains_key(type_id)
    }

    /// Get all registered effect type IDs
    pub fn registered_types(&self) -> Vec<EffectTypeId> {
        self.factories.keys().copied().collect()
    }

    /// Check if an effect type supports in-place updates
    pub fn supports_in_place_update(&self, type_id: EffectTypeId) -> bool {
        self.factories
            .get(type_id)
            .map(|f| f.supports_in_place_update)
            .unwrap_or(false)
    }

    /// Register all built-in effects
    fn register_builtin_effects(&mut self) {
        use crate::effects::*;
        use super::loudness_impls::HeadroomParams;
        use soul_loudness::headroom::{HeadroomManager, HeadroomMode};

        // Parametric EQ
        self.register(EffectFactory {
            type_id: "parametric_eq",
            display_name: "Parametric EQ",
            create: Arc::new(|params| {
                if let Some(bands) = params.downcast_ref::<Vec<EqBand>>() {
                    let mut eq = ParametricEq::new();
                    eq.set_bands(bands.clone());
                    Some(Box::new(eq))
                } else {
                    Some(Box::new(ParametricEq::new()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(eq) = effect.as_any_mut().downcast_mut::<ParametricEq>() {
                    if let Some(bands) = params.downcast_ref::<Vec<EqBand>>() {
                        eq.set_bands(bands.clone());
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Graphic EQ
        self.register(EffectFactory {
            type_id: "graphic_eq",
            display_name: "Graphic EQ",
            create: Arc::new(|params| {
                if let Some(gains) = params.downcast_ref::<GraphicEqParams>() {
                    let mut eq = match gains.band_count {
                        31 => GraphicEq::new_31_band(),
                        _ => GraphicEq::new_10_band(),
                    };
                    for (i, &gain) in gains.gains.iter().enumerate() {
                        eq.set_band_gain(i, gain);
                    }
                    Some(Box::new(eq))
                } else {
                    Some(Box::new(GraphicEq::new_10_band()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(eq) = effect.as_any_mut().downcast_mut::<GraphicEq>() {
                    if let Some(gains) = params.downcast_ref::<GraphicEqParams>() {
                        for (i, &gain) in gains.gains.iter().enumerate() {
                            eq.set_band_gain(i, gain);
                        }
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Compressor
        self.register(EffectFactory {
            type_id: "compressor",
            display_name: "Compressor",
            create: Arc::new(|params| {
                if let Some(settings) = params.downcast_ref::<CompressorSettings>() {
                    Some(Box::new(Compressor::with_settings(settings.clone())))
                } else {
                    Some(Box::new(Compressor::new()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(comp) = effect.as_any_mut().downcast_mut::<Compressor>() {
                    if let Some(settings) = params.downcast_ref::<CompressorSettings>() {
                        comp.set_settings(settings.clone());
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Limiter
        self.register(EffectFactory {
            type_id: "limiter",
            display_name: "Limiter",
            create: Arc::new(|params| {
                if let Some(settings) = params.downcast_ref::<LimiterSettings>() {
                    Some(Box::new(Limiter::with_settings(settings.clone())))
                } else {
                    Some(Box::new(Limiter::new()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(lim) = effect.as_any_mut().downcast_mut::<Limiter>() {
                    if let Some(settings) = params.downcast_ref::<LimiterSettings>() {
                        lim.set_threshold(settings.threshold_db);
                        lim.set_release(settings.release_ms);
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Stereo Enhancer
        self.register(EffectFactory {
            type_id: "stereo_enhancer",
            display_name: "Stereo Enhancer",
            create: Arc::new(|params| {
                if let Some(settings) = params.downcast_ref::<StereoSettings>() {
                    Some(Box::new(StereoEnhancer::with_settings(settings.clone())))
                } else {
                    Some(Box::new(StereoEnhancer::new()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(stereo) = effect.as_any_mut().downcast_mut::<StereoEnhancer>() {
                    if let Some(settings) = params.downcast_ref::<StereoSettings>() {
                        stereo.set_width(settings.width);
                        stereo.set_mid_gain_db(settings.mid_gain_db);
                        stereo.set_side_gain_db(settings.side_gain_db);
                        stereo.set_balance(settings.balance);
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Crossfeed
        self.register(EffectFactory {
            type_id: "crossfeed",
            display_name: "Crossfeed",
            create: Arc::new(|params| {
                if let Some(settings) = params.downcast_ref::<CrossfeedSettings>() {
                    Some(Box::new(Crossfeed::with_settings(settings.clone())))
                } else if let Some(preset) = params.downcast_ref::<CrossfeedPreset>() {
                    Some(Box::new(Crossfeed::with_preset(*preset)))
                } else {
                    Some(Box::new(Crossfeed::new()))
                }
            }),
            update: Arc::new(|effect, params| {
                if let Some(cf) = effect.as_any_mut().downcast_mut::<Crossfeed>() {
                    if let Some(settings) = params.downcast_ref::<CrossfeedSettings>() {
                        cf.set_level_db(settings.level_db);
                        cf.set_cutoff_hz(settings.cutoff_hz);
                        return true;
                    } else if let Some(preset) = params.downcast_ref::<CrossfeedPreset>() {
                        cf.set_preset(*preset);
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });

        // Convolution
        self.register(EffectFactory {
            type_id: "convolution",
            display_name: "Convolution Reverb",
            create: Arc::new(|params| {
                if let Some(settings) = params.downcast_ref::<ConvolutionParams>() {
                    let mut engine = ConvolutionEngine::new();
                    match engine.load_from_wav(&settings.ir_path) {
                        Ok(()) => {
                            engine.set_dry_wet_mix(settings.dry_wet_mix);
                            Some(Box::new(engine))
                        }
                        Err(_) => None,
                    }
                } else {
                    None // Convolution requires an IR path
                }
            }),
            update: Arc::new(|effect, params| {
                // Convolution only supports dry/wet update, not IR reload
                if let Some(conv) = effect.as_any_mut().downcast_mut::<ConvolutionEngine>() {
                    if let Some(mix) = params.downcast_ref::<f32>() {
                        conv.set_dry_wet_mix(*mix);
                        return true;
                    }
                }
                false
            }),
            // IR changes require rebuild
            supports_in_place_update: false,
        });

        // Headroom Manager
        self.register(EffectFactory {
            type_id: "headroom_manager",
            display_name: "Headroom Manager",
            create: Arc::new(|params| {
                let mut manager = HeadroomManager::new();
                if let Some(p) = params.downcast_ref::<HeadroomParams>() {
                    manager.set_mode(p.mode);
                    if let Some(rg) = p.replaygain_db {
                        manager.set_replaygain_db(rg);
                    }
                    if let Some(preamp) = p.preamp_db {
                        manager.set_preamp_db(preamp);
                    }
                    if let Some(eq) = p.eq_max_boost_db {
                        manager.set_eq_max_boost_db(eq);
                    }
                    if let Some(additional) = p.additional_gain_db {
                        manager.set_additional_gain_db(additional);
                    }
                } else if let Some(mode) = params.downcast_ref::<HeadroomMode>() {
                    manager.set_mode(*mode);
                }
                Some(Box::new(manager))
            }),
            update: Arc::new(|effect, params| {
                if let Some(manager) = effect.as_any_mut().downcast_mut::<HeadroomManager>() {
                    if let Some(p) = params.downcast_ref::<HeadroomParams>() {
                        manager.set_mode(p.mode);
                        if let Some(rg) = p.replaygain_db {
                            manager.set_replaygain_db(rg);
                        }
                        if let Some(preamp) = p.preamp_db {
                            manager.set_preamp_db(preamp);
                        }
                        if let Some(eq) = p.eq_max_boost_db {
                            manager.set_eq_max_boost_db(eq);
                        }
                        if let Some(additional) = p.additional_gain_db {
                            manager.set_additional_gain_db(additional);
                        }
                        return true;
                    } else if let Some(mode) = params.downcast_ref::<HeadroomMode>() {
                        manager.set_mode(*mode);
                        return true;
                    }
                }
                false
            }),
            supports_in_place_update: true,
        });
    }
}

/// Parameters for Graphic EQ creation/updates
#[derive(Debug, Clone)]
pub struct GraphicEqParams {
    /// Number of bands (10 or 31)
    pub band_count: usize,
    /// Gain values for each band
    pub gains: Vec<f32>,
}

/// Parameters for Convolution engine
#[derive(Debug, Clone)]
pub struct ConvolutionParams {
    /// Path to impulse response WAV file
    pub ir_path: String,
    /// Dry/wet mix (0.0 = dry, 1.0 = wet)
    pub dry_wet_mix: f32,
}

impl Clone for EffectRegistry {
    fn clone(&self) -> Self {
        Self {
            factories: self.factories.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{CompressorSettings, EqBand, LimiterSettings, StereoSettings};
    use soul_loudness::headroom::HeadroomMode;

    #[test]
    fn test_registry_builtin_effects() {
        let registry = EffectRegistry::with_builtin_effects();

        assert!(registry.is_registered("parametric_eq"));
        assert!(registry.is_registered("graphic_eq"));
        assert!(registry.is_registered("compressor"));
        assert!(registry.is_registered("limiter"));
        assert!(registry.is_registered("stereo_enhancer"));
        assert!(registry.is_registered("crossfeed"));
        assert!(registry.is_registered("convolution"));
        assert!(registry.is_registered("headroom_manager"));
    }

    #[test]
    fn test_create_parametric_eq() {
        let registry = EffectRegistry::with_builtin_effects();

        let bands = vec![EqBand::peaking(1000.0, 3.0, 1.0)];
        let effect = registry.create("parametric_eq", &bands);

        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert_eq!(effect.info().type_id, "parametric_eq");
    }

    #[test]
    fn test_create_compressor() {
        let registry = EffectRegistry::with_builtin_effects();

        let settings = CompressorSettings::default();
        let effect = registry.create("compressor", &settings);

        assert!(effect.is_some());
    }

    #[test]
    fn test_update_in_place() {
        let registry = EffectRegistry::with_builtin_effects();

        // Create limiter
        let settings = LimiterSettings::default();
        let mut effect = registry.create("limiter", &settings).unwrap();

        // Update in place
        let new_settings = LimiterSettings {
            threshold_db: -3.0,
            release_ms: 100.0,
        };
        let updated = registry.update_in_place("limiter", effect.as_mut(), &new_settings);

        assert!(updated);
    }

    #[test]
    fn test_convolution_no_in_place() {
        let registry = EffectRegistry::with_builtin_effects();

        // Convolution should not support in-place updates for IR changes
        assert!(!registry.supports_in_place_update("convolution"));
    }

    #[test]
    fn test_registered_types() {
        let registry = EffectRegistry::with_builtin_effects();
        let types = registry.registered_types();

        assert!(types.len() >= 8);
    }

    #[test]
    fn test_create_headroom_manager() {
        let registry = EffectRegistry::with_builtin_effects();

        // Create with default
        let effect = registry.create("headroom_manager", &());
        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert_eq!(effect.info().type_id, "headroom_manager");

        // Create with mode
        let mode = HeadroomMode::Manual(-6.0);
        let effect = registry.create("headroom_manager", &mode);
        assert!(effect.is_some());
    }

    #[test]
    fn test_headroom_manager_update_in_place() {
        use crate::pipeline::loudness_impls::HeadroomParams;

        let registry = EffectRegistry::with_builtin_effects();

        // Create headroom manager
        let mut effect = registry.create("headroom_manager", &()).unwrap();

        // Update with HeadroomParams
        let params = HeadroomParams::auto_with_gains(5.0, 3.0, 6.0);
        let updated = registry.update_in_place("headroom_manager", effect.as_mut(), &params);
        assert!(updated);

        // Update with just mode
        let mode = HeadroomMode::Manual(-3.0);
        let updated = registry.update_in_place("headroom_manager", effect.as_mut(), &mode);
        assert!(updated);
    }
}
