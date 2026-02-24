use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Brew,       // macOS
    Apt,        // Debian/Ubuntu
    Pacman,     // Arch Linux
    Dnf,        // Fedora/RHEL
    Winget,     // Windows
    Chocolatey, // Windows
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            Platform::Linux
        }
    }

    pub fn is_windows(self) -> bool {
        matches!(self, Platform::Windows)
    }

    pub fn is_macos(self) -> bool {
        matches!(self, Platform::MacOS)
    }

    pub fn is_linux(self) -> bool {
        matches!(self, Platform::Linux)
    }

    pub fn executable_extension(self) -> &'static str {
        match self {
            Platform::Windows => ".exe",
            _ => "",
        }
    }

    pub fn path_separator(self) -> char {
        match self {
            Platform::Windows => ';',
            _ => ':',
        }
    }
}

impl PackageManager {
    pub fn detect() -> Result<Option<Self>> {
        let platform = Platform::current();

        match platform {
            Platform::MacOS => {
                // Check for Homebrew
                if which::which("brew").is_ok() {
                    Ok(Some(PackageManager::Brew))
                } else {
                    Ok(None)
                }
            }
            Platform::Linux => {
                // Check for package managers in order of preference
                if which::which("apt").is_ok() {
                    Ok(Some(PackageManager::Apt))
                } else if which::which("pacman").is_ok() {
                    Ok(Some(PackageManager::Pacman))
                } else if which::which("dnf").is_ok() {
                    Ok(Some(PackageManager::Dnf))
                } else {
                    Ok(None)
                }
            }
            Platform::Windows => {
                // Prefer winget over chocolatey
                if which::which("winget").is_ok() {
                    Ok(Some(PackageManager::Winget))
                } else if which::which("choco").is_ok() {
                    Ok(Some(PackageManager::Chocolatey))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn install_command(self, packages: &[&str]) -> Vec<String> {
        let mut cmd = vec![];

        match self {
            PackageManager::Brew => {
                cmd.push("brew".to_string());
                cmd.push("install".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
            }
            PackageManager::Apt => {
                cmd.push("sudo".to_string());
                cmd.push("apt".to_string());
                cmd.push("install".to_string());
                cmd.push("-y".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
            }
            PackageManager::Pacman => {
                cmd.push("sudo".to_string());
                cmd.push("pacman".to_string());
                cmd.push("-S".to_string());
                cmd.push("--noconfirm".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
            }
            PackageManager::Dnf => {
                cmd.push("sudo".to_string());
                cmd.push("dnf".to_string());
                cmd.push("install".to_string());
                cmd.push("-y".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
            }
            PackageManager::Winget => {
                cmd.push("winget".to_string());
                cmd.push("install".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
            }
            PackageManager::Chocolatey => {
                cmd.push("choco".to_string());
                cmd.push("install".to_string());
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd.push("-y".to_string());
            }
        }

        cmd
    }
}

/// Get the workspace root directory
pub fn workspace_root() -> Result<std::path::PathBuf> {
    let output = std::process::Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to locate workspace root");
    }

    let cargo_toml_path = String::from_utf8(output.stdout)?.trim().to_string();
    let workspace_root = std::path::Path::new(&cargo_toml_path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?
        .to_path_buf();

    Ok(workspace_root)
}
