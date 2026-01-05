/// Shared application state
use crate::services::{AuthService, FileStorage};
use soul_storage::Database;
use std::sync::Arc;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub auth_service: Arc<AuthService>,
    pub file_storage: Arc<FileStorage>,
}

impl AppState {
    pub fn new(
        db: Arc<Database>,
        auth_service: Arc<AuthService>,
        file_storage: Arc<FileStorage>,
    ) -> Self {
        Self {
            db,
            auth_service,
            file_storage,
        }
    }
}
