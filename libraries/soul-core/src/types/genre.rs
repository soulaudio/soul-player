//! Genre types

use serde::{Deserialize, Serialize};

pub type GenreId = i64;

/// A music genre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: GenreId,
    pub name: String,
    pub canonical_name: String,
    pub created_at: String,
}

/// Data for creating a new genre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGenre {
    pub name: String,
    pub canonical_name: String,
}
