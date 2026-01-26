use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InstallationMethod {
    AppImage,
    Deb,
    Rpm,
    Flatpak,
    Snap,
    Aur,
    Unknown,
}

impl InstallationMethod {
    /// Returns the update command for this installation method
    pub fn update_command(&self) -> Option<String> {
        match self {
            InstallationMethod::AppImage => None, // Auto-updates via Tauri updater
            InstallationMethod::Deb => {
                Some("sudo apt update && sudo apt upgrade soul-player".to_string())
            }
            InstallationMethod::Rpm => Some("sudo dnf upgrade soul-player".to_string()),
            InstallationMethod::Flatpak => {
                Some("flatpak update io.github.soulaudio.SoulPlayer".to_string())
            }
            InstallationMethod::Snap => Some("sudo snap refresh soul-player".to_string()),
            InstallationMethod::Aur => Some("yay -Syu soul-player".to_string()),
            InstallationMethod::Unknown => None,
        }
    }

    /// Returns true if this installation method supports Tauri auto-updates
    pub fn supports_auto_update(&self) -> bool {
        matches!(self, InstallationMethod::AppImage)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationInfo {
    pub method: InstallationMethod,
    pub update_command: Option<String>,
    pub supports_auto_update: bool,
}

/// Detect how the application was installed on Linux
///
/// Detection strategy:
/// 1. Check APPIMAGE environment variable (AppImage)
/// 2. Check if running in Flatpak sandbox
/// 3. Check if running in Snap confinement
/// 4. Check /proc/self/exe symlink for package manager hints
/// 5. Check common installation paths
#[cfg(target_os = "linux")]
pub fn detect_installation_method() -> InstallationInfo {
    use std::env;
    use std::fs;
    use std::path::Path;

    // 1. Check for AppImage (most reliable)
    if env::var("APPIMAGE").is_ok() {
        return InstallationInfo {
            method: InstallationMethod::AppImage,
            update_command: None,
            supports_auto_update: true,
        };
    }

    // 2. Check for Flatpak (FLATPAK_ID env var)
    if env::var("FLATPAK_ID").is_ok() {
        return InstallationInfo {
            method: InstallationMethod::Flatpak,
            update_command: Some("flatpak update io.github.soulaudio.SoulPlayer".to_string()),
            supports_auto_update: false,
        };
    }

    // 3. Check for Snap (SNAP env var or /snap path)
    if env::var("SNAP").is_ok() || env::var("SNAP_NAME").is_ok() {
        return InstallationInfo {
            method: InstallationMethod::Snap,
            update_command: Some("sudo snap refresh soul-player".to_string()),
            supports_auto_update: false,
        };
    }

    // 4. Check /proc/self/exe for installation path hints
    if let Ok(exe_path) = fs::read_link("/proc/self/exe") {
        let exe_str = exe_path.to_string_lossy();

        // Check for AppImage mount paths
        if exe_str.contains("/tmp/.mount_") || exe_str.contains("appimage") {
            return InstallationInfo {
                method: InstallationMethod::AppImage,
                update_command: None,
                supports_auto_update: true,
            };
        }

        // Check for Flatpak paths
        if exe_str.contains("/app/") || exe_str.contains("/.var/app/") {
            return InstallationInfo {
                method: InstallationMethod::Flatpak,
                update_command: Some("flatpak update io.github.soulaudio.SoulPlayer".to_string()),
                supports_auto_update: false,
            };
        }

        // Check for Snap paths
        if exe_str.contains("/snap/") {
            return InstallationInfo {
                method: InstallationMethod::Snap,
                update_command: Some("sudo snap refresh soul-player".to_string()),
                supports_auto_update: false,
            };
        }
    }

    // 5. Check for package manager installation markers
    // Check if dpkg knows about this package (DEB)
    if Path::new("/var/lib/dpkg/info/soul-player.list").exists() {
        return InstallationInfo {
            method: InstallationMethod::Deb,
            update_command: Some("sudo apt update && sudo apt upgrade soul-player".to_string()),
            supports_auto_update: false,
        };
    }

    // Check if rpm knows about this package (RPM)
    // Note: This is a heuristic - we check common rpm database paths
    if Path::new("/usr/bin/soul-player").exists() {
        // Check if we're on an RPM-based system
        if Path::new("/etc/redhat-release").exists()
            || Path::new("/etc/fedora-release").exists()
            || Path::new("/etc/centos-release").exists()
        {
            // Try to detect if installed via RPM by checking for rpm database
            if std::process::Command::new("rpm")
                .args(["-q", "soul-player"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return InstallationInfo {
                    method: InstallationMethod::Rpm,
                    update_command: Some("sudo dnf upgrade soul-player".to_string()),
                    supports_auto_update: false,
                };
            }
        }

        // Check for Arch Linux (AUR)
        if Path::new("/etc/arch-release").exists() {
            return InstallationInfo {
                method: InstallationMethod::Aur,
                update_command: Some("yay -Syu soul-player".to_string()),
                supports_auto_update: false,
            };
        }

        // If installed in /usr/bin but can't determine method, check package managers
        // Try DEB first (most common)
        if std::process::Command::new("dpkg")
            .args(["-l", "soul-player"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return InstallationInfo {
                method: InstallationMethod::Deb,
                update_command: Some("sudo apt update && sudo apt upgrade soul-player".to_string()),
                supports_auto_update: false,
            };
        }
    }

    // 6. Fallback: Unknown installation method
    InstallationInfo {
        method: InstallationMethod::Unknown,
        update_command: None,
        supports_auto_update: false,
    }
}

/// For non-Linux platforms, always return AppImage equivalent (supports auto-update)
#[cfg(not(target_os = "linux"))]
pub fn detect_installation_method() -> InstallationInfo {
    InstallationInfo {
        method: InstallationMethod::AppImage, // Equivalent to "standard install"
        update_command: None,
        supports_auto_update: true,
    }
}

#[tauri::command]
pub fn get_installation_info() -> InstallationInfo {
    let info = detect_installation_method();
    tracing::info!(
        "Detected installation method: {:?}, supports_auto_update: {}",
        info.method,
        info.supports_auto_update
    );
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_method_update_commands() {
        assert_eq!(InstallationMethod::AppImage.update_command(), None);
        assert!(InstallationMethod::Deb.update_command().is_some());
        assert!(InstallationMethod::Rpm.update_command().is_some());
        assert!(InstallationMethod::Flatpak.update_command().is_some());
        assert!(InstallationMethod::Snap.update_command().is_some());
        assert!(InstallationMethod::Aur.update_command().is_some());
        assert_eq!(InstallationMethod::Unknown.update_command(), None);
    }

    #[test]
    fn test_deb_update_command() {
        let command = InstallationMethod::Deb.update_command();
        assert_eq!(
            command,
            Some("sudo apt update && sudo apt upgrade soul-player".to_string())
        );
    }

    #[test]
    fn test_rpm_update_command() {
        let command = InstallationMethod::Rpm.update_command();
        assert_eq!(command, Some("sudo dnf upgrade soul-player".to_string()));
    }

    #[test]
    fn test_flatpak_update_command() {
        let command = InstallationMethod::Flatpak.update_command();
        assert_eq!(
            command,
            Some("flatpak update io.github.soulaudio.SoulPlayer".to_string())
        );
    }

    #[test]
    fn test_snap_update_command() {
        let command = InstallationMethod::Snap.update_command();
        assert_eq!(command, Some("sudo snap refresh soul-player".to_string()));
    }

    #[test]
    fn test_aur_update_command() {
        let command = InstallationMethod::Aur.update_command();
        assert_eq!(command, Some("yay -Syu soul-player".to_string()));
    }

    #[test]
    fn test_installation_method_auto_update_support() {
        assert!(InstallationMethod::AppImage.supports_auto_update());
        assert!(!InstallationMethod::Deb.supports_auto_update());
        assert!(!InstallationMethod::Rpm.supports_auto_update());
        assert!(!InstallationMethod::Flatpak.supports_auto_update());
        assert!(!InstallationMethod::Snap.supports_auto_update());
        assert!(!InstallationMethod::Aur.supports_auto_update());
        assert!(!InstallationMethod::Unknown.supports_auto_update());
    }

    #[test]
    fn test_installation_info_creation() {
        let info = InstallationInfo {
            method: InstallationMethod::Deb,
            update_command: Some("test command".to_string()),
            supports_auto_update: false,
        };

        assert_eq!(info.method, InstallationMethod::Deb);
        assert_eq!(info.update_command, Some("test command".to_string()));
        assert!(!info.supports_auto_update);
    }

    #[test]
    fn test_installation_info_appimage() {
        let info = InstallationInfo {
            method: InstallationMethod::AppImage,
            update_command: None,
            supports_auto_update: true,
        };

        assert_eq!(info.method, InstallationMethod::AppImage);
        assert_eq!(info.update_command, None);
        assert!(info.supports_auto_update);
    }

    #[test]
    fn test_installation_info_consistency() {
        // Test that each installation method's info matches its method's capabilities
        let methods = vec![
            InstallationMethod::AppImage,
            InstallationMethod::Deb,
            InstallationMethod::Rpm,
            InstallationMethod::Flatpak,
            InstallationMethod::Snap,
            InstallationMethod::Aur,
            InstallationMethod::Unknown,
        ];

        for method in methods {
            let info = InstallationInfo {
                method: method.clone(),
                update_command: method.update_command(),
                supports_auto_update: method.supports_auto_update(),
            };

            assert_eq!(info.update_command, method.update_command());
            assert_eq!(info.supports_auto_update, method.supports_auto_update());
        }
    }

    #[test]
    fn test_installation_method_serialization() {
        // Test that installation methods can be serialized/deserialized
        let methods = vec![
            InstallationMethod::AppImage,
            InstallationMethod::Deb,
            InstallationMethod::Rpm,
            InstallationMethod::Flatpak,
            InstallationMethod::Snap,
            InstallationMethod::Aur,
            InstallationMethod::Unknown,
        ];

        for method in methods {
            let info = InstallationInfo {
                method: method.clone(),
                update_command: method.update_command(),
                supports_auto_update: method.supports_auto_update(),
            };

            // Serialize and deserialize
            let json = serde_json::to_string(&info).expect("Failed to serialize");
            let deserialized: InstallationInfo =
                serde_json::from_str(&json).expect("Failed to deserialize");

            assert_eq!(deserialized.method, method);
            assert_eq!(deserialized.update_command, method.update_command());
            assert_eq!(
                deserialized.supports_auto_update,
                method.supports_auto_update()
            );
        }
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_non_linux_detection() {
        // On non-Linux platforms, should always return AppImage equivalent
        let info = detect_installation_method();
        assert_eq!(info.method, InstallationMethod::AppImage);
        assert_eq!(info.update_command, None);
        assert!(info.supports_auto_update);
    }

    #[test]
    fn test_installation_method_equality() {
        assert_eq!(InstallationMethod::AppImage, InstallationMethod::AppImage);
        assert_ne!(InstallationMethod::AppImage, InstallationMethod::Deb);
        assert_ne!(InstallationMethod::Deb, InstallationMethod::Rpm);
    }

    #[test]
    fn test_installation_info_with_none_command() {
        let info = InstallationInfo {
            method: InstallationMethod::Unknown,
            update_command: None,
            supports_auto_update: false,
        };

        assert_eq!(info.update_command, None);
    }

    #[test]
    fn test_installation_info_with_empty_command() {
        // This shouldn't happen in practice, but test it anyway
        let info = InstallationInfo {
            method: InstallationMethod::Deb,
            update_command: Some(String::new()),
            supports_auto_update: false,
        };

        assert_eq!(info.update_command, Some(String::new()));
    }
}
