//! Fuzzy matching with confidence scoring for artists, albums, and genres

use crate::{FuzzyMatch, MatchType, Result};
use soul_core::types::{
    Album, AlbumId, Artist, ArtistId, CreateAlbum, CreateArtist, Genre, GenreId,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use strsim::normalized_levenshtein;

/// In-memory cache of entities for O(1) lookups during import scans.
///
/// Instead of querying the database for ALL artists/albums on every file,
/// this cache loads all entities once at the start and provides fast
/// normalized-name lookups. New entities created during the scan are
/// inserted into the cache so subsequent files benefit immediately.
pub struct EntityCache {
    /// normalized_name -> (id, original_name)
    artists: HashMap<String, (ArtistId, String)>,
    /// (normalized_title, artist_id) -> (id, original_title)
    albums: HashMap<(String, Option<ArtistId>), (AlbumId, String)>,
    /// normalized_name -> (id, original_name)
    genres: HashMap<String, (GenreId, String)>,
}

impl EntityCache {
    /// Load all artists, albums, and genres from the database into the cache.
    pub async fn preload(pool: &SqlitePool) -> Result<Self> {
        let all_artists = soul_storage::artists::get_all(pool).await?;
        let all_albums = soul_storage::albums::get_all(pool).await?;
        let all_genres = soul_storage::genres::get_all(pool).await?;

        let mut artists = HashMap::with_capacity(all_artists.len());
        for artist in all_artists {
            let normalized = normalize_string(&artist.name);
            artists.insert(normalized, (artist.id, artist.name));
        }

        let mut albums = HashMap::with_capacity(all_albums.len());
        for album in all_albums {
            let normalized = normalize_string(&album.title);
            albums.insert((normalized, album.artist_id), (album.id, album.title));
        }

        let mut genres = HashMap::with_capacity(all_genres.len());
        for genre in all_genres {
            let normalized = normalize_string(&genre.name);
            genres.insert(normalized, (genre.id, genre.name));
        }

        Ok(Self {
            artists,
            albums,
            genres,
        })
    }

    /// Find an artist by normalized name. Returns `(id, original_name)`.
    pub fn find_artist_by_normalized(&self, normalized_name: &str) -> Option<(ArtistId, &str)> {
        self.artists
            .get(normalized_name)
            .map(|(id, name)| (*id, name.as_str()))
    }

    /// Find an album by normalized title and artist_id. Returns `(id, original_title)`.
    pub fn find_album_by_normalized(
        &self,
        normalized_title: &str,
        artist_id: Option<ArtistId>,
    ) -> Option<(AlbumId, &str)> {
        self.albums
            .get(&(normalized_title.to_string(), artist_id))
            .map(|(id, title)| (*id, title.as_str()))
    }

    /// Find a genre by normalized name. Returns `(id, original_name)`.
    pub fn find_genre_by_normalized(&self, normalized_name: &str) -> Option<(GenreId, &str)> {
        self.genres
            .get(normalized_name)
            .map(|(id, name)| (*id, name.as_str()))
    }

    /// Insert a new artist into the cache after creating it in the DB.
    pub fn insert_artist(&mut self, id: ArtistId, name: &str) {
        let normalized = normalize_string(name);
        self.artists.insert(normalized, (id, name.to_string()));
    }

    /// Insert a new album into the cache after creating it in the DB.
    pub fn insert_album(&mut self, id: AlbumId, title: &str, artist_id: Option<ArtistId>) {
        let normalized = normalize_string(title);
        self.albums
            .insert((normalized, artist_id), (id, title.to_string()));
    }

    /// Insert a new genre into the cache after creating it in the DB.
    pub fn insert_genre(&mut self, id: GenreId, name: &str) {
        let normalized = normalize_string(name);
        self.genres.insert(normalized, (id, name.to_string()));
    }

    /// Iterate over all artist entries for Levenshtein fallback.
    /// Yields `(normalized_name, id, original_name)`.
    pub fn artist_entries(&self) -> impl Iterator<Item = (&str, ArtistId, &str)> {
        self.artists
            .iter()
            .map(|(norm, (id, name))| (norm.as_str(), *id, name.as_str()))
    }

    /// Iterate over album entries for a specific artist (Levenshtein fallback).
    /// Yields `(normalized_title, id, original_title)`.
    pub fn album_entries_for_artist(
        &self,
        artist_id: Option<ArtistId>,
    ) -> impl Iterator<Item = (&str, AlbumId, &str)> {
        self.albums
            .iter()
            .filter(move |((_, aid), _)| *aid == artist_id)
            .map(|((norm, _), (id, title))| (norm.as_str(), *id, title.as_str()))
    }
}

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
    pub async fn find_or_create_artist(
        &self,
        pool: &SqlitePool,
        name: &str,
    ) -> Result<FuzzyMatch<Artist>> {
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
            let similarity =
                normalized_levenshtein(&normalized_name, &normalize_string(&artist.name));

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

            let similarity =
                normalized_levenshtein(&normalized_title, &normalize_string(&album.title));

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

    // --- Cached variants (use EntityCache instead of DB scans) ---

    /// Find or create an artist using the in-memory cache for O(1) lookups.
    /// Falls back to Levenshtein on cache entries (not DB) if no exact/normalized match.
    /// Creates in DB + updates cache if truly new.
    pub async fn find_or_create_artist_cached(
        &self,
        pool: &SqlitePool,
        name: &str,
        cache: &mut EntityCache,
    ) -> Result<FuzzyMatch<Artist>> {
        let normalized_name = normalize_string(name);

        // Check cache for exact original name match
        // We look up by normalized; if the original name matches exactly, confidence=100
        if let Some((id, original_name)) = cache.find_artist_by_normalized(&normalized_name) {
            let artist = soul_storage::artists::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached artist id {} not found in DB", id))
                })?;

            let confidence = if original_name == name { 100 } else { 95 };
            let match_type = if confidence == 100 {
                MatchType::Exact
            } else {
                MatchType::Normalized
            };

            return Ok(FuzzyMatch {
                entity: artist,
                confidence,
                match_type,
            });
        }

        // Levenshtein fallback on cache entries
        let mut best_match: Option<(ArtistId, f64)> = None;

        for (norm, id, _original) in cache.artist_entries() {
            let similarity = normalized_levenshtein(&normalized_name, norm);
            if similarity >= (self.fuzzy_threshold as f64 / 100.0) {
                if let Some((_, best_sim)) = &best_match {
                    if similarity > *best_sim {
                        best_match = Some((id, similarity));
                    }
                } else {
                    best_match = Some((id, similarity));
                }
            }
        }

        if let Some((id, similarity)) = best_match {
            let artist = soul_storage::artists::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached artist id {} not found in DB", id))
                })?;
            let confidence = (similarity * 100.0).round() as u8;
            return Ok(FuzzyMatch {
                entity: artist,
                confidence,
                match_type: MatchType::Fuzzy,
            });
        }

        // No match - create new artist
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

        cache.insert_artist(new_artist.id, &new_artist.name);

        Ok(FuzzyMatch {
            entity: new_artist,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }

    /// Find or create an album using the in-memory cache for O(1) lookups.
    pub async fn find_or_create_album_cached(
        &self,
        pool: &SqlitePool,
        title: &str,
        artist_id: Option<ArtistId>,
        cache: &mut EntityCache,
    ) -> Result<FuzzyMatch<Album>> {
        let normalized_title = normalize_string(title);

        // Check cache for exact/normalized match
        if let Some((id, original_title)) =
            cache.find_album_by_normalized(&normalized_title, artist_id)
        {
            let album = soul_storage::albums::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached album id {} not found in DB", id))
                })?;

            let confidence = if original_title == title { 100 } else { 95 };
            let match_type = if confidence == 100 {
                MatchType::Exact
            } else {
                MatchType::Normalized
            };

            return Ok(FuzzyMatch {
                entity: album,
                confidence,
                match_type,
            });
        }

        // Levenshtein fallback on cache entries for this artist
        let mut best_match: Option<(AlbumId, f64)> = None;

        for (norm, id, _original) in cache.album_entries_for_artist(artist_id) {
            let similarity = normalized_levenshtein(&normalized_title, norm);
            if similarity >= (self.fuzzy_threshold as f64 / 100.0) {
                if let Some((_, best_sim)) = &best_match {
                    if similarity > *best_sim {
                        best_match = Some((id, similarity));
                    }
                } else {
                    best_match = Some((id, similarity));
                }
            }
        }

        if let Some((id, similarity)) = best_match {
            let album = soul_storage::albums::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached album id {} not found in DB", id))
                })?;
            let confidence = (similarity * 100.0).round() as u8;
            return Ok(FuzzyMatch {
                entity: album,
                confidence,
                match_type: MatchType::Fuzzy,
            });
        }

        // No match - create new album
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

        cache.insert_album(new_album.id, &new_album.title, artist_id);

        Ok(FuzzyMatch {
            entity: new_album,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }

    /// Find or create a genre using the in-memory cache for O(1) lookups.
    pub async fn find_or_create_genre_cached(
        &self,
        pool: &SqlitePool,
        name: &str,
        cache: &mut EntityCache,
    ) -> Result<FuzzyMatch<Genre>> {
        let canonical_name = canonicalize_genre_name(name);
        let normalized_canonical = normalize_string(&canonical_name);

        // Check cache by canonical name (normalized)
        if let Some((id, original_name)) = cache.find_genre_by_normalized(&normalized_canonical) {
            let genre = soul_storage::genres::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached genre id {} not found in DB", id))
                })?;

            let confidence = if original_name.to_lowercase() == name.to_lowercase() {
                100
            } else {
                95
            };
            let match_type = if confidence == 100 {
                MatchType::Exact
            } else {
                MatchType::Normalized
            };

            return Ok(FuzzyMatch {
                entity: genre,
                confidence,
                match_type,
            });
        }

        // Also try looking up by the input name directly (normalized)
        let normalized_name = normalize_string(name);
        if normalized_name != normalized_canonical {
            if let Some((id, _original_name)) = cache.find_genre_by_normalized(&normalized_name) {
                let genre = soul_storage::genres::get_by_id(pool, id)
                    .await?
                    .ok_or_else(|| {
                        crate::ImportError::Metadata(format!(
                            "Cached genre id {} not found in DB",
                            id
                        ))
                    })?;

                return Ok(FuzzyMatch {
                    entity: genre,
                    confidence: 95,
                    match_type: MatchType::Normalized,
                });
            }
        }

        // Create new genre
        let new_genre = create_genre(pool, name, &canonical_name).await?;

        // Insert into cache by both the original name and canonical name
        cache.insert_genre(new_genre.id, &new_genre.name);
        if normalized_canonical != normalize_string(&new_genre.name) {
            // Also cache under canonical name for future lookups
            cache.insert_genre(new_genre.id, &canonical_name);
        }

        Ok(FuzzyMatch {
            entity: new_genre,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }

    /// Find or create a genre with fuzzy matching and canonicalization
    pub async fn find_or_create_genre(
        &self,
        pool: &SqlitePool,
        name: &str,
    ) -> Result<FuzzyMatch<Genre>> {
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
    let normalized = normalize_string(name).replace(['-', '_'], " ");

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

    let row = sqlx::query("SELECT id, name, canonical_name, created_at FROM genres WHERE name = ?")
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
    let result = sqlx::query("INSERT INTO genres (name, canonical_name) VALUES (?, ?)")
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
