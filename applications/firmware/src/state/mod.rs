//! Shared application state
//!
//! This module defines global state and inter-task communication channels.
//! Uses Embassy synchronization primitives (Channels, Signals, Mutexes).
//!
//! Communication patterns:
//! - Embassy Channels: Multi-producer, multi-consumer queues
//! - Embassy Signals: Single-value notifications
//! - Embassy Mutexes: Critical-section based locks

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    signal::Signal,
};

// Example channel definitions (to be uncommented as needed):
//
// /// Commands sent to the audio task (play, pause, seek, etc.)
// pub static AUDIO_CMD_CHANNEL: Channel<CriticalSectionRawMutex, AudioCommand, 4> =
//     Channel::new();
//
// /// UI update events (track changed, volume changed, etc.)
// pub static UI_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, UiEvent, 16> =
//     Channel::new();
//
// /// Input events from buttons/encoder
// pub static INPUT_EVENT_CHANNEL: Channel<CriticalSectionRawMutex, InputEvent, 16> =
//     Channel::new();
//
// /// Power status notifications (battery level, charging state)
// pub static POWER_STATUS_SIGNAL: Signal<CriticalSectionRawMutex, PowerStatus> =
//     Signal::new();

// App state module (uncomment when implementing):
// pub mod app_state;
