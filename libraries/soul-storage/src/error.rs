/// Storage-specific errors
use thiserror::Error;

/// Result type alias using `StorageError`
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, StorageError>;

/// Storage error types
#[derive(Error, Debug)]
pub enum StorageError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// Entity not found
    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Database error from `SQLx`
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl StorageError {
    /// Create a not found error
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }
}

impl From<StorageError> for soul_core::SoulError {
    fn from(err: StorageError) -> Self {
        soul_core::SoulError::storage(err.to_string())
    }
}
