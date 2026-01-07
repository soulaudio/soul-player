use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum DeepLinkAction {
    PlayTrack { id: i64 },
    OpenPlaylist { id: i64 },
    OpenLibrary,
    Search { query: String },
}

/// Parse a deep link URL into an action
///
/// Supported patterns:
/// - soul://play/track/{id}
/// - soul://playlist/{id}
/// - soul://library
/// - soul://search?q={query}
pub fn parse_deep_link(url: &str) -> Result<DeepLinkAction, String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    if parsed.scheme() != "soul" {
        return Err("Invalid scheme, expected 'soul://'".to_string());
    }

    match parsed.host_str() {
        Some("play") => {
            let path = parsed.path().trim_start_matches('/');
            if let Some(id_str) = path.strip_prefix("track/") {
                let id = id_str
                    .parse::<i64>()
                    .map_err(|_| "Invalid track ID".to_string())?;
                Ok(DeepLinkAction::PlayTrack { id })
            } else {
                Err("Invalid play path, expected 'soul://play/track/{id}'".to_string())
            }
        }
        Some("playlist") => {
            let id = parsed
                .path()
                .trim_start_matches('/')
                .parse::<i64>()
                .map_err(|_| "Invalid playlist ID".to_string())?;
            Ok(DeepLinkAction::OpenPlaylist { id })
        }
        Some("library") => Ok(DeepLinkAction::OpenLibrary),
        Some("search") => {
            let query = parsed
                .query_pairs()
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.to_string())
                .ok_or("Missing query parameter 'q'".to_string())?;
            Ok(DeepLinkAction::Search { query })
        }
        _ => Err(format!("Unknown host: {:?}", parsed.host_str())),
    }
}

/// Setup deep link handling
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_deep_link::DeepLinkExt;

    app.deep_link().register_all()?;

    let app_handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(url) = event.urls().first() {
                let url_str = url.as_str();
                if let Ok(action) = parse_deep_link(url_str) {
                    // Bring window to front
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }

                    // Emit to frontend
                    let _ = app.emit("deep-link", &action);
                }
            }
        });
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_play_track() {
        let result = parse_deep_link("soul://play/track/123").unwrap();
        match result {
            DeepLinkAction::PlayTrack { id } => assert_eq!(id, 123),
            _ => panic!("Expected PlayTrack"),
        }
    }

    #[test]
    fn test_parse_playlist() {
        let result = parse_deep_link("soul://playlist/456").unwrap();
        match result {
            DeepLinkAction::OpenPlaylist { id } => assert_eq!(id, 456),
            _ => panic!("Expected OpenPlaylist"),
        }
    }

    #[test]
    fn test_parse_library() {
        let result = parse_deep_link("soul://library").unwrap();
        matches!(result, DeepLinkAction::OpenLibrary);
    }

    #[test]
    fn test_parse_search() {
        let result = parse_deep_link("soul://search?q=test%20query").unwrap();
        match result {
            DeepLinkAction::Search { query } => assert_eq!(query, "test query"),
            _ => panic!("Expected Search"),
        }
    }

    #[test]
    fn test_parse_invalid_scheme() {
        let result = parse_deep_link("http://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid scheme"));
    }

    #[test]
    fn test_parse_invalid_track_id() {
        let result = parse_deep_link("soul://play/track/invalid");
        assert!(result.is_err());
    }
}
