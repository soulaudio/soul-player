//! Utility functions for soul-storage
//!
//! Common helpers used across multiple modules to reduce code duplication

/// Convert Unix timestamp to ISO 8601 string (RFC 3339 format)
///
/// # Arguments
///
/// * `timestamp` - Unix timestamp in seconds
///
/// # Returns
///
/// ISO 8601 formatted string, or empty string if conversion fails
///
/// # Example
///
/// ```rust
/// use soul_storage::utils::unix_to_iso8601;
///
/// let iso = unix_to_iso8601(1704067200); // 2024-01-01 00:00:00 UTC
/// assert!(!iso.is_empty());
/// assert!(iso.contains("2024"));
/// ```
pub fn unix_to_iso8601(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_to_iso8601_valid() {
        // Test a known timestamp: 2024-01-01 00:00:00 UTC
        let result = unix_to_iso8601(1704067200);
        assert!(result.contains("2024-01-01"));
    }

    #[test]
    fn test_unix_to_iso8601_invalid() {
        // Test an invalid timestamp (too large)
        let result = unix_to_iso8601(i64::MAX);
        assert_eq!(result, ""); // Should return empty string
    }

    #[test]
    fn test_unix_to_iso8601_zero() {
        // Test Unix epoch
        let result = unix_to_iso8601(0);
        assert!(result.contains("1970-01-01"));
    }
}
