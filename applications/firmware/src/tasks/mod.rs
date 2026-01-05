//! Embassy async tasks
//!
//! Each task runs independently and communicates via Embassy channels/signals.
//! Tasks are spawned from main.rs during initialization.
//!
//! Task priority considerations:
//! - Audio task: Real-time critical (high priority)
//! - Display task: Low frequency updates (low priority)
//! - Input task: Event-driven (medium priority)
//! - Power task: Background monitoring (low priority)
//! - Bluetooth task: Background communication (low priority)

// Task modules will be added as they are implemented:
// pub mod audio_task;
// pub mod display_task;
// pub mod input_task;
// pub mod power_task;
// pub mod bluetooth_task;
