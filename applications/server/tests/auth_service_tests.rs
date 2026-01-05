/// Authentication service tests
/// Tests JWT generation, password hashing, token validation
mod common;

use common::{create_test_database, fixtures};
use soul_core::{Storage, UserId};
use soul_server::services::auth::AuthService;
use soul_storage::Database;
use std::sync::Arc;

/// Test password hashing produces valid bcrypt hashes
#[tokio::test]
async fn test_password_hashing() {
    let auth_service = create_test_auth_service();

    let password = "MySecurePassword123!";
    let hash = auth_service.hash_password(password).unwrap();

    // Verify hash format (bcrypt starts with $2b$ or $2a$)
    assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$"));
    assert!(hash.len() > 50); // bcrypt hashes are typically 60 characters

    // Verify the hash is different each time (salt is random)
    let hash2 = auth_service.hash_password(password).unwrap();
    assert_ne!(hash, hash2, "Hashes should differ due to random salt");
}

/// Test password verification with correct password
#[tokio::test]
async fn test_password_verification_success() {
    let auth_service = create_test_auth_service();

    let password = "MySecurePassword123!";
    let hash = auth_service.hash_password(password).unwrap();

    // Correct password should verify
    let result = auth_service.verify_password(password, &hash).unwrap();
    assert!(result, "Correct password should verify successfully");
}

/// Test password verification with incorrect password
#[tokio::test]
async fn test_password_verification_failure() {
    let auth_service = create_test_auth_service();

    let password = "MySecurePassword123!";
    let hash = auth_service.hash_password(password).unwrap();

    // Wrong password should not verify
    let result = auth_service.verify_password("WrongPassword", &hash).unwrap();
    assert!(!result, "Incorrect password should not verify");
}

/// Test password verification with invalid hash format
#[tokio::test]
async fn test_password_verification_invalid_hash() {
    let auth_service = create_test_auth_service();

    let result = auth_service.verify_password("password", "not-a-valid-hash");
    assert!(result.is_err(), "Invalid hash should return error");
}

/// Test JWT access token generation and validation
#[tokio::test]
async fn test_access_token_generation_and_validation() {
    let auth_service = create_test_auth_service();
    let user_id = UserId::new("user123".to_string());

    // Generate token
    let token = auth_service.create_access_token(&user_id).unwrap();
    assert!(!token.is_empty(), "Token should not be empty");

    // Validate token
    let decoded_user_id = auth_service.verify_access_token(&token).unwrap();
    assert_eq!(user_id, decoded_user_id, "Decoded user ID should match original");
}

/// Test JWT refresh token generation and validation
#[tokio::test]
async fn test_refresh_token_generation_and_validation() {
    let auth_service = create_test_auth_service();
    let user_id = UserId::new("user123".to_string());

    // Generate token
    let token = auth_service.create_refresh_token(&user_id).unwrap();
    assert!(!token.is_empty(), "Token should not be empty");

    // Validate token
    let decoded_user_id = auth_service.verify_refresh_token(&token).unwrap();
    assert_eq!(user_id, decoded_user_id, "Decoded user ID should match original");
}

/// Test that access token cannot be used as refresh token
#[tokio::test]
async fn test_token_type_enforcement_access_as_refresh() {
    let auth_service = create_test_auth_service();
    let user_id = UserId::new("user123".to_string());

    let access_token = auth_service.create_access_token(&user_id).unwrap();

    // Attempting to verify access token as refresh token should fail
    let result = auth_service.verify_refresh_token(&access_token);
    assert!(result.is_err(), "Access token should not validate as refresh token");
}

/// Test that refresh token cannot be used as access token
#[tokio::test]
async fn test_token_type_enforcement_refresh_as_access() {
    let auth_service = create_test_auth_service();
    let user_id = UserId::new("user123".to_string());

    let refresh_token = auth_service.create_refresh_token(&user_id).unwrap();

    // Attempting to verify refresh token as access token should fail
    let result = auth_service.verify_access_token(&refresh_token);
    assert!(result.is_err(), "Refresh token should not validate as access token");
}

/// Test token validation with invalid signature
#[tokio::test]
async fn test_token_validation_invalid_signature() {
    let auth_service = create_test_auth_service();

    // Create a token with different secret
    let other_auth = AuthService::new("different-secret".to_string(), 1, 1); // 1 hour access, 1 day refresh
    let user_id = UserId::new("user123".to_string());
    let token = other_auth.create_access_token(&user_id).unwrap();

    // Should fail validation with different secret
    let result = auth_service.verify_access_token(&token);
    assert!(result.is_err(), "Token with wrong signature should fail validation");
}

/// Test token validation with malformed token
#[tokio::test]
async fn test_token_validation_malformed() {
    let auth_service = create_test_auth_service();

    let result = auth_service.verify_access_token("not.a.valid.jwt.token");
    assert!(result.is_err(), "Malformed token should fail validation");
}

/// Test token validation with empty token
#[tokio::test]
async fn test_token_validation_empty() {
    let auth_service = create_test_auth_service();

    let result = auth_service.verify_access_token("");
    assert!(result.is_err(), "Empty token should fail validation");
}

/// Test token expiration (access token)
/// NOTE: This test is ignored because AuthService currently only supports hour-granularity expiration
/// Testing expiration would require waiting 1+ hours, which is impractical for unit tests
/// TODO: Refactor AuthService to support second/millisecond expiration for testing
#[tokio::test]
#[ignore]
async fn test_access_token_expiration() {
    // This test is skipped because it would require waiting over an hour
    // In production, tokens are validated properly with hour-based expiration
    let auth_service = AuthService::new(
        "test-secret".to_string(),
        1,  // 1 hour minimum
        1,  // 1 day
    );

    let user_id = UserId::new("user123".to_string());
    let token = auth_service.create_access_token(&user_id).unwrap();

    // Token should be valid immediately
    assert!(auth_service.verify_access_token(&token).is_ok());

    // Would need to wait 1+ hours for token to expire
    // tokio::time::sleep(Duration::from_secs(3700)).await;

    // Token should now be expired (if we waited)
    // let result = auth_service.verify_access_token(&token);
    // assert!(result.is_err(), "Expired token should fail validation");
}

/// Test complete authentication flow with database
#[tokio::test]
async fn test_complete_authentication_flow() {
    let db = create_test_database().await.unwrap();
    let auth_service = create_test_auth_service();

    // Create user
    let user = db.create_user(fixtures::TEST_USERNAME).await.unwrap();

    // Hash and store password
    let password_hash = auth_service.hash_password(fixtures::TEST_PASSWORD).unwrap();
    store_user_credentials(&db, &user.id, &password_hash).await.unwrap();

    // Simulate login: retrieve hash and verify password
    let stored_hash = get_user_password_hash(&db, &user.id).await.unwrap();
    let password_valid = auth_service.verify_password(fixtures::TEST_PASSWORD, &stored_hash).unwrap();
    assert!(password_valid, "Password should be valid");

    // Generate tokens
    let access_token = auth_service.create_access_token(&user.id).unwrap();
    let refresh_token = auth_service.create_refresh_token(&user.id).unwrap();

    // Validate access token
    let decoded_id = auth_service.verify_access_token(&access_token).unwrap();
    assert_eq!(user.id, decoded_id);

    // Validate refresh token
    let decoded_id = auth_service.verify_refresh_token(&refresh_token).unwrap();
    assert_eq!(user.id, decoded_id);
}

/// Test authentication with wrong password
#[tokio::test]
async fn test_authentication_wrong_password() {
    let db = create_test_database().await.unwrap();
    let auth_service = create_test_auth_service();

    // Create user
    let user = db.create_user(fixtures::TEST_USERNAME).await.unwrap();

    // Hash and store password
    let password_hash = auth_service.hash_password(fixtures::TEST_PASSWORD).unwrap();
    store_user_credentials(&db, &user.id, &password_hash).await.unwrap();

    // Try to authenticate with wrong password
    let stored_hash = get_user_password_hash(&db, &user.id).await.unwrap();
    let password_valid = auth_service.verify_password("WrongPassword", &stored_hash).unwrap();
    assert!(!password_valid, "Wrong password should not be valid");
}

/// Test authentication with non-existent user
#[tokio::test]
async fn test_authentication_nonexistent_user() {
    let db = create_test_database().await.unwrap();
    let fake_user_id = UserId::new("nonexistent".to_string());

    let result = get_user_password_hash(&db, &fake_user_id).await;
    assert!(result.is_err(), "Should error for non-existent user");
}

/// Test multiple users with different passwords
#[tokio::test]
async fn test_multiple_users_authentication() {
    let db = create_test_database().await.unwrap();
    let auth_service = create_test_auth_service();

    // Create first user
    let user1 = db.create_user("user1").await.unwrap();
    let password1 = "Password1!";
    let hash1 = auth_service.hash_password(password1).unwrap();
    store_user_credentials(&db, &user1.id, &hash1).await.unwrap();

    // Create second user
    let user2 = db.create_user("user2").await.unwrap();
    let password2 = "Password2!";
    let hash2 = auth_service.hash_password(password2).unwrap();
    store_user_credentials(&db, &user2.id, &hash2).await.unwrap();

    // Verify user1 can authenticate with password1
    let hash = get_user_password_hash(&db, &user1.id).await.unwrap();
    assert!(auth_service.verify_password(password1, &hash).unwrap());
    assert!(!auth_service.verify_password(password2, &hash).unwrap());

    // Verify user2 can authenticate with password2
    let hash = get_user_password_hash(&db, &user2.id).await.unwrap();
    assert!(auth_service.verify_password(password2, &hash).unwrap());
    assert!(!auth_service.verify_password(password1, &hash).unwrap());
}

/// Test password update flow
#[tokio::test]
async fn test_password_update() {
    let db = create_test_database().await.unwrap();
    let auth_service = create_test_auth_service();

    // Create user with initial password
    let user = db.create_user(fixtures::TEST_USERNAME).await.unwrap();
    let old_password = "OldPassword123!";
    let old_hash = auth_service.hash_password(old_password).unwrap();
    store_user_credentials(&db, &user.id, &old_hash).await.unwrap();

    // Verify old password works
    let hash = get_user_password_hash(&db, &user.id).await.unwrap();
    assert!(auth_service.verify_password(old_password, &hash).unwrap());

    // Update password
    let new_password = "NewPassword456!";
    let new_hash = auth_service.hash_password(new_password).unwrap();
    update_user_credentials(&db, &user.id, &new_hash).await.unwrap();

    // Verify old password no longer works
    let hash = get_user_password_hash(&db, &user.id).await.unwrap();
    assert!(!auth_service.verify_password(old_password, &hash).unwrap());

    // Verify new password works
    assert!(auth_service.verify_password(new_password, &hash).unwrap());
}

// Helper functions

fn create_test_auth_service() -> AuthService {
    AuthService::new(
        "test-secret-key-for-testing".to_string(),
        1,    // 1 hour access token
        1,    // 1 day refresh token
    )
}

async fn store_user_credentials(
    db: &Arc<Database>,
    user_id: &UserId,
    password_hash: &str,
) -> Result<(), soul_storage::StorageError> {
    use sqlx::Row;
    let pool = db.pool();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_credentials (user_id, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id.as_str())
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_user_password_hash(
    db: &Arc<Database>,
    user_id: &UserId,
) -> Result<String, soul_storage::StorageError> {
    use sqlx::Row;
    let pool = db.pool();

    let row = sqlx::query(
        "SELECT password_hash FROM user_credentials WHERE user_id = ?"
    )
    .bind(user_id.as_str())
    .fetch_one(pool)
    .await?;

    Ok(row.get("password_hash"))
}

async fn update_user_credentials(
    db: &Arc<Database>,
    user_id: &UserId,
    password_hash: &str,
) -> Result<(), soul_storage::StorageError> {
    use sqlx::Row;
    let pool = db.pool();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "UPDATE user_credentials SET password_hash = ?, updated_at = ? WHERE user_id = ?"
    )
    .bind(password_hash)
    .bind(now)
    .bind(user_id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}
