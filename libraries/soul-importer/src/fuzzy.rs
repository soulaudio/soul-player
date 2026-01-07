//! Fuzzy matching with confidence scoring for artists, albums, and genres

use crate::{FuzzyMatch, MatchType, Result};
use soul_core::types::{Album, Artist, ArtistId, CreateAlbum, CreateArtist, Genre};
use sqlx::SqlitePool;
use strsim::normalized_levenshtein;

/// Fuzzy matcher for entity matching with confidence scoring
pub struct FuzzyMatcher {
    /// Confidence threshold for fuzzy matches (default: 60)
    fuzzy_threshold: u8,
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self {
            fuzzy_threshold: 60,
        }
    }
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher with default thresholds
    pub fn new() -> Self {
        Self::default()
    }

    /// Find or create an artist with fuzzy matching
    pub async fn find_or_create_artist(&self, pool: &SqlitePool, name: &str) -> Result<FuzzyMatch<Artist>> {
        let normalized_name = normalize_string(name);

        // Try exact match first (case-sensitive)
        if let Some(artist) = soul_storage::artists::find_by_name(pool, name).await? {
            return Ok(FuzzyMatch {
                entity: artist,
                confidence: 100,
                match_type: MatchType::Exact,
            });
        }

        // Try normalized match (case-insensitive, trimmed)
        let all_artists = soul_storage::artists::get_all(pool).await?;

        for artist in &all_artists {
            let artist_normalized = normalize_string(&artist.name);

            // Check for normalized match
            if artist_normalized == normalized_name {
                return Ok(FuzzyMatch {
                    entity: artist.clone(),
                    confidence: 95,
                    match_type: MatchType::Normalized,
                });
            }
        }

        // Try fuzzy match using Levenshtein distance
        let mut best_match: Option<(Artist, f64)> = None;

        for artist in &all_artists {
            let similarity = normalized_levenshtein(&normalized_name, &normalize_string(&artist.name));

            if similarity >= (self.fuzzy_threshold as f64 / 100.0) {
                if let Some((_, best_similarity)) = &best_match {
                    if similarity > *best_similarity {
                        best_match = Some((artist.clone(), similarity));
                    }
                } else {
                    best_match = Some((artist.clone(), similarity));
                }
            }
        }

        if let Some((artist, similarity)) = best_match {
            let confidence = (similarity * 100.0).round() as u8;
            return Ok(FuzzyMatch {
                entity: artist,
                confidence,
                match_type: MatchType::Fuzzy,
            });
        }

        // No match found - create new artist
        let sort_name = normalize_sort_name(name);
        let new_artist = soul_storage::artists::create(
            pool,
            CreateArtist {
                name: name.to_string(),
                sort_name: Some(sort_name),
                musicbrainz_id: None,
            },
        )
        .await?;

        Ok(FuzzyMatch {
            entity: new_artist,
            confidence: 100, // Always 100 for created entities
            match_type: MatchType::Created,
        })
    }

    /// Find or create an album with fuzzy matching
    pub async fn find_or_create_album(
        &self,
        pool: &SqlitePool,
        title: &str,
        artist_id: Option<ArtistId>,
    ) -> Result<FuzzyMatch<Album>> {
        let normalized_title = normalize_string(title);

        // Get albums by artist (or all if no artist specified)
        let albums = if let Some(aid) = artist_id {
            soul_storage::albums::get_by_artist(pool, aid).await?
        } else {
            soul_storage::albums::get_all(pool).await?
        };

        // Try exact match
        for album in &albums {
            if album.title == title && album.artist_id == artist_id {
                return Ok(FuzzyMatch {
                    entity: album.clone(),
                    confidence: 100,
                    match_type: MatchType::Exact,
                });
            }
        }

        // Try normalized match
        for album in &albums {
            let album_normalized = normalize_string(&album.title);
            if album_normalized == normalized_title && album.artist_id == artist_id {
                return Ok(FuzzyMatch {
                    entity: album.clone(),
                    confidence: 95,
                    match_type: MatchType::Normalized,
                });
            }
        }

        // Try fuzzy match
        let mut best_match: Option<(Album, f64)> = None;

        for album in &albums {
            // Only match albums with same artist
            if album.artist_id != artist_id {
                continue;
            }

            let similarity = normalized_levenshtein(&normalized_title, &normalize_string(&album.title));

            if similarity >= (self.fuzzy_threshold as f64 / 100.0) {
                if let Some((_, best_similarity)) = &best_match {
                    if similarity > *best_similarity {
                        best_match = Some((album.clone(), similarity));
                    }
                } else {
                    best_match = Some((album.clone(), similarity));
                }
            }
        }

        if let Some((album, similarity)) = best_match {
            let confidence = (similarity * 100.0).round() as u8;
            return Ok(FuzzyMatch {
                entity: album,
                confidence,
                match_type: MatchType::Fuzzy,
            });
        }

        // No match found - create new album
        let new_album = soul_storage::albums::create(
            pool,
            CreateAlbum {
                title: title.to_string(),
                artist_id,
                year: None,
                musicbrainz_id: None,
            },
        )
        .await?;

        Ok(FuzzyMatch {
            entity: new_album,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }

    /// Find or create a genre with fuzzy matching and canonicalization
    pub async fn find_or_create_genre(&self, pool: &SqlitePool, name: &str) -> Result<FuzzyMatch<Genre>> {
        let canonical_name = canonicalize_genre_name(name);

        // Try to find by canonical name first (most reliable)
        if let Some(genre) = find_genre_by_canonical(pool, &canonical_name).await? {
            let confidence = if genre.name.to_lowercase() == name.to_lowercase() {
                100
            } else {
                95
            };

            return Ok(FuzzyMatch {
                entity: genre,
                confidence,
                match_type: if confidence == 100 {
                    MatchType::Exact
                } else {
                    MatchType::Normalized
                },
            });
        }

        // Try exact match by name
        if let Some(genre) = find_genre_by_name(pool, name).await? {
            return Ok(FuzzyMatch {
                entity: genre,
                confidence: 100,
                match_type: MatchType::Exact,
            });
        }

        // Create new genre
        let new_genre = create_genre(pool, name, &canonical_name).await?;

        Ok(FuzzyMatch {
            entity: new_genre,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }
}

/// Normalize a string for comparison (lowercase, trim, collapse whitespace)
fn normalize_string(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Normalize artist name for sorting (remove leading articles)
fn normalize_sort_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.starts_with("the ") {
        name[4..].to_string()
    } else if lower.starts_with("a ") {
        name[2..].to_string()
    } else if lower.starts_with("an ") {
        name[3..].to_string()
    } else {
        name.to_string()
    }
}

/// Canonicalize genre name (standardize common variations)
fn canonicalize_genre_name(name: &str) -> String {
    let normalized = normalize_string(name).replace('-', " ").replace('_', " ");

    // Map common variations to canonical forms
    match normalized.as_str() {
        "hip hop" | "hiphop" | "hip_hop" => "Hip-Hop".to_string(),
        "r&b" | "rnb" | "r and b" | "rhythm and blues" => "R&B".to_string(),
        "edm" | "electronic dance music" => "EDM".to_string(),
        "alt rock" | "alternative rock" => "Alternative Rock".to_string(),
        "indie pop" | "indiepop" => "Indie Pop".to_string(),
        "drum and bass" | "drum & bass" | "dnb" => "Drum & Bass".to_string(),
        "k pop" | "kpop" => "K-Pop".to_string(),
        "j pop" | "jpop" => "J-Pop".to_string(),
        _ => {
            // Default: Title case
            name.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first
                            .to_uppercase()
                            .chain(chars.as_str().to_lowercase().chars())
                            .collect(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

/// Find genre by canonical name
async fn find_genre_by_canonical(pool: &SqlitePool, canonical_name: &str) -> Result<Option<Genre>> {
    use sqlx::Row;

    let row = sqlx::query(
        "SELECT id, name, canonical_name, created_at FROM genres WHERE LOWER(canonical_name) = LOWER(?)"
    )
    .bind(canonical_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Genre {
        id: row.get("id"),
        name: row.get("name"),
        canonical_name: row.get("canonical_name"),
        created_at: row.get("created_at"),
    }))
}

/// Find genre by exact name
async fn find_genre_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Genre>> {
    use sqlx::Row;

    let row = sqlx::query(
        "SELECT id, name, canonical_name, created_at FROM genres WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Genre {
        id: row.get("id"),
        name: row.get("name"),
        canonical_name: row.get("canonical_name"),
        created_at: row.get("created_at"),
    }))
}

/// Create a new genre
async fn create_genre(pool: &SqlitePool, name: &str, canonical_name: &str) -> Result<Genre> {
    let result = sqlx::query(
        "INSERT INTO genres (name, canonical_name) VALUES (?, ?)"
    )
    .bind(name)
    .bind(canonical_name)
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    Ok(Genre {
        id,
        name: name.to_string(),
        canonical_name: canonical_name.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_string() {
        assert_eq!(normalize_string("  The Beatles  "), "the beatles");
        assert_eq!(normalize_string("Hip-Hop"), "hip-hop");
        assert_eq!(normalize_string("  Multiple   Spaces  "), "multiple spaces");
    }

    #[test]
    fn test_normalize_sort_name() {
        assert_eq!(normalize_sort_name("The Beatles"), "Beatles");
        assert_eq!(normalize_sort_name("A Day To Remember"), "Day To Remember");
        assert_eq!(normalize_sort_name("An Artist"), "Artist");
        assert_eq!(normalize_sort_name("Queen"), "Queen");
    }

    #[test]
    fn test_canonicalize_genre_name() {
        assert_eq!(canonicalize_genre_name("hip hop"), "Hip-Hop");
        assert_eq!(canonicalize_genre_name("Hip-Hop"), "Hip-Hop");
        assert_eq!(canonicalize_genre_name("HIPHOP"), "Hip-Hop");
        assert_eq!(canonicalize_genre_name("r&b"), "R&B");
        assert_eq!(canonicalize_genre_name("rnb"), "R&B");
        assert_eq!(canonicalize_genre_name("indie pop"), "Indie Pop");
        assert_eq!(canonicalize_genre_name("rock"), "Rock");
    }
}
