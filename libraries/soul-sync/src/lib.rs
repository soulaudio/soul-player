mod cleaner;
mod error;
mod manager;
mod metadata;
mod scanner;
mod state;
mod types;
mod validator;

// Public exports
pub use error::{Result, SyncError};
pub use manager::SyncManager;
pub use types::{SyncPhase, SyncProgress, SyncStatus, SyncSummary, SyncTrigger};
