//! Timestamp conversion utilities
//!
//! Provides centralized helpers for converting between Unix timestamps and DateTime types.
//! This reduces duplication and ensures consistent timestamp handling across the codebase.

use chrono::{DateTime, Utc};

/// Get the current Unix timestamp (seconds since epoch)
///
/// # Example
///
/// ```rust
/// use soul_storage::utils::time::now_timestamp;
///
/// let now = now_timestamp();
/// assert!(now > 0);
/// ```
#[inline]
pub fn now_timestamp() -> i64 {
    Utc::now().timestamp()
}

/// Convert a Unix timestamp to an ISO 8601 string
///
/// Returns an empty string if the timestamp is invalid.
///
/// # Arguments
///
/// * `timestamp` - Unix timestamp (seconds since epoch)
///
/// # Example
///
/// ```rust
/// use soul_storage::utils::time::timestamp_to_iso8601;
///
/// let iso = timestamp_to_iso8601(1609459200); // 2021-01-01 00:00:00 UTC
/// assert_eq!(iso, "2021-01-01T00:00:00+00:00");
///
/// let invalid = timestamp_to_iso8601(-1);
/// assert_eq!(invalid, "");
/// ```
#[inline]
pub fn timestamp_to_iso8601(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

/// Convert a Unix timestamp to a DateTime<Utc>
///
/// Returns None if the timestamp is invalid.
///
/// # Arguments
///
/// * `timestamp` - Unix timestamp (seconds since epoch)
///
/// # Example
///
/// ```rust
/// use soul_storage::utils::time::timestamp_to_datetime;
///
/// let dt = timestamp_to_datetime(1609459200);
/// assert!(dt.is_some());
///
/// let invalid = timestamp_to_datetime(-1);
/// assert!(invalid.is_none());
/// ```
#[inline]
pub fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_timestamp() {
        let now = now_timestamp();
        // Should be a reasonable Unix timestamp (after 2020-01-01)
        assert!(now > 1577836800);
    }

    #[test]
    fn test_timestamp_to_iso8601() {
        // Valid timestamp
        let iso = timestamp_to_iso8601(1609459200);
        assert_eq!(iso, "2021-01-01T00:00:00+00:00");

        // Zero timestamp (epoch)
        let epoch = timestamp_to_iso8601(0);
        assert_eq!(epoch, "1970-01-01T00:00:00+00:00");

        // Invalid timestamp
        let invalid = timestamp_to_iso8601(i64::MAX);
        assert_eq!(invalid, "");
    }

    #[test]
    fn test_timestamp_to_datetime() {
        // Valid timestamp
        let dt = timestamp_to_datetime(1609459200);
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().year(), 2021);

        // Zero timestamp
        let epoch = timestamp_to_datetime(0);
        assert!(epoch.is_some());

        // Invalid timestamp
        let invalid = timestamp_to_datetime(i64::MAX);
        assert!(invalid.is_none());
    }
}
