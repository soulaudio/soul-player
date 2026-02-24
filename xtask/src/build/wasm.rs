//! WASM build automation
//!
//! Builds soul-playback WASM module for the marketing demo.
//! Replaces scripts/build-wasm.mjs

use anyhow::{Context, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::util::{exec, output, platform};

/// Build WASM module
pub fn run(watch: bool, release: bool) -> Result<()> {
    output::print_header("WASM Build");

    let workspace_root = platform::workspace_root()?;
    let wasm_source = workspace_root.join("libraries").join("soul-playback");
    let wasm_output = workspace_root
        .join("applications")
        .join("marketing")
        .join("src")
        .join("wasm")
        .join("soul-playback");

    // Verify source directory exists
    if !wasm_source.exists() {
        anyhow::bail!("Source directory not found: {}", wasm_source.display());
    }

    // Check and install wasm-pack if needed
    check_wasm_pack()?;

    if watch {
        output::print_info("Running in watch mode (debounce: 500ms)");
        output::print_info("Press Ctrl+C to stop");
        build_wasm_watch(&wasm_source, &wasm_output, release)?;
    } else {
        build_wasm_once(&wasm_source, &wasm_output, release)?;
    }

    Ok(())
}

/// Check if wasm-pack is installed, offer to install if missing
fn check_wasm_pack() -> Result<()> {
    output::print_step("Checking for wasm-pack...");

    if exec::command_exists("wasm-pack") {
        output::print_success("wasm-pack found");
        Ok(())
    } else {
        output::print_warning("wasm-pack not installed");
        output::print_info("Install with: cargo install wasm-pack");
        output::print_info("Or visit: https://rustwasm.github.io/wasm-pack/installer/");

        // Offer to install
        println!("\n  Install wasm-pack now? [y/N]: ");
        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;

        if response.trim().to_lowercase() == "y" {
            output::print_step("Installing wasm-pack...");
            let success = exec::run_command_inherit(
                "cargo",
                &["install", "wasm-pack"],
                "cargo install wasm-pack",
            )?;

            if success {
                output::print_success("wasm-pack installed successfully");
                Ok(())
            } else {
                anyhow::bail!("Failed to install wasm-pack")
            }
        } else {
            anyhow::bail!("wasm-pack is required. Install with: cargo install wasm-pack")
        }
    }
}

/// Build WASM once
fn build_wasm_once(source: &Path, output: &Path, release: bool) -> Result<()> {
    output::print_step("Building soul-playback WASM module...");
    output::print_info(&format!("Source: {}", source.display()));
    output::print_info(&format!("Output: {}", output.display()));

    let mut args = vec![
        "build",
        "--target",
        "web",
        "--out-dir",
        output.to_str().unwrap(),
    ];

    if release {
        args.push("--release");
    }

    // Add features
    args.extend_from_slice(&["--", "--features", "wasm"]);

    let success = exec::run_command_in_dir("wasm-pack", &args, source, "wasm-pack build")?;

    if success {
        output::print_success("WASM build complete!");
        output::print_success(&format!("Output: {}", output.display()));
        Ok(())
    } else {
        anyhow::bail!("WASM build failed")
    }
}

/// Build WASM in watch mode
fn build_wasm_watch(source: &Path, output: &Path, release: bool) -> Result<()> {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    // Initial build
    build_wasm_once(source, output, release)?;

    output::print_header("Watching for changes...");

    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;

    // Watch the source directory
    let src_dir = source.join("src");
    watcher
        .watch(&src_dir, RecursiveMode::Recursive)
        .context(format!("Failed to watch directory: {}", src_dir.display()))?;

    output::print_success(&format!("Watching: {}", src_dir.display()));

    // Debouncing state
    let last_build = Arc::new(Mutex::new(Instant::now()));
    let debounce_duration = Duration::from_millis(500);

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(Event {
                kind: notify::EventKind::Modify(_) | notify::EventKind::Create(_),
                paths,
                ..
            })) => {
                // Only rebuild for Rust files
                let is_rust_file = paths.iter().any(|p| {
                    p.extension()
                        .map(|ext| ext == "rs" || ext == "toml")
                        .unwrap_or(false)
                });

                if !is_rust_file {
                    continue;
                }

                // Debounce
                let mut last = last_build.lock().unwrap();
                let now = Instant::now();
                if now.duration_since(*last) < debounce_duration {
                    continue;
                }
                *last = now;
                drop(last);

                // Print changed files
                output::print_info("Files changed:");
                for path in paths {
                    println!("    - {}", path.display());
                }

                // Rebuild
                output::print_step("Rebuilding WASM...");
                match build_wasm_once(source, output, release) {
                    Ok(_) => {}
                    Err(e) => {
                        output::print_error(&format!("Build failed: {}", e));
                        output::print_info("Watching for changes...");
                    }
                }
            }
            Ok(Ok(_)) => {} // Ignore other event types
            Ok(Err(e)) => output::print_error(&format!("Watch error: {}", e)),
            Err(e) => {
                output::print_error(&format!("Channel error: {}", e));
                break;
            }
        }
    }

    Ok(())
}
