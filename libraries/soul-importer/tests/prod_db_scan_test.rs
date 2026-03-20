//! Tests against the production database to validate scan completeness.
//! Run with: cargo test --test prod_db_scan_test -- --nocapture --ignored

use sqlx::SqlitePool;
use std::path::Path;

async fn connect_prod_db() -> Option<SqlitePool> {
    let appdata = std::env::var("APPDATA").ok()?;
    let db_path = format!("{}/Soul Player/soul-player.db", appdata);
    if !Path::new(&db_path).exists() {
        eprintln!("SKIP: prod DB not found at {}", db_path);
        return None;
    }
    SqlitePool::connect(&format!("sqlite:{}?mode=ro", db_path))
        .await
        .ok()
}

#[tokio::test]
#[ignore]
async fn prod_db_identify_missing_albums() {
    let pool = match connect_prod_db().await {
        Some(p) => p,
        None => return,
    };

    // Get all folder_paths stored in DB
    let db_folders: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT DISTINCT folder_path FROM albums")
            .fetch_all(&pool)
            .await
            .expect("query failed");

    let db_folder_set: std::collections::HashSet<String> = db_folders
        .into_iter()
        .filter_map(|(f,)| f)
        .map(|f| f.replace('\\', "/"))
        .collect();

    println!("DB folder paths: {}", db_folder_set.len());

    // Scan filesystem album dirs
    let mut missing = vec![];
    for entry in walkdir::WalkDir::new("D:/music")
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let path = entry.path().to_string_lossy().replace('\\', "/");
        // Skip resource forks
        if entry.file_name().to_string_lossy().starts_with("._") {
            continue;
        }
        // Only dirs that contain audio files (are album dirs)
        let has_audio = std::fs::read_dir(entry.path())
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok()).any(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    matches!(
                        name.rsplit('.').next().unwrap_or(""),
                        "flac" | "mp3" | "wav" | "opus" | "dsf" | "dff" | "aiff" | "m4a" | "ogg"
                    )
                })
            })
            .unwrap_or(false);

        if has_audio && !db_folder_set.contains(&path) {
            missing.push(path);
        }
    }

    println!("\nMissing album dirs ({}):", missing.len());
    for m in &missing {
        println!("  {}", m);
    }

    pool.close().await;
}

#[tokio::test]
#[ignore]
async fn prod_db_go_on_album_has_artwork() {
    let pool = match connect_prod_db().await {
        Some(p) => p,
        None => return,
    };

    let results: Vec<(i64, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, title, cover_art_path, folder_path FROM albums WHERE LOWER(title) LIKE '%go on%'"
    ).fetch_all(&pool).await.expect("query failed");

    println!("Go On album(s) in DB:");
    for (id, title, cover, folder) in &results {
        println!("  id={id} title={title:?} cover={cover:?} folder={folder:?}");
        // List actual files in folder
        if let Some(f) = folder {
            let f = f.replace('\\', "/");
            if let Ok(rd) = std::fs::read_dir(&f) {
                for entry in rd.filter_map(|e| e.ok()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".png")
                    {
                        println!("    image file: {}", name);
                    }
                }
            }
        }
    }

    assert!(!results.is_empty(), "Go On album must exist in DB");
    pool.close().await;
}

#[tokio::test]
#[ignore]
async fn prod_db_albums_without_artwork_with_available_images() {
    let pool = match connect_prod_db().await {
        Some(p) => p,
        None => return,
    };

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT title, folder_path FROM albums WHERE cover_art_path IS NULL ORDER BY title",
    )
    .fetch_all(&pool)
    .await
    .expect("query failed");

    let mut has_images = 0;
    println!("Albums without artwork that HAVE image files (missed by discovery):");
    for (title, folder) in &rows {
        let Some(f) = folder else { continue };
        let f = f.replace('\\', "/");
        let images: Vec<_> = std::fs::read_dir(&f)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let n = e.file_name().to_string_lossy().to_lowercase();
                (n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".png"))
                    && !e.file_name().to_string_lossy().starts_with("._")
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        if !images.is_empty() {
            has_images += 1;
            println!("  {:?}: {:?}", title, images);
        }
    }
    println!(
        "\n{} albums have image files but no cover_art_path",
        has_images
    );

    pool.close().await;
}
