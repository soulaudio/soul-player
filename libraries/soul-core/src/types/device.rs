/// Device domain types for multi-device sync
use serde::{Deserialize, Serialize};

/// Device type indicating the platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Web,
    Desktop,
    Mobile,
}

impl DeviceType {
    /// Convert to string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "web" => Some(Self::Web),
            "desktop" => Some(Self::Desktop),
            "mobile" => Some(Self::Mobile),
            _ => None,
        }
    }
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A connected device
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier (UUID)
    pub id: String,

    /// Owner user ID
    pub user_id: String,

    /// Display name (e.g., "Chrome on Windows", "Desktop App")
    pub name: String,

    /// Device platform type
    pub device_type: DeviceType,

    /// Last activity timestamp (Unix epoch seconds)
    pub last_seen_at: i64,

    /// Device registration timestamp (Unix epoch seconds)
    pub created_at: i64,
}

/// Request to register a new device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDevice {
    /// Display name for the device
    pub name: String,

    /// Device platform type
    pub device_type: DeviceType,
}
