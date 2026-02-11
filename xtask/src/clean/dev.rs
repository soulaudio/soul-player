use anyhow::Result;
use std::path::PathBuf;

use crate::util::{
    fs::{find_dirs, find_files, remove_dir_all, remove_file},
    output::{print_complete, print_header, print_step, print_success, print_warning},
    platform::workspace_root,
};

/// Clean development artifacts
pub fn run() -> Result<()> {
    print_header("Cleaning development artifacts");

    let root = workspace_root()?;

    // 1. Clean Rust target directories
    print_step("Cleaning Rust build artifacts");
    clean_rust_targets(&root)?;

    // 2. Clean node_modules directories
    print_step("Cleaning node_modules directories");
    clean_node_modules(&root)?;

    // 3. Clean frontend dist folders
    print_step("Cleaning frontend dist folders");
    clean_dist_folders(&root)?;

    // 4. Clean .tmp directories
    print_step("Cleaning temporary directories");
    clean_tmp_dirs(&root)?;

    // 5. Clean log files
    print_step("Cleaning log files");
    clean_log_files(&root)?;

    print_complete("Development cleanup complete!");
    println!();
    print_warning("First build will be slower (rebuilding everything)");
    print_warning("Run 'cargo xtask dev desktop' or 'yarn dev:desktop' to start fresh");

    Ok(())
}

fn clean_rust_targets(root: &PathBuf) -> Result<()> {
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

fn clean_node_modules(root: &PathBuf) -> Result<()> {
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

fn clean_dist_folders(root: &PathBuf) -> Result<()> {
    // Find all dist directories in applications/
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

fn clean_tmp_dirs(root: &PathBuf) -> Result<()> {
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

fn clean_log_files(root: &PathBuf) -> Result<()> {
    let log_files = find_files(root, ".log")?;

    if log_files.is_empty() {
        print_warning("No log files found");
        return Ok(());
    }

    for file in log_files {
        // Skip files in node_modules (already cleaned)
        if file.to_string_lossy().contains("node_modules") {
            continue;
        }

        remove_file(&file)?;
        print_success(&format!("Removed {}", file.display()));
    }

    Ok(())
}
