use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct InitProgress {
    pub step: String,
    pub progress: u8,
}

/// Emit initialization progress to the splash screen
pub async fn emit_init_progress(app: &AppHandle, step: &str, progress: u8) {
    if let Err(e) = app.emit(
        "init-progress",
        InitProgress {
            step: step.to_string(),
            progress,
        },
    ) {
        tracing::warn!(error = %e, event = "init-progress", step = %step, progress = %progress, "Failed to emit event to frontend");
    }
}
