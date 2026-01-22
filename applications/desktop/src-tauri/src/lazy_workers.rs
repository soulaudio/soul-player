//! Lazy initialization wrappers for background workers
//!
//! These wrappers defer worker initialization until first use, reducing startup time.
//! Workers are initialized once on first access and reused for all subsequent calls.

use crate::fingerprint::FingerprintWorker;
use crate::import::ImportManager;
use crate::loudness::AnalysisWorker;
use crate::sync::SyncState;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};

/// Lazy wrapper for AnalysisWorker
///
/// Initialization is deferred until first use via `get()`.
pub struct LazyAnalysisWorker {
    inner: std::sync::OnceLock<Arc<Mutex<AnalysisWorker>>>,
}

impl LazyAnalysisWorker {
    pub const fn new() -> Self {
        Self {
            inner: std::sync::OnceLock::new(),
        }
    }

    pub fn get(&self) -> &Arc<Mutex<AnalysisWorker>> {
        self.inner.get_or_init(|| {
            tracing::info!("[LazyAnalysisWorker] First use - initializing loudness analyzer");
            Arc::new(Mutex::new(AnalysisWorker::new()))
        })
    }
}

/// Lazy wrapper for ImportManager
///
/// Requires async initialization due to database queries.
pub struct LazyImportManager {
    inner: Arc<OnceCell<ImportManager>>,
    pool: sqlx::SqlitePool,
    user_id: String,
    library_path: std::path::PathBuf,
}

impl LazyImportManager {
    pub fn new(pool: sqlx::SqlitePool, user_id: String, library_path: std::path::PathBuf) -> Self {
        Self {
            inner: Arc::new(OnceCell::new()),
            pool,
            user_id,
            library_path,
        }
    }

    pub async fn get(&self) -> Result<&ImportManager, String> {
        self.inner
            .get_or_try_init(|| async {
                tracing::info!("[LazyImportManager] First use - initializing import system");
                ImportManager::new(
                    self.pool.clone(),
                    self.user_id.clone(),
                    self.library_path.clone(),
                )
                .await
            })
            .await
    }
}

/// Lazy wrapper for SyncState
///
/// Initialization is lightweight (just wraps SyncManager).
#[derive(Clone)]
pub struct LazySyncState {
    inner: Arc<std::sync::OnceLock<Arc<Mutex<SyncState>>>>,
    pool: sqlx::SqlitePool,
}

impl LazySyncState {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            inner: Arc::new(std::sync::OnceLock::new()),
            pool,
        }
    }

    pub fn get(&self) -> &Arc<Mutex<SyncState>> {
        self.inner.get_or_init(|| {
            tracing::info!("[LazySyncState] First use - initializing sync system");
            Arc::new(Mutex::new(SyncState::new(self.pool.clone())))
        })
    }

    /// Get mutable reference for auto-sync check during startup
    pub async fn get_for_startup_check(&self) -> Result<Arc<Mutex<SyncState>>, String> {
        let state = self.get().clone();
        Ok(state)
    }
}

/// Lazy wrapper for FingerprintWorker
///
/// Initialization is lightweight (just creates worker struct).
pub struct LazyFingerprintWorker {
    inner: std::sync::OnceLock<Arc<FingerprintWorker>>,
}

impl LazyFingerprintWorker {
    pub const fn new() -> Self {
        Self {
            inner: std::sync::OnceLock::new(),
        }
    }

    pub fn get(&self) -> &Arc<FingerprintWorker> {
        self.inner.get_or_init(|| {
            tracing::info!("[LazyFingerprintWorker] First use - initializing fingerprint worker");
            Arc::new(FingerprintWorker::new())
        })
    }
}
