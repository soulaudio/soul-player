use anyhow::Result;
use std::path::PathBuf;

use crate::util::{
    fs::{find_dirs, remove_dir_all},
    output::{print_complete, print_header, print_step, print_success, print_warning},
    platform::workspace_root,
};

/// Clean caches (SQLx, build, node_modules/.cache)
pub fn run() -> Result<()> {
    print_header("Cleaning caches");

    let root = workspace_root()?;

    // 1. Clean SQLx offline data
    print_step("Cleaning SQLx offline data");
    clean_sqlx_cache(&root)?;

    // 2. Clean Cargo incremental build cache
    print_step("Cleaning Cargo incremental build cache");
    clean_cargo_incremental_cache(&root)?;

    // 3. Clean node_modules/.cache directories
    print_step("Cleaning node_modules caches");
    clean_node_modules_cache(&root)?;

    print_complete("Cache cleanup complete!");
    println!();
    print_warning("SQLx queries will need to be re-prepared: cargo sqlx prepare -- --lib");

    Ok(())
}

fn clean_sqlx_cache(root: &PathBuf) -> Result<()> {
    let sqlx_dir = root.join("libraries/soul-storage/.sqlx");

    if sqlx_dir.exists() {
        remove_dir_all(&sqlx_dir)?;
        print_success(&format!("Removed {}", sqlx_dir.display()));
    } else {
        print_warning("No SQLx cache found");
    }

    Ok(())
}

fn clean_cargo_incremental_cache(root: &PathBuf) -> Result<()> {
    let incremental_dirs = vec![
        root.join("target/debug/incremental"),
        root.join("target/release/incremental"),
        root.join("applications/desktop/src-tauri/target/debug/incremental"),
        root.join("applications/desktop/src-tauri/target/release/incremental"),
    ];

    let mut found_any = false;

    for dir in incremental_dirs {
        if dir.exists() {
            remove_dir_all(&dir)?;
            print_success(&format!("Removed {}", dir.display()));
            found_any = true;
        }
    }

    if !found_any {
        print_warning("No Cargo incremental cache found");
    }

    Ok(())
}

fn clean_node_modules_cache(root: &PathBuf) -> Result<()> {
    // Find all node_modules directories
    let node_modules_dirs = find_dirs(root, "node_modules")?;

    if node_modules_dirs.is_empty() {
        print_warning("No node_modules directories found");
        return Ok(());
    }

    let mut found_any = false;

    for nm_dir in node_modules_dirs {
        let cache_dir = nm_dir.join(".cache");
        if cache_dir.exists() {
            remove_dir_all(&cache_dir)?;
            print_success(&format!("Removed {}", cache_dir.display()));
            found_any = true;
        }
    }

    if !found_any {
        print_warning("No node_modules cache directories found");
    }

    Ok(())
}
