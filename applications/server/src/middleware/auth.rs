/// Authentication middleware
use crate::{error::ServerError, services::AuthService};
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use soul_core::UserId;
use std::sync::Arc;

/// Extension type to store authenticated user ID in request
/// Can be used as an extractor in handlers
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub UserId);

impl AuthenticatedUser {
    pub fn user_id(&self) -> &UserId {
        &self.0
    }
}

/// Middleware that extracts and validates JWT from Authorization header
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check Bearer prefix
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify token
    let user_id = auth_service
        .verify_access_token(token)
        .map_err(|e| {
            tracing::warn!("Token verification failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // Insert user ID into request extensions
    request.extensions_mut().insert(AuthenticatedUser(user_id));

    Ok(next.run(request).await)
}

/// Implement FromRequestParts so AuthenticatedUser can be used as an extractor
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| ServerError::Unauthorized("Not authenticated".to_string()))
    }
}
