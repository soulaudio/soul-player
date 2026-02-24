use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::util::{fs, output, platform};

/// Update workspace Cargo.toml version
pub fn update_workspace_cargo_toml(version: &str) -> Result<PathBuf> {
    let workspace_root = platform::workspace_root()?;
    let path = workspace_root.join("Cargo.toml");

    update_cargo_toml(&path, version)?;

    Ok(path)
}

/// Update a Cargo.toml file with new version
pub fn update_cargo_toml(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_file(path)?;
    let mut doc = content.parse::<toml_edit::DocumentMut>()?;

    // Update workspace version if present
    if let Some(workspace) = doc.get_mut("workspace") {
        if let Some(package) = workspace.get_mut("package") {
            if let Some(item) = package.get_mut("version") {
                *item = toml_edit::value(version);
            }
        }
    }

    // Update package version if present
    if let Some(package) = doc.get_mut("package") {
        if let Some(item) = package.get_mut("version") {
            *item = toml_edit::value(version);
        }
    }

    fs::write_file(path, &doc.to_string())?;

    Ok(())
}

/// Update all library Cargo.toml files
pub fn update_library_cargo_tomls(version: &str) -> Result<Vec<PathBuf>> {
    let workspace_root = platform::workspace_root()?;
    let libraries_dir = workspace_root.join("libraries");

    if !libraries_dir.exists() {
        return Ok(Vec::new());
    }

    let mut updated = Vec::new();

    for entry in std::fs::read_dir(&libraries_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                update_cargo_toml(&cargo_toml, version)?;
                updated.push(cargo_toml);
            }
        }
    }

    Ok(updated)
}

/// Update application Cargo.toml files
pub fn update_app_cargo_tomls(version: &str) -> Result<Vec<PathBuf>> {
    let workspace_root = platform::workspace_root()?;
    let mut updated = Vec::new();

    // Desktop app
    let desktop_cargo = workspace_root.join("applications/desktop/src-tauri/Cargo.toml");
    if desktop_cargo.exists() {
        update_cargo_toml(&desktop_cargo, version)?;
        updated.push(desktop_cargo);
    }

    // Mobile app (if exists)
    let mobile_cargo = workspace_root.join("applications/mobile/src-tauri/Cargo.toml");
    if mobile_cargo.exists() {
        update_cargo_toml(&mobile_cargo, version)?;
        updated.push(mobile_cargo);
    }

    // Server app
    let server_cargo = workspace_root.join("applications/server/Cargo.toml");
    if server_cargo.exists() {
        update_cargo_toml(&server_cargo, version)?;
        updated.push(server_cargo);
    }

    Ok(updated)
}

/// Update a package.json file with new version
pub fn update_package_json(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_file(path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        );
    }

    // Write with 2-space indent
    let formatted = serde_json::to_string_pretty(&json)?;
    fs::write_file(path, &format!("{}\n", formatted))?;

    Ok(())
}

/// Update all package.json files
pub fn update_package_jsons(version: &str) -> Result<Vec<PathBuf>> {
    let workspace_root = platform::workspace_root()?;
    let mut updated = Vec::new();

    // Root package.json
    let root_package = workspace_root.join("package.json");
    if root_package.exists() {
        update_package_json(&root_package, version)?;
        updated.push(root_package);
    }

    // Desktop package.json
    let desktop_package = workspace_root.join("applications/desktop/package.json");
    if desktop_package.exists() {
        update_package_json(&desktop_package, version)?;
        updated.push(desktop_package);
    }

    // Shared package.json
    let shared_package = workspace_root.join("applications/shared/package.json");
    if shared_package.exists() {
        update_package_json(&shared_package, version)?;
        updated.push(shared_package);
    }

    // Marketing package.json
    let marketing_package = workspace_root.join("applications/marketing/package.json");
    if marketing_package.exists() {
        update_package_json(&marketing_package, version)?;
        updated.push(marketing_package);
    }

    Ok(updated)
}

/// Update a tauri.conf.json file
pub fn update_tauri_conf(path: &Path, version: &str) -> Result<()> {
    let content = fs::read_file(path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    // Update version field (Tauri 2.0 uses top-level "version")
    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        );
    }

    // Write with 2-space indent
    let formatted = serde_json::to_string_pretty(&json)?;
    fs::write_file(path, &format!("{}\n", formatted))?;

    Ok(())
}

/// Update all tauri.conf.json files
pub fn update_tauri_confs(version: &str) -> Result<Vec<PathBuf>> {
    let workspace_root = platform::workspace_root()?;
    let mut updated = Vec::new();

    // Desktop tauri.conf.json
    let desktop_tauri = workspace_root.join("applications/desktop/src-tauri/tauri.conf.json");
    if desktop_tauri.exists() {
        update_tauri_conf(&desktop_tauri, version)?;
        updated.push(desktop_tauri);
    }

    // Mobile tauri.conf.json (if exists)
    let mobile_tauri = workspace_root.join("applications/mobile/src-tauri/tauri.conf.json");
    if mobile_tauri.exists() {
        update_tauri_conf(&mobile_tauri, version)?;
        updated.push(mobile_tauri);
    }

    Ok(updated)
}

/// Update .github/release-config.json
pub fn update_release_config(version: &str) -> Result<PathBuf> {
    let workspace_root = platform::workspace_root()?;
    let path = workspace_root.join(".github/release-config.json");

    if !path.exists() {
        anyhow::bail!("Release config not found: {}", path.display());
    }

    let content = fs::read_file(&path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        );
    }

    // Write with 2-space indent
    let formatted = serde_json::to_string_pretty(&json)?;
    fs::write_file(&path, &format!("{}\n", formatted))?;

    Ok(path)
}

/// Update all files and return list of updated paths
pub fn update_all_files(version: &str, dry_run: bool) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    output::print_header("Updating Files");

    // Workspace Cargo.toml
    if dry_run {
        output::print_step("Would update: Cargo.toml (workspace)");
    } else {
        let path = update_workspace_cargo_toml(version)?;
        output::print_success("Cargo.toml (workspace)");
        all_files.push(path);
    }

    // Libraries
    if dry_run {
        output::print_step("Would update: libraries/*/Cargo.toml");
    } else {
        let mut libs = update_library_cargo_tomls(version)?;
        for lib in &libs {
            let rel_path = lib
                .strip_prefix(&platform::workspace_root()?)
                .unwrap_or(lib);
            output::print_success(&format!("{}", rel_path.display()));
        }
        all_files.append(&mut libs);
    }

    // Applications
    if dry_run {
        output::print_step("Would update: applications/*/Cargo.toml");
    } else {
        let mut apps = update_app_cargo_tomls(version)?;
        for app in &apps {
            let rel_path = app
                .strip_prefix(&platform::workspace_root()?)
                .unwrap_or(app);
            output::print_success(&format!("{}", rel_path.display()));
        }
        all_files.append(&mut apps);
    }

    // package.json files
    if dry_run {
        output::print_step("Would update: package.json files");
    } else {
        let mut pkgs = update_package_jsons(version)?;
        for pkg in &pkgs {
            let rel_path = pkg
                .strip_prefix(&platform::workspace_root()?)
                .unwrap_or(pkg);
            output::print_success(&format!("{}", rel_path.display()));
        }
        all_files.append(&mut pkgs);
    }

    // tauri.conf.json files
    if dry_run {
        output::print_step("Would update: tauri.conf.json files");
    } else {
        let mut tauris = update_tauri_confs(version)?;
        for tauri in &tauris {
            let rel_path = tauri
                .strip_prefix(&platform::workspace_root()?)
                .unwrap_or(tauri);
            output::print_success(&format!("{}", rel_path.display()));
        }
        all_files.append(&mut tauris);
    }

    // Release config
    if dry_run {
        output::print_step("Would update: .github/release-config.json");
    } else {
        let path = update_release_config(version)?;
        output::print_success(".github/release-config.json");
        all_files.push(path);
    }

    Ok(all_files)
}

/// Validate that all files were updated correctly
pub fn validate_updates(version: &str) -> Result<()> {
    output::print_header("Validating Updates");

    let workspace_root = platform::workspace_root()?;

    // Check workspace Cargo.toml
    let cargo_toml = workspace_root.join("Cargo.toml");
    let content = fs::read_file(&cargo_toml)?;
    let doc = content.parse::<toml_edit::DocumentMut>()?;

    let cargo_version = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Version not found in workspace Cargo.toml"))?;

    if cargo_version != version {
        output::print_error(&format!(
            "Cargo.toml version mismatch! Expected: {}, Actual: {}",
            version, cargo_version
        ));
        anyhow::bail!("Version validation failed");
    }

    output::print_success(&format!("Cargo.toml version = {}", cargo_version));

    // Check tauri.conf.json
    let tauri_conf = workspace_root.join("applications/desktop/src-tauri/tauri.conf.json");
    if tauri_conf.exists() {
        let content = fs::read_file(&tauri_conf)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let tauri_version = json
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Version not found in tauri.conf.json"))?;

        if tauri_version != version {
            output::print_error(&format!(
                "tauri.conf.json version mismatch! Expected: {}, Actual: {}",
                version, tauri_version
            ));
            output::print_warning("This will cause UI to show wrong version!");
            anyhow::bail!("Version validation failed");
        }

        output::print_success(&format!("tauri.conf.json version = {}", tauri_version));
    }

    // Check release-config.json
    let release_config = workspace_root.join(".github/release-config.json");
    if release_config.exists() {
        let content = fs::read_file(&release_config)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let release_version = json
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Version not found in release-config.json"))?;

        if release_version != version {
            output::print_error(&format!(
                "release-config.json version mismatch! Expected: {}, Actual: {}",
                version, release_version
            ));
            output::print_warning("This will cause wrong version in latest.json for auto-updates!");
            anyhow::bail!("Version validation failed");
        }

        output::print_success(&format!(
            "release-config.json version = {}",
            release_version
        ));
    }

    output::print_success("All files updated successfully");
    output::print_success("Version consistency verified");

    Ok(())
}
