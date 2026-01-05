/// User domain type
use serde::{Deserialize, Serialize};

/// User account
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: i64,

    /// Display name
    pub name: String,

    /// Account creation timestamp (ISO string)
    pub created_at: String,
}
