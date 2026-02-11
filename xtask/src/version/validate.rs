use anyhow::Result;

use crate::util::{output, platform, validation};

/// Show the current version from workspace Cargo.toml
pub fn show_current() -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let cargo_toml_path = workspace_root.join("Cargo.toml");

    let content = std::fs::read_to_string(&cargo_toml_path)?;

    // Parse TOML to find workspace version
    let doc = content.parse::<toml_edit::DocumentMut>()?;

    let version = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Version not found in workspace Cargo.toml"))?;

    output::print_header("Current Version");
    output::print_success(&format!("Workspace version: {}", version));

    Ok(())
}

/// Validate a semantic version string
pub fn validate(version: &str) -> Result<()> {
    output::print_header("Validating Version");
    output::print_info(&format!("Version: {}", version));

    match validation::validate_semver(version) {
        Ok(_) => {
            output::print_success("Valid semantic version format");

            // Also parse and show components
            let semver = validation::SemVer::parse(version)?;
            println!();
            output::print_info(&format!("  Major: {}", semver.major));
            output::print_info(&format!("  Minor: {}", semver.minor));
            output::print_info(&format!("  Patch: {}", semver.patch));

            if let Some(ref prerelease) = semver.prerelease {
                output::print_info(&format!("  Pre-release: {}", prerelease));
            }

            Ok(())
        }
        Err(e) => {
            output::print_error(&format!("Invalid version: {}", e));
            println!();
            output::print_info("Expected format: X.Y.Z (e.g., 0.1.0)");
            output::print_info("Or with pre-release: X.Y.Z-alpha.1, X.Y.Z-beta.1, X.Y.Z-rc.1");

            Err(e)
        }
    }
}
