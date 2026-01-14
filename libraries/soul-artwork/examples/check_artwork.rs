use lofty::{Probe, TaggedFileExt};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example check_artwork <audio_file>");
        std::process::exit(1);
    }

    let file_path = &args[1];

    println!("Checking file: {}", file_path);

    match Probe::open(file_path) {
        Ok(probe) => {
            match probe.read() {
                Ok(tagged_file) => {
                    let mut found_artwork = false;

                    // Check all tags
                    for tag in tagged_file.tags() {
                        let pictures = tag.pictures();
                        if !pictures.is_empty() {
                            println!(
                                "✓ Found {} picture(s) in {:?} tag",
                                pictures.len(),
                                tag.tag_type()
                            );
                            for (i, pic) in pictures.iter().enumerate() {
                                println!(
                                    "  Picture {}: {:?} ({} bytes)",
                                    i + 1,
                                    pic.pic_type(),
                                    pic.data().len()
                                );
                            }
                            found_artwork = true;
                        }
                    }

                    if found_artwork {
                        println!("\n✓ Artwork is embedded in the file!");
                        println!("If Windows Media Player doesn't show it, try:");
                        println!("  1. Close Windows Media Player completely");
                        println!("  2. Delete: %LOCALAPPDATA%\\Microsoft\\Media Player\\*.wmdb");
                        println!("  3. Restart Windows Media Player and re-scan library");
                    } else {
                        println!("✗ No artwork found in file");
                    }
                }
                Err(e) => eprintln!("Error reading file: {}", e),
            }
        }
        Err(e) => eprintln!("Error opening file: {}", e),
    }
}
