/// Authentication service - JWT and password handling
use crate::error::{Result, ServerError};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use soul_core::UserId;

#[derive(Debug, Clone)]
pub struct AuthService {
    secret: String,
    access_token_expiration: Duration,
    refresh_token_expiration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub exp: i64,    // Expiration time
    pub iat: i64,    // Issued at
    pub token_type: TokenType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl AuthService {
    pub fn new(secret: String, access_expiration_hours: u64, refresh_expiration_days: u64) -> Self {
        Self {
            secret,
            access_token_expiration: Duration::hours(access_expiration_hours as i64),
            refresh_token_expiration: Duration::days(refresh_expiration_days as i64),
        }
    }

    /// Hash a password using bcrypt
    pub fn hash_password(&self, password: &str) -> Result<String> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(ServerError::from)
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        bcrypt::verify(password, hash).map_err(ServerError::from)
    }

    /// Create an access token
    pub fn create_access_token(&self, user_id: &UserId) -> Result<String> {
        self.create_token(user_id, TokenType::Access, self.access_token_expiration)
    }

    /// Create a refresh token
    pub fn create_refresh_token(&self, user_id: &UserId) -> Result<String> {
        self.create_token(user_id, TokenType::Refresh, self.refresh_token_expiration)
    }

    /// Verify and decode a token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::default();

        let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Verify that a token is an access token
    pub fn verify_access_token(&self, token: &str) -> Result<UserId> {
        let claims = self.verify_token(token)?;
        if claims.token_type != TokenType::Access {
            return Err(ServerError::Auth("Invalid token type".to_string()));
        }
        Ok(UserId::new(claims.sub))
    }

    /// Verify that a token is a refresh token
    pub fn verify_refresh_token(&self, token: &str) -> Result<UserId> {
        let claims = self.verify_token(token)?;
        if claims.token_type != TokenType::Refresh {
            return Err(ServerError::Auth("Invalid token type".to_string()));
        }
        Ok(UserId::new(claims.sub))
    }

    fn create_token(
        &self,
        user_id: &UserId,
        token_type: TokenType,
        expiration: Duration,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + expiration;

        let claims = Claims {
            sub: user_id.as_str().to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type,
        };

        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());
        encode(&Header::default(), &claims, &encoding_key).map_err(ServerError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let auth = AuthService::new("secret".to_string(), 24, 30);
        let password = "my_secure_password";

        let hash = auth.hash_password(password).unwrap();
        assert!(auth.verify_password(password, &hash).unwrap());
        assert!(!auth.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_creation_and_verification() {
        let auth = AuthService::new("secret".to_string(), 24, 30);
        let user_id = UserId::new("user-123");

        let access_token = auth.create_access_token(&user_id).unwrap();
        let verified_id = auth.verify_access_token(&access_token).unwrap();
        assert_eq!(verified_id, user_id);

        let refresh_token = auth.create_refresh_token(&user_id).unwrap();
        let verified_id = auth.verify_refresh_token(&refresh_token).unwrap();
        assert_eq!(verified_id, user_id);
    }

    #[test]
    fn test_token_type_validation() {
        let auth = AuthService::new("secret".to_string(), 24, 30);
        let user_id = UserId::new("user-123");

        let access_token = auth.create_access_token(&user_id).unwrap();
        assert!(auth.verify_refresh_token(&access_token).is_err());

        let refresh_token = auth.create_refresh_token(&user_id).unwrap();
        assert!(auth.verify_access_token(&refresh_token).is_err());
    }
}
