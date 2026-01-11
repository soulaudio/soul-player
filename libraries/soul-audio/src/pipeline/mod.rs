//! Audio Pipeline Abstractions
//!
//! This module provides a trait-based architecture for audio pipeline components.
//! All components in the audio pipeline derive from `PipelineComponent` and can be
//! managed through the effect registry factory pattern.
//!
//! # Architecture
//!
//! ```text
//! AudioSource -> [PipelineComponent]* -> Output
//!                      |
//!                      v
//!              EffectRegistry (factory)
//! ```

mod component;
mod effect_impls;
mod loudness_impls;
mod registry;
mod state;

pub use component::{PipelineComponent, PipelineComponentInfo};
pub use effect_impls::{CrossfeedUpdateParams, GraphicEqUpdateParams};
pub use loudness_impls::HeadroomParams;
pub use registry::{ConvolutionParams, EffectFactory, EffectRegistry, EffectTypeId, GraphicEqParams};
pub use state::{
    CrossfadeProgress, PipelineEvent, PipelineState, PipelineStateMachine, TrackTransition,
};

// Re-export headroom types from soul-loudness for convenience
pub use soul_loudness::headroom::{HeadroomManager, HeadroomMode};
