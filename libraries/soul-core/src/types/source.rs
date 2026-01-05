//! Source types for multi-source architecture
//!
//! Supports local files + multiple remote servers with one active server at a time.

use serde::{Deserialize, Serialize};

pub type SourceId = i64;

/// A source of music (local files or remote server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub name: String,
    pub source_type: SourceType,
    pub config: SourceConfig,
    pub is_active: bool,
    pub is_online: bool,
    pub last_sync_at: Option<String>,
}

/// Type of source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Local,
    Server,
}

/// Source configuration (discriminated union based on type)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceConfig {
    Local,
    Server {
        url: String,
        username: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
}

/// Data for creating a new source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSource {
    pub name: String,
    pub config: SourceConfig,
}
