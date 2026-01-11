//! PipelineComponent implementations for all effect types
//!
//! This module provides the bridge between the effects module and the pipeline abstraction.

use super::component::{PipelineComponent, PipelineComponentInfo};
use crate::effects::{
    AudioEffect, Compressor, CompressorSettings, ConvolutionEngine, Crossfeed, CrossfeedPreset,
    CrossfeedSettings, EqBand, GraphicEq, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use std::any::Any;

// ===== ParametricEq =====

impl PipelineComponent for ParametricEq {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "parametric_eq",
            display_name: "Parametric EQ",
            description: "Multi-band parametric equalizer",
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
        if let Some(bands) = params.downcast_ref::<Vec<EqBand>>() {
            self.set_bands(bands.clone());
            true
        } else {
            false
        }
    }
}

// ===== GraphicEq =====

/// Parameters for GraphicEq updates
#[derive(Debug, Clone)]
pub struct GraphicEqUpdateParams {
    pub gains: Vec<f32>,
}

impl PipelineComponent for GraphicEq {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "graphic_eq",
            display_name: "Graphic EQ",
            description: "10 or 31-band graphic equalizer",
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
        if let Some(update) = params.downcast_ref::<GraphicEqUpdateParams>() {
            for (i, &gain) in update.gains.iter().enumerate() {
                self.set_band_gain(i, gain);
            }
            true
        } else if let Some(gains) = params.downcast_ref::<Vec<f32>>() {
            for (i, &gain) in gains.iter().enumerate() {
                self.set_band_gain(i, gain);
            }
            true
        } else {
            false
        }
    }
}

// ===== Compressor =====

impl PipelineComponent for Compressor {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "compressor",
            display_name: "Compressor",
            description: "Dynamic range compressor",
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
        if let Some(settings) = params.downcast_ref::<CompressorSettings>() {
            self.set_settings(settings.clone());
            true
        } else {
            false
        }
    }
}

// ===== Limiter =====

impl PipelineComponent for Limiter {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "limiter",
            display_name: "Limiter",
            description: "Brick-wall limiter",
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
        if let Some(settings) = params.downcast_ref::<LimiterSettings>() {
            self.set_threshold(settings.threshold_db);
            self.set_release(settings.release_ms);
            true
        } else {
            false
        }
    }
}

// ===== StereoEnhancer =====

impl PipelineComponent for StereoEnhancer {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "stereo_enhancer",
            display_name: "Stereo Enhancer",
            description: "Width control, mid/side processing, balance",
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
        if let Some(settings) = params.downcast_ref::<StereoSettings>() {
            self.set_width(settings.width);
            self.set_mid_gain_db(settings.mid_gain_db);
            self.set_side_gain_db(settings.side_gain_db);
            self.set_balance(settings.balance);
            true
        } else {
            false
        }
    }
}

// ===== Crossfeed =====

/// Parameters for Crossfeed updates (can be settings or preset)
#[derive(Debug, Clone)]
pub enum CrossfeedUpdateParams {
    Settings(CrossfeedSettings),
    Preset(CrossfeedPreset),
}

impl PipelineComponent for Crossfeed {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "crossfeed",
            display_name: "Crossfeed",
            description: "Bauer stereophonic-to-binaural DSP for headphones",
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
        if let Some(update) = params.downcast_ref::<CrossfeedUpdateParams>() {
            match update {
                CrossfeedUpdateParams::Settings(settings) => {
                    self.set_level_db(settings.level_db);
                    self.set_cutoff_hz(settings.cutoff_hz);
                }
                CrossfeedUpdateParams::Preset(preset) => {
                    self.set_preset(*preset);
                }
            }
            true
        } else if let Some(settings) = params.downcast_ref::<CrossfeedSettings>() {
            self.set_level_db(settings.level_db);
            self.set_cutoff_hz(settings.cutoff_hz);
            true
        } else if let Some(preset) = params.downcast_ref::<CrossfeedPreset>() {
            self.set_preset(*preset);
            true
        } else {
            false
        }
    }
}

// ===== ConvolutionEngine =====

impl PipelineComponent for ConvolutionEngine {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        AudioEffect::process(self, buffer, sample_rate)
    }

    fn reset(&mut self) {
        AudioEffect::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        AudioEffect::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        AudioEffect::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "convolution",
            display_name: "Convolution Reverb",
            description: "Impulse response based reverb",
            // IR changes require rebuild, only dry/wet can be updated in-place
            supports_in_place_update: false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update_parameters(&mut self, params: &dyn Any) -> bool {
        // Only dry/wet mix can be updated in-place
        if let Some(&mix) = params.downcast_ref::<f32>() {
            self.set_dry_wet_mix(mix);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parametric_eq_as_pipeline_component() {
        let mut eq = ParametricEq::new();
        let comp: &mut dyn PipelineComponent = &mut eq;

        assert_eq!(comp.info().type_id, "parametric_eq");
        assert!(comp.info().supports_in_place_update);

        // Test update
        let bands = vec![EqBand::peaking(1000.0, 3.0, 1.0)];
        assert!(comp.update_parameters(&bands));
    }

    #[test]
    fn test_compressor_as_pipeline_component() {
        let mut comp = Compressor::new();
        let pipeline_comp: &mut dyn PipelineComponent = &mut comp;

        assert_eq!(pipeline_comp.info().type_id, "compressor");

        let settings = CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        };
        assert!(pipeline_comp.update_parameters(&settings));
    }

    #[test]
    fn test_convolution_no_in_place_for_ir() {
        let info = PipelineComponentInfo {
            type_id: "convolution",
            display_name: "Convolution Reverb",
            description: "IR based reverb",
            supports_in_place_update: false,
        };
        assert!(!info.supports_in_place_update);
    }
}
