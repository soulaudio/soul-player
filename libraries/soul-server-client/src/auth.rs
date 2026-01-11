//! Authentication methods for Soul Player Server.

use crate::error::{Result, ServerClientError};
use crate::types::{
    LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, UserInfo,
};
use reqwest::Client;
use tracing::{debug, info, warn};

/// Authentication client for Soul Player Server.
pub struct AuthClient<'a> {
    http: &'a Client,
    base_url: &'a str,
}

impl<'a> AuthClient<'a> {
    pub(crate) fn new(http: &'a Client, base_url: &'a str) -> Self {
        Self { http, base_url }
    }

    /// Login with username and password.
    ///
    /// Returns tokens on success.
    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let url = format!("{}/api/auth/login", self.base_url);
        debug!(url = %url, username = %username, "Attempting login");

        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    ServerClientError::ServerUnreachable(e.to_string())
                } else {
                    ServerClientError::Request(e)
                }
            })?;

        let status = response.status();

        if status.is_success() {
            let login_response: LoginResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse login response: {}", e))
            })?;

            info!(
                username = %login_response.username,
                user_id = %login_response.user_id,
                "Login successful"
            );

            Ok(login_response)
        } else if status.as_u16() == 401 {
            let error_text = response.text().await.unwrap_or_default();
            warn!(status = %status, error = %error_text, "Login failed: invalid credentials");
            Err(ServerClientError::AuthFailed(
                "Invalid username or password".to_string(),
            ))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Refresh an expired access token using the refresh token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<RefreshTokenResponse> {
        let url = format!("{}/api/auth/refresh", self.base_url);
        debug!(url = %url, "Refreshing access token");

        let request = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    ServerClientError::ServerUnreachable(e.to_string())
                } else {
                    ServerClientError::Request(e)
                }
            })?;

        let status = response.status();

        if status.is_success() {
            let refresh_response: RefreshTokenResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse refresh response: {}", e))
            })?;

            debug!("Token refresh successful");
            Ok(refresh_response)
        } else if status.as_u16() == 401 {
            warn!("Token refresh failed: refresh token expired or invalid");
            Err(ServerClientError::TokenRefreshFailed(
                "Refresh token expired or invalid".to_string(),
            ))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Get current user info using an access token.
    pub async fn get_current_user(&self, access_token: &str) -> Result<UserInfo> {
        let url = format!("{}/api/auth/me", self.base_url);
        debug!(url = %url, "Getting current user info");

        let response = self
            .http
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    ServerClientError::ServerUnreachable(e.to_string())
                } else {
                    ServerClientError::Request(e)
                }
            })?;

        let status = response.status();

        if status.is_success() {
            let user_info: UserInfo = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse user info: {}", e))
            })?;

            Ok(user_info)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Validate that an access token is still valid.
    pub async fn validate_token(&self, access_token: &str) -> Result<bool> {
        match self.get_current_user(access_token).await {
            Ok(_) => Ok(true),
            Err(ServerClientError::AuthRequired) => Ok(false),
            Err(ServerClientError::AuthFailed(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mocked HTTP responses
}
