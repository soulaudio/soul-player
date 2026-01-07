/// Server error types
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ServerError>;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Authorization failed: {0}")]
    Unauthorized(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] soul_core::SoulError),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Transcoding error: {0}")]
    Transcoding(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Bcrypt error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
}

impl From<soul_storage::StorageError> for ServerError {
    fn from(err: soul_storage::StorageError) -> Self {
        // Convert StorageError -> SoulError -> ServerError
        ServerError::Database(err.into())
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ServerError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            ServerError::Unauthorized(msg) => (StatusCode::FORBIDDEN, msg),
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ServerError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            ServerError::Storage(ref msg) => {
                tracing::error!("Storage error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage error".to_string(),
                )
            }
            ServerError::Config(ref msg) => {
                tracing::error!("Config error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration error".to_string(),
                )
            }
            ServerError::Transcoding(ref msg) => {
                tracing::error!("Transcoding error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Transcoding error".to_string(),
                )
            }
            ServerError::Internal(ref msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ServerError::Io(ref e) => {
                tracing::error!("IO error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "IO error".to_string())
            }
            ServerError::Jwt(ref e) => {
                tracing::error!("JWT error: {:?}", e);
                (StatusCode::UNAUTHORIZED, "Invalid token".to_string())
            }
            ServerError::Bcrypt(ref e) => {
                tracing::error!("Bcrypt error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Password error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
