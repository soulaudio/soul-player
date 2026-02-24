use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::util::{fs, output, platform, validation};
use crate::version::{files, git};

/// Run version bump
pub fn run(version: &str, dry_run: bool, skip_git: bool, force: bool) -> Result<()> {
    // Print header
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".cyan()
    );
    println!("{}", "  Soul Player Version Bumping".cyan().bold());
    if dry_run {
        println!("{}", "  [DRY-RUN MODE - No changes will be made]".cyan());
    }
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".cyan()
    );
    println!();

    // Validate version format
    validation::validate_semver(version)?;

    // Get current version
    let current_version = get_current_version()?;

    output::print_info(&format!("Current version: {}", current_version));
    output::print_info(&format!("New version:     {}", version));
    println!();

    // Compare versions
    let current_semver = validation::SemVer::parse(&current_version)?;
    let new_semver = validation::SemVer::parse(version)?;

    match new_semver.cmp(&current_semver) {
        std::cmp::Ordering::Less => {
            output::print_warning(&format!(
                "New version {} is LOWER than current {}",
                version, current_version
            ));
            output::print_info("This is a version downgrade - are you sure?");
        }
        std::cmp::Ordering::Equal => {
            output::print_error(&format!(
                "New version {} is the SAME as current {}",
                version, current_version
            ));
            output::print_info("Version must be different from current version");
            anyhow::bail!("Version must be different");
        }
        std::cmp::Ordering::Greater => {
            output::print_success(&format!(
                "Version will be bumped from {} → {}",
                current_version, version
            ));
        }
    }

    println!();

    // Pre-flight checks (skip if dry-run)
    if !dry_run {
        run_preflight_checks(force)?;
    }

    // Backup files before modification
    let mut backups: HashMap<PathBuf, PathBuf> = HashMap::new();

    let result = (|| -> Result<Vec<PathBuf>> {
        // Update all files
        let updated_files = if dry_run {
            files::update_all_files(version, true)?;
            Vec::new()
        } else {
            // Create backups before updating
            let workspace_root = platform::workspace_root()?;
            let files_to_backup = collect_files_to_update(&workspace_root)?;

            for file in &files_to_backup {
                if file.exists() {
                    let backup = file.with_extension("backup");
                    fs::copy_file(file, &backup)?;
                    backups.insert(file.clone(), backup);
                }
            }

            // Update files
            let updated = files::update_all_files(version, false)?;

            println!();

            // Validate updates
            files::validate_updates(version)?;

            updated
        };

        Ok(updated_files)
    })();

    match result {
        Ok(updated_files) => {
            // Clean up backups on success
            if !dry_run {
                cleanup_backups(&backups)?;
            }

            println!();
            println!(
                "{}",
                "═══════════════════════════════════════════════════════".cyan()
            );

            if dry_run {
                output::print_success("Dry-run complete - no changes were made");
                output::print_info(&format!("Would update {} file(s)", updated_files.len()));
            } else {
                output::print_success("Version bump complete!");
                output::print_info(&format!("Updated {} file(s)", updated_files.len()));

                // Git operations
                if !skip_git {
                    println!();
                    git::run_git_operations(version, updated_files)?;

                    println!();
                    println!(
                        "{}",
                        "═══════════════════════════════════════════════════════".cyan()
                    );
                    output::print_complete(&format!("Release v{} initiated!", version));
                    println!();
                    output::print_info("GitHub Actions will now:");
                    println!("  • Detect the new tag v{}", version);
                    println!("  • Trigger the release workflow");
                    println!("  • Build installers for Windows, macOS, Linux");
                    println!("  • Create GitHub release with auto-generated changelog");
                    println!("  • Generate latest.json for auto-updater");
                    println!();
                    output::print_info("Monitor release progress at:");
                    println!("  https://github.com/soulaudio/soul-player/actions");
                }
            }

            println!();
            output::print_complete("Script complete!");
            println!();

            Ok(())
        }
        Err(e) => {
            // Rollback on error
            if !dry_run && !backups.is_empty() {
                output::print_error(&format!("Error: {}", e));
                rollback_changes(&backups)?;
            }

            Err(e)
        }
    }
}

/// Get current version from workspace Cargo.toml
fn get_current_version() -> Result<String> {
    let workspace_root = platform::workspace_root()?;
    let cargo_toml_path = workspace_root.join("Cargo.toml");

    let content = fs::read_file(&cargo_toml_path)?;
    let doc = content.parse::<toml_edit::DocumentMut>()?;

    let version = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Version not found in workspace Cargo.toml"))?;

    Ok(version.to_string())
}

/// Run pre-flight checks
fn run_preflight_checks(force: bool) -> Result<()> {
    output::print_header("Pre-flight Checks");

    git::check_clean_working_tree()?;
    git::check_on_main_branch(force)?;

    println!();
    output::print_success("All pre-flight checks passed");

    Ok(())
}

/// Collect all files that will be updated
fn collect_files_to_update(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Workspace Cargo.toml
    files.push(workspace_root.join("Cargo.toml"));

    // Libraries
    let libraries_dir = workspace_root.join("libraries");
    if libraries_dir.exists() {
        for entry in std::fs::read_dir(&libraries_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let cargo_toml = path.join("Cargo.toml");
                if cargo_toml.exists() {
                    files.push(cargo_toml);
                }
            }
        }
    }

    // Applications
    let app_cargos = vec![
        "applications/desktop/src-tauri/Cargo.toml",
        "applications/mobile/src-tauri/Cargo.toml",
        "applications/server/Cargo.toml",
    ];

    for app_cargo in app_cargos {
        let path = workspace_root.join(app_cargo);
        if path.exists() {
            files.push(path);
        }
    }

    // package.json files
    let package_jsons = vec![
        "package.json",
        "applications/desktop/package.json",
        "applications/shared/package.json",
        "applications/marketing/package.json",
    ];

    for pkg_json in package_jsons {
        let path = workspace_root.join(pkg_json);
        if path.exists() {
            files.push(path);
        }
    }

    // tauri.conf.json files
    let tauri_confs = vec![
        "applications/desktop/src-tauri/tauri.conf.json",
        "applications/mobile/src-tauri/tauri.conf.json",
    ];

    for tauri_conf in tauri_confs {
        let path = workspace_root.join(tauri_conf);
        if path.exists() {
            files.push(path);
        }
    }

    // Release config
    files.push(workspace_root.join(".github/release-config.json"));

    Ok(files)
}

/// Rollback changes by restoring from backups
fn rollback_changes(backups: &HashMap<PathBuf, PathBuf>) -> Result<()> {
    output::print_warning("Rolling back changes...");

    for (original, backup) in backups {
        if backup.exists() {
            fs::copy_file(backup, original)?;
            fs::remove_file(backup)?;
            output::print_success(&format!("Restored: {}", original.display()));
        }
    }

    Ok(())
}

/// Cleanup backups on success
fn cleanup_backups(backups: &HashMap<PathBuf, PathBuf>) -> Result<()> {
    for backup in backups.values() {
        if backup.exists() {
            fs::remove_file(backup)?;
        }
    }

    Ok(())
}
