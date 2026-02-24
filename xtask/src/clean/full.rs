use anyhow::Result;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::util::{
    fs::{find_dirs, remove_dir_all},
    output::{print_complete, print_error, print_header, print_step, print_success, print_warning},
    platform::workspace_root,
};

#[allow(deprecated)]
use dirs;

/// Nuclear clean - removes everything including node_modules, SQLx cache, and optionally Cargo cache
pub fn run(cargo_cache: bool) -> Result<()> {
    print_header("Full cleanup (nuclear)");
    println!();
    print_warning("This will remove ALL build artifacts, dependencies, and caches");
    print_warning("You will need to run 'cargo xtask setup all' afterwards");

    if cargo_cache {
        print_warning("Cargo cache will also be deleted (this is rarely needed)");
    }

    println!();
    print!("Continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        print_error("Cleanup cancelled");
        return Ok(());
    }

    let root = workspace_root()?;

    // 1. Clean Rust target directories
    print_step("Cleaning Rust build artifacts");
    clean_rust_targets(&root)?;

    // 2. Clean all node_modules
    print_step("Cleaning all node_modules");
    clean_all_node_modules(&root)?;

    // 3. Clean SQLx offline data
    print_step("Cleaning SQLx offline data");
    clean_sqlx_cache(&root)?;

    // 4. Clean test databases
    print_step("Cleaning test databases");
    clean_test_databases(&root)?;

    // 5. Clean dist folders
    print_step("Cleaning dist folders");
    clean_dist_folders(&root)?;

    // 6. Clean .tmp directories
    print_step("Cleaning temporary directories");
    clean_tmp_dirs(&root)?;

    // 7. Optionally clean Cargo cache
    if cargo_cache {
        print_step("Cleaning Cargo cache");
        clean_cargo_cache()?;
    }

    print_complete("Full cleanup complete!");
    println!();
    print_warning("Next steps:");
    println!("  1. Run: cargo xtask setup all");
    println!("  2. Run: cargo xtask dev desktop");

    Ok(())
}

fn clean_rust_targets(root: &Path) -> Result<()> {
    let targets = vec![
        root.join("target"),
        root.join("applications/desktop/src-tauri/target"),
    ];

    for target in targets {
        if target.exists() {
            remove_dir_all(&target)?;
            print_success(&format!("Removed {}", target.display()));
        }
    }

    Ok(())
}

fn clean_all_node_modules(root: &Path) -> Result<()> {
    let node_modules_dirs = find_dirs(root, "node_modules")?;

    if node_modules_dirs.is_empty() {
        print_warning("No node_modules directories found");
        return Ok(());
    }

    for dir in node_modules_dirs {
        remove_dir_all(&dir)?;
        print_success(&format!("Removed {}", dir.display()));
    }

    Ok(())
}

fn clean_sqlx_cache(root: &Path) -> Result<()> {
    let sqlx_dir = root.join("libraries/soul-storage/.sqlx");

    if sqlx_dir.exists() {
        remove_dir_all(&sqlx_dir)?;
        print_success(&format!("Removed {}", sqlx_dir.display()));
    } else {
        print_warning("No SQLx cache found");
    }

    Ok(())
}

fn clean_test_databases(root: &Path) -> Result<()> {
    let tmp_dir = root.join(".tmp");

    if !tmp_dir.exists() {
        print_warning("No .tmp directory found");
        return Ok(());
    }

    // Find all .db files in .tmp
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        let mut found_any = false;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                if let Err(e) = std::fs::remove_file(&path) {
                    print_warning(&format!("Failed to remove {}: {}", path.display(), e));
                } else {
                    print_success(&format!("Removed {}", path.display()));
                    found_any = true;
                }
            }
        }
        if !found_any {
            print_warning("No test databases found");
        }
    }

    Ok(())
}

fn clean_dist_folders(root: &Path) -> Result<()> {
    let applications_dir = root.join("applications");
    if !applications_dir.exists() {
        return Ok(());
    }

    let dist_dirs = find_dirs(&applications_dir, "dist")?;

    if dist_dirs.is_empty() {
        print_warning("No dist directories found");
        return Ok(());
    }

    for dir in dist_dirs {
        remove_dir_all(&dir)?;
        print_success(&format!("Removed {}", dir.display()));
    }

    Ok(())
}

fn clean_tmp_dirs(root: &Path) -> Result<()> {
    let tmp_dirs = find_dirs(root, ".tmp")?;

    if tmp_dirs.is_empty() {
        print_warning("No .tmp directories found");
        return Ok(());
    }

    for dir in tmp_dirs {
        remove_dir_all(&dir)?;
        print_success(&format!("Removed {}", dir.display()));
    }

    Ok(())
}

fn clean_cargo_cache() -> Result<()> {
    // Get cargo home directory
    let cargo_home = std::env::var("CARGO_HOME")
        .ok()
        .or_else(|| dirs::home_dir().map(|h| h.join(".cargo").to_string_lossy().to_string()));

    if let Some(cargo_home) = cargo_home {
        let cargo_home_path = PathBuf::from(cargo_home);
        let registry = cargo_home_path.join("registry");
        let git = cargo_home_path.join("git");

        let mut found_any = false;

        if registry.exists() {
            remove_dir_all(&registry)?;
            print_success(&format!("Removed {}", registry.display()));
            found_any = true;
        }

        if git.exists() {
            remove_dir_all(&git)?;
            print_success(&format!("Removed {}", git.display()));
            found_any = true;
        }

        if !found_any {
            print_warning("No Cargo cache found");
        }
    } else {
        print_warning("Could not determine CARGO_HOME location");
    }

    Ok(())
}
