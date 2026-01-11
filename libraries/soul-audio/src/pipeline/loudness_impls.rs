//! PipelineComponent implementations for loudness-related types
//!
//! This module provides the bridge between soul-loudness types and the pipeline abstraction.

use super::component::{PipelineComponent, PipelineComponentInfo};
use soul_loudness::headroom::{HeadroomManager, HeadroomMode};
use std::any::Any;

/// Parameters for HeadroomManager updates
#[derive(Debug, Clone)]
pub struct HeadroomParams {
    /// The headroom mode
    pub mode: HeadroomMode,
    /// ReplayGain value in dB
    pub replaygain_db: Option<f64>,
    /// Pre-amp gain in dB
    pub preamp_db: Option<f64>,
    /// Maximum EQ boost in dB
    pub eq_max_boost_db: Option<f64>,
    /// Additional DSP gain in dB
    pub additional_gain_db: Option<f64>,
}

impl HeadroomParams {
    /// Create params with just the mode
    pub fn with_mode(mode: HeadroomMode) -> Self {
        Self {
            mode,
            replaygain_db: None,
            preamp_db: None,
            eq_max_boost_db: None,
            additional_gain_db: None,
        }
    }

    /// Create params for auto mode with gain values
    pub fn auto_with_gains(
        replaygain_db: f64,
        preamp_db: f64,
        eq_max_boost_db: f64,
    ) -> Self {
        Self {
            mode: HeadroomMode::Auto,
            replaygain_db: Some(replaygain_db),
            preamp_db: Some(preamp_db),
            eq_max_boost_db: Some(eq_max_boost_db),
            additional_gain_db: None,
        }
    }

    /// Create params for manual mode
    pub fn manual(headroom_db: f64) -> Self {
        Self {
            mode: HeadroomMode::Manual(headroom_db),
            replaygain_db: None,
            preamp_db: None,
            eq_max_boost_db: None,
            additional_gain_db: None,
        }
    }

    /// Create params for disabled mode
    pub fn disabled() -> Self {
        Self {
            mode: HeadroomMode::Disabled,
            replaygain_db: None,
            preamp_db: None,
            eq_max_boost_db: None,
            additional_gain_db: None,
        }
    }
}

impl PipelineComponent for HeadroomManager {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        self.process_with_sample_rate(buffer, sample_rate)
    }

    fn reset(&mut self) {
        HeadroomManager::reset(self)
    }

    fn set_enabled(&mut self, enabled: bool) {
        HeadroomManager::set_enabled(self, enabled)
    }

    fn is_enabled(&self) -> bool {
        HeadroomManager::is_enabled(self)
    }

    fn info(&self) -> PipelineComponentInfo {
        PipelineComponentInfo {
            type_id: "headroom_manager",
            display_name: "Headroom Manager",
            description: "Automatic headroom attenuation to prevent clipping",
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
        if let Some(p) = params.downcast_ref::<HeadroomParams>() {
            self.set_mode(p.mode);
            if let Some(rg) = p.replaygain_db {
                self.set_replaygain_db(rg);
            }
            if let Some(preamp) = p.preamp_db {
                self.set_preamp_db(preamp);
            }
            if let Some(eq) = p.eq_max_boost_db {
                self.set_eq_max_boost_db(eq);
            }
            if let Some(additional) = p.additional_gain_db {
                self.set_additional_gain_db(additional);
            }
            true
        } else if let Some(mode) = params.downcast_ref::<HeadroomMode>() {
            self.set_mode(*mode);
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
    fn test_headroom_manager_as_pipeline_component() {
        let mut manager = HeadroomManager::new();
        let comp: &mut dyn PipelineComponent = &mut manager;

        assert_eq!(comp.info().type_id, "headroom_manager");
        assert!(comp.info().supports_in_place_update);
        assert!(comp.is_enabled());
    }

    #[test]
    fn test_headroom_manager_update_params() {
        let mut manager = HeadroomManager::new();
        let comp: &mut dyn PipelineComponent = &mut manager;

        let params = HeadroomParams::auto_with_gains(5.0, 3.0, 6.0);
        assert!(comp.update_parameters(&params));

        // Check values were applied
        let headroom = comp.as_any_mut().downcast_mut::<HeadroomManager>().unwrap();
        assert_eq!(headroom.mode(), HeadroomMode::Auto);
        assert!((headroom.total_potential_gain_db() - 14.0).abs() < 0.001);
    }

    #[test]
    fn test_headroom_manager_update_mode_only() {
        let mut manager = HeadroomManager::new();
        let comp: &mut dyn PipelineComponent = &mut manager;

        let mode = HeadroomMode::Manual(-6.0);
        assert!(comp.update_parameters(&mode));

        let headroom = comp.as_any().downcast_ref::<HeadroomManager>().unwrap();
        assert_eq!(headroom.mode(), HeadroomMode::Manual(-6.0));
    }

    #[test]
    fn test_headroom_manager_process() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        let comp: &mut dyn PipelineComponent = &mut manager;

        let mut buffer = vec![1.0f32; 100];
        comp.process(&mut buffer, 44100);

        // -6 dB = ~0.501 linear
        let expected = 10.0_f32.powf(-6.0 / 20.0);
        for &sample in &buffer {
            assert!((sample - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_headroom_manager_disabled() {
        let mut manager = HeadroomManager::new();
        manager.set_mode(HeadroomMode::Manual(-6.0));

        let comp: &mut dyn PipelineComponent = &mut manager;
        comp.set_enabled(false);

        let mut buffer = vec![1.0f32; 100];
        comp.process(&mut buffer, 44100);

        // Should not process when disabled
        for &sample in &buffer {
            assert!((sample - 1.0).abs() < 0.0001);
        }
    }
}
