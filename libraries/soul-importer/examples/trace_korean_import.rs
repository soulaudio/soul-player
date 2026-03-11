/// Trace Korean character handling through the full import pipeline
/// Tests: D:\music\Indie\Mid-Air Thief\기다림 (Waiting)\01 - 기다림 (Waiting).mp3
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_file =
        Path::new(r"D:\music\Indie\Mid-Air Thief\기다림 (Waiting)\01 - 기다림 (Waiting).mp3");

    println!("═══════════════════════════════════════════════════════════");
    println!("KOREAN CHARACTER IMPORT TRACING TEST");
    println!("═══════════════════════════════════════════════════════════\n");

    // STAGE 1: PATH HANDLING
    println!("📁 STAGE 1: PATH HANDLING");
    println!("  File path: {}", test_file.display());
    println!("  File exists: {}", test_file.exists());

    if let Some(parent) = test_file.parent() {
        if let Some(folder_name) = parent.file_name() {
            println!("  Folder name (OsStr): {:?}", folder_name);
            println!(
                "  Folder name (to_string_lossy): {}",
                folder_name.to_string_lossy()
            );
            println!(
                "  Folder name bytes: {:?}",
                folder_name.to_string_lossy().as_bytes()
            );

            // Check if contains Korean
            let folder_str = folder_name.to_string_lossy();
            if folder_str.contains('기') {
                println!("  ✅ Korean character '기' found in folder name");
            } else {
                println!("  ❌ Korean character '기' NOT found in folder name");
            }
        }
    }
    println!();

    // STAGE 2: METADATA EXTRACTION
    println!("📋 STAGE 2: METADATA EXTRACTION (lofty)");
    match soul_importer::metadata::extract_metadata(test_file) {
        Ok(metadata) => {
            println!("  ✅ Metadata extracted successfully");
            println!("  Title: {:?}", metadata.title);
            println!("  Artists: {:?}", metadata.artists);
            println!("  Album: {:?}", metadata.album);

            if !metadata.artists.is_empty() {
                let artist = metadata.artists.join(", ");
                println!("  Artist bytes: {:?}", artist.as_bytes());
                if artist.contains('기') || artist.contains("???") {
                    if artist.contains('기') {
                        println!("  ✅ Korean character '기' found in artist");
                    } else if artist.contains("???") {
                        println!("  ❌ Found '???' in artist name - ENCODING ISSUE DETECTED HERE!");
                    }
                }
            }

            if let Some(ref album) = metadata.album {
                println!("  Album bytes: {:?}", album.as_bytes());
                if album.contains('기') || album.contains("???") {
                    if album.contains('기') {
                        println!("  ✅ Korean character '기' found in album");
                    } else if album.contains("???") {
                        println!("  ❌ Found '???' in album name - ENCODING ISSUE DETECTED HERE!");
                    }
                }
            }

            // If metadata doesn't have Korean, check folder fallback
            if metadata.artists.is_empty() || metadata.album.is_none() {
                println!("\n  📝 Testing folder name fallback...");
                if let Some(parent) = test_file.parent() {
                    if let Some(folder_name) = parent.file_name() {
                        let folder_str = folder_name.to_string_lossy();
                        let parsed = soul_importer::metadata::parse_folder_name(&folder_str);
                        println!(
                            "  Parsed from folder: artist={:?}, album={:?}",
                            parsed.artist, parsed.album
                        );

                        if let Some(ref artist) = parsed.artist {
                            if artist.contains('기') {
                                println!("  ✅ Korean '기' preserved in parsed artist");
                            } else if artist.contains("???") {
                                println!(
                                    "  ❌ Found '???' in parsed artist - FOLDER PARSING ISSUE!"
                                );
                            }
                        }

                        if let Some(ref album) = parsed.album {
                            if album.contains('기') {
                                println!("  ✅ Korean '기' preserved in parsed album");
                            } else if album.contains("???") {
                                println!(
                                    "  ❌ Found '???' in parsed album - FOLDER PARSING ISSUE!"
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("  ❌ Failed to extract metadata: {}", e);
            return Ok(());
        }
    }
    println!();

    // STAGE 3: DATABASE SIMULATION
    println!("💾 STAGE 3: DATABASE STRING HANDLING");
    let test_strings = vec![
        ("Korean direct", "기다림 (Waiting)"),
        ("Mixed", "Mid-Air Thief"),
    ];

    for (label, test_str) in test_strings {
        println!("  Testing '{}': {}", label, test_str);
        println!("    Bytes: {:?}", test_str.as_bytes());
        println!(
            "    Len: {} bytes, {} chars",
            test_str.len(),
            test_str.chars().count()
        );

        // Simulate SQLite round-trip by checking UTF-8 validity
        if std::str::from_utf8(test_str.as_bytes()).is_ok() {
            println!("    ✅ Valid UTF-8 for database");
        } else {
            println!("    ❌ Invalid UTF-8 - would corrupt in database!");
        }
    }
    println!();

    // STAGE 4: JSON SERIALIZATION (for Tauri commands)
    println!("📤 STAGE 4: JSON SERIALIZATION (Tauri IPC)");
    let test_artist = serde_json::json!({
        "id": 1,
        "name": "기다림 (Waiting)",
        "album_count": 1,
        "track_count": 1
    });

    let json_str = serde_json::to_string(&test_artist)?;
    println!("  Serialized JSON: {}", json_str);

    if json_str.contains("기다림") {
        println!("  ✅ Korean characters preserved in JSON");
    } else if json_str.contains("???") {
        println!("  ❌ Found '???' in JSON - SERIALIZATION ISSUE!");
    } else {
        println!("  ⚠️  Korean characters may be escaped but not replaced");
    }

    // Try to deserialize back
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
        println!("  Deserialized name: {}", name);
        if name.contains('기') {
            println!("  ✅ Korean '기' survived round-trip");
        } else if name.contains("???") {
            println!("  ❌ Korean became '???' in round-trip!");
        }
    }
    println!();

    println!("═══════════════════════════════════════════════════════════");
    println!("TEST COMPLETE");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
