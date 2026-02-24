//! Timeout wrapper for device checks to prevent indefinite hangs
//!
//! Audio services (CoreAudio, PulseAudio, WASAPI) can become unresponsive,
//! causing device enumeration calls to hang indefinitely. This module provides
//! a timeout wrapper that ensures device checks never block longer than a
//! configurable timeout (default: 5 seconds).
//!
//! # Architecture
//!
//! Uses a separate thread to perform the device check, with a channel-based
//! timeout mechanism. If the check doesn't complete within the timeout,
//! returns None to indicate a timeout occurred.
//!
//! # Example
//!
//! ```ignore
//! use soul_audio_desktop::device_check_with_timeout_sync;
//!
//! match device_check_with_timeout_sync(|| {
//!     // This could potentially hang if audio service is frozen
//!     my_device_check_function()
//! }) {
//!     Some(Ok(result)) => println!("Check succeeded: {:?}", result),
//!     Some(Err(e)) => println!("Check failed: {}", e),
//!     None => println!("Check timed out after 5 seconds"),
//! }
//! ```

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Default timeout for device checks (5 seconds)
///
/// This is long enough to handle slow audio drivers but short enough
/// to prevent user-perceived hangs.
pub const DEFAULT_DEVICE_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout configuration for device checks
#[derive(Debug, Clone, Copy)]
pub struct TimeoutConfig {
    /// Timeout duration for device checks
    pub timeout: Duration,
    /// Maximum consecutive timeouts before giving up
    pub max_consecutive_timeouts: u32,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_DEVICE_CHECK_TIMEOUT,
            max_consecutive_timeouts: 3,
        }
    }
}

/// Wraps a device check function with a timeout
///
/// Executes the provided function in a separate thread and returns:
/// - `Some(Ok(T))` if the check succeeds within the timeout
/// - `Some(Err(E))` if the check fails within the timeout
/// - `None` if the check doesn't complete within the timeout
///
/// # Type Parameters
/// - `F`: Function to execute (must be `Send + 'static`)
/// - `T`: Success result type (must be `Send + 'static`)
/// - `E`: Error result type (must be `Send + 'static`)
///
/// # Arguments
/// - `check_fn`: Function to execute with timeout protection
///
/// # Returns
/// - `Some(result)` if check completed within timeout
/// - `None` if check timed out
///
/// # Example
///
/// ```no_run
/// use soul_audio_desktop::device_check_with_timeout_sync;
///
/// let result = device_check_with_timeout_sync(|| {
///     // Potentially slow device check
///     Ok::<_, String>(42)
/// });
///
/// match result {
///     Some(Ok(value)) => println!("Got value: {}", value),
///     Some(Err(e)) => println!("Error: {}", e),
///     None => println!("Timeout!"),
/// }
/// ```
pub fn device_check_with_timeout_sync<F, T, E>(check_fn: F) -> Option<Result<T, E>>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    device_check_with_timeout_sync_custom(check_fn, DEFAULT_DEVICE_CHECK_TIMEOUT)
}

/// Wraps a device check function with a custom timeout
///
/// Same as `device_check_with_timeout_sync` but allows specifying a custom timeout duration.
///
/// # Arguments
/// - `check_fn`: Function to execute with timeout protection
/// - `timeout`: Custom timeout duration
///
/// # Returns
/// - `Some(result)` if check completed within timeout
/// - `None` if check timed out
pub fn device_check_with_timeout_sync_custom<F, T, E>(
    check_fn: F,
    timeout: Duration,
) -> Option<Result<T, E>>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    // Spawn thread to execute the check
    thread::spawn(move || {
        let result = check_fn();
        // Ignore send errors (receiver may have timed out and dropped)
        let _ = tx.send(result);
    });

    // Wait for result with timeout
    rx.recv_timeout(timeout).ok()
}

/// Tracks consecutive timeout occurrences
///
/// Helps detect when an audio service is persistently unresponsive
/// vs. experiencing transient issues.
///
/// # Example
///
/// ```
/// use soul_audio_desktop::TimeoutTracker;
///
/// let mut tracker = TimeoutTracker::new();
///
/// // Simulate timeouts
/// for _ in 0..3 {
///     tracker.record_timeout();
/// }
///
/// if tracker.is_max_reached() {
///     println!("Audio service appears to be frozen - giving up");
/// }
/// ```
pub struct TimeoutTracker {
    consecutive_timeouts: u32,
    config: TimeoutConfig,
}

impl TimeoutTracker {
    /// Create a new timeout tracker with default config
    pub fn new() -> Self {
        Self {
            consecutive_timeouts: 0,
            config: TimeoutConfig::default(),
        }
    }

    /// Create a new timeout tracker with custom config
    pub fn with_config(config: TimeoutConfig) -> Self {
        Self {
            consecutive_timeouts: 0,
            config,
        }
    }

    /// Record a timeout occurrence
    pub fn record_timeout(&mut self) {
        self.consecutive_timeouts = self.consecutive_timeouts.saturating_add(1);
    }

    /// Record a successful check (resets timeout counter)
    pub fn record_success(&mut self) {
        self.consecutive_timeouts = 0;
    }

    /// Check if maximum consecutive timeouts reached
    pub fn is_max_reached(&self) -> bool {
        self.consecutive_timeouts >= self.config.max_consecutive_timeouts
    }

    /// Get the current consecutive timeout count
    pub fn get_count(&self) -> u32 {
        self.consecutive_timeouts
    }

    /// Reset the tracker
    pub fn reset(&mut self) {
        self.consecutive_timeouts = 0;
    }
}

impl Default for TimeoutTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successful_check() {
        let result = device_check_with_timeout_sync(|| Ok::<i32, String>(42));
        assert_eq!(result, Some(Ok(42)));
    }

    #[test]
    fn test_failed_check() {
        let result = device_check_with_timeout_sync(|| Err::<i32, String>("error".to_string()));
        assert_eq!(result, Some(Err("error".to_string())));
    }

    #[test]
    fn test_timeout() {
        let result = device_check_with_timeout_sync_custom(
            || {
                thread::sleep(Duration::from_secs(10));
                Ok::<i32, String>(42)
            },
            Duration::from_millis(100),
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_timeout_tracker() {
        let mut tracker = TimeoutTracker::new();
        assert_eq!(tracker.get_count(), 0);
        assert!(!tracker.is_max_reached());

        tracker.record_timeout();
        assert_eq!(tracker.get_count(), 1);
        assert!(!tracker.is_max_reached());

        tracker.record_timeout();
        tracker.record_timeout();
        assert_eq!(tracker.get_count(), 3);
        assert!(tracker.is_max_reached());

        tracker.record_success();
        assert_eq!(tracker.get_count(), 0);
        assert!(!tracker.is_max_reached());
    }

    #[test]
    fn test_timeout_tracker_with_custom_config() {
        let config = TimeoutConfig {
            timeout: Duration::from_secs(5),
            max_consecutive_timeouts: 5,
        };
        let mut tracker = TimeoutTracker::with_config(config);

        for _ in 0..4 {
            tracker.record_timeout();
        }
        assert!(!tracker.is_max_reached());

        tracker.record_timeout();
        assert!(tracker.is_max_reached());
    }
}
