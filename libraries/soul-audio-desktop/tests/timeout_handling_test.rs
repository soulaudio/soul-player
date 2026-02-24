//! Timeout handling edge case tests
//!
//! Tests the timeout handling infrastructure to ensure it correctly:
//! - Tracks consecutive timeout occurrences
//! - Recovers from timeouts after successful checks
//! - Handles saturation at maximum timeout count
//! - Doesn't interfere with active playback
//!
//! These tests verify the `TimeoutTracker` state machine and the timeout
//! wrapper functions used for device enumeration.

#![allow(clippy::doc_markdown)]

use soul_audio_desktop::{
    device_check_with_timeout_sync, device_check_with_timeout_sync_custom, TimeoutConfig,
    TimeoutTracker, DEFAULT_DEVICE_CHECK_TIMEOUT,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// Test basic timeout tracker state transitions
#[test]
fn test_timeout_tracker_basic_state_machine() {
    let mut tracker = TimeoutTracker::new();

    // Initial state
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());

    // First timeout
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 1);
    assert!(!tracker.is_max_reached());

    // Second timeout
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 2);
    assert!(!tracker.is_max_reached());

    // Third timeout - should reach max (default is 3)
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 3);
    assert!(tracker.is_max_reached());

    // Success resets the counter
    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());
}

/// Test timeout tracker with custom configuration
#[test]
fn test_timeout_tracker_custom_config() {
    let config = TimeoutConfig {
        timeout: Duration::from_secs(5),
        max_consecutive_timeouts: 5,
    };
    let mut tracker = TimeoutTracker::with_config(config);

    // Should not reach max until 5 timeouts
    for i in 0..4 {
        tracker.record_timeout();
        assert_eq!(tracker.get_count(), i + 1);
        assert!(!tracker.is_max_reached());
    }

    // Fifth timeout reaches max
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 5);
    assert!(tracker.is_max_reached());
}

/// Test repeated timeout/recovery sequence
#[test]
fn test_timeout_recovery_sequence() {
    let config = TimeoutConfig {
        timeout: Duration::from_secs(5),
        max_consecutive_timeouts: 3,
    };
    let mut tracker = TimeoutTracker::with_config(config);

    // First cycle: 3 timeouts, then success
    for _ in 0..3 {
        tracker.record_timeout();
    }
    assert!(tracker.is_max_reached());

    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());

    // Second cycle: 2 timeouts, then success
    tracker.record_timeout();
    tracker.record_timeout();
    assert!(!tracker.is_max_reached());

    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);

    // Third cycle: 3 timeouts again
    for _ in 0..3 {
        tracker.record_timeout();
    }
    assert!(tracker.is_max_reached());
}

/// Test timeout tracker saturation behavior
#[test]
fn test_timeout_tracker_saturation() {
    let mut tracker = TimeoutTracker::new();

    // Record many more timeouts than the maximum
    for _ in 0..10 {
        tracker.record_timeout();
    }

    // Should saturate at max, not overflow
    assert!(tracker.is_max_reached());
    assert!(tracker.get_count() >= 3); // At least the max count

    // Should still be able to reset
    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());
}

/// Test timeout tracker reset functionality
#[test]
fn test_timeout_tracker_reset() {
    let mut tracker = TimeoutTracker::new();

    // Build up some timeouts
    tracker.record_timeout();
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 2);

    // Reset should clear count
    tracker.reset();
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());

    // Should work after reset
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 1);
}

/// Test timeout wrapper with successful check
#[test]
fn test_timeout_wrapper_success() {
    let result = device_check_with_timeout_sync(|| Ok::<i32, String>(42));

    assert_eq!(result, Some(Ok(42)));
}

/// Test timeout wrapper with failed check
#[test]
fn test_timeout_wrapper_failure() {
    let result = device_check_with_timeout_sync(|| Err::<i32, String>("test error".to_string()));

    assert_eq!(result, Some(Err("test error".to_string())));
}

/// Test timeout wrapper with actual timeout
#[test]
fn test_timeout_wrapper_timeout() {
    // Use a very short timeout to keep test fast
    let result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(500));
            Ok::<i32, String>(42)
        },
        Duration::from_millis(50),
    );

    assert_eq!(result, None, "Check should have timed out");
}

/// Test timeout wrapper with fast completion
#[test]
fn test_timeout_wrapper_fast_completion() {
    let result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(10));
            Ok::<i32, String>(42)
        },
        Duration::from_millis(100),
    );

    assert_eq!(
        result,
        Some(Ok(42)),
        "Fast check should complete before timeout"
    );
}

/// Test timeout wrapper with custom timeout value
#[test]
fn test_timeout_wrapper_custom_timeout() {
    // Short timeout should fail
    let short_result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(100));
            Ok::<i32, String>(42)
        },
        Duration::from_millis(50),
    );
    assert_eq!(short_result, None);

    // Longer timeout should succeed — use a generous timeout vs sleep to avoid flakiness under load
    let long_result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(50));
            Ok::<i32, String>(42)
        },
        Duration::from_millis(500),
    );
    assert_eq!(long_result, Some(Ok(42)));
}

/// Test that default timeout is reasonable
#[test]
fn test_default_timeout_value() {
    // Default should be 5 seconds
    assert_eq!(DEFAULT_DEVICE_CHECK_TIMEOUT, Duration::from_secs(5));

    // Verify it's actually used by the default function
    let start = std::time::Instant::now();
    let result = device_check_with_timeout_sync(|| {
        thread::sleep(Duration::from_secs(10));
        Ok::<(), String>(())
    });
    let elapsed = start.elapsed();

    assert_eq!(result, None);
    // Should timeout around 5 seconds (allow some margin)
    assert!(
        elapsed >= Duration::from_secs(4) && elapsed <= Duration::from_secs(6),
        "Timeout should be around 5 seconds, was {:?}",
        elapsed
    );
}

/// Test concurrent timeouts don't interfere with each other
#[test]
fn test_concurrent_timeout_wrappers() {
    let success_count = Arc::new(AtomicU32::new(0));
    let timeout_count = Arc::new(AtomicU32::new(0));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let success = success_count.clone();
            let timeout = timeout_count.clone();

            thread::spawn(move || {
                let result = device_check_with_timeout_sync_custom(
                    move || {
                        // Half should succeed, half should timeout
                        if i % 2 == 0 {
                            thread::sleep(Duration::from_millis(20));
                            Ok::<i32, String>(i)
                        } else {
                            thread::sleep(Duration::from_millis(200));
                            Ok::<i32, String>(i)
                        }
                    },
                    Duration::from_millis(50),
                );

                match result {
                    Some(Ok(_)) => success.fetch_add(1, Ordering::SeqCst),
                    None => timeout.fetch_add(1, Ordering::SeqCst),
                    _ => 0,
                };
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have 5 successes and 5 timeouts
    assert_eq!(success_count.load(Ordering::SeqCst), 5);
    assert_eq!(timeout_count.load(Ordering::SeqCst), 5);
}

/// Test timeout tracker with alternating success/failure
#[test]
fn test_timeout_tracker_alternating_pattern() {
    let mut tracker = TimeoutTracker::new();

    // Pattern: timeout, success, timeout, success
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 1);

    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);

    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 1);

    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);

    // Never should reach max with this pattern
    assert!(!tracker.is_max_reached());
}

/// Test timeout tracker edge case: max_consecutive_timeouts = 1
#[test]
fn test_timeout_tracker_max_one() {
    let config = TimeoutConfig {
        timeout: Duration::from_secs(5),
        max_consecutive_timeouts: 1,
    };
    let mut tracker = TimeoutTracker::with_config(config);

    // Single timeout should reach max
    tracker.record_timeout();
    assert_eq!(tracker.get_count(), 1);
    assert!(tracker.is_max_reached());

    // Success should reset
    tracker.record_success();
    assert_eq!(tracker.get_count(), 0);
    assert!(!tracker.is_max_reached());
}

/// Test timeout tracker edge case: max_consecutive_timeouts = 0
#[test]
fn test_timeout_tracker_max_zero() {
    let config = TimeoutConfig {
        timeout: Duration::from_secs(5),
        max_consecutive_timeouts: 0,
    };
    let tracker = TimeoutTracker::with_config(config);

    // Should already be at max with 0 count
    assert!(tracker.is_max_reached());
}

/// Test that timeouts don't prevent subsequent successful operations
#[test]
fn test_timeout_does_not_affect_future_calls() {
    // First call times out
    let timeout_result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(200));
            Ok::<i32, String>(1)
        },
        Duration::from_millis(50),
    );
    assert_eq!(timeout_result, None);

    // Second call should succeed independently — generous timeout vs sleep to avoid flakiness
    let success_result = device_check_with_timeout_sync_custom(
        || {
            thread::sleep(Duration::from_millis(10));
            Ok::<i32, String>(2)
        },
        Duration::from_millis(500),
    );
    assert_eq!(success_result, Some(Ok(2)));
}

/// Test timeout wrapper with panic in check function
#[test]
fn test_timeout_wrapper_with_panic() {
    // Note: This test verifies that panics in the spawned thread
    // don't crash the main thread. The wrapper will timeout instead.
    let result = device_check_with_timeout_sync_custom(
        || -> Result<i32, String> { panic!("test panic") },
        Duration::from_millis(100),
    );

    // The panic happens in the spawned thread, so we get None (timeout)
    // The main thread is not affected - it just sees the channel close
    assert_eq!(
        result, None,
        "Panic in spawned thread should result in timeout"
    );
}

/// Test timeout tracker should_retry logic
#[test]
fn test_timeout_tracker_should_retry() {
    let config = TimeoutConfig {
        timeout: Duration::from_secs(5),
        max_consecutive_timeouts: 3,
    };
    let mut tracker = TimeoutTracker::with_config(config);

    // Should retry for first few timeouts
    for i in 0..3 {
        assert!(
            !tracker.is_max_reached(),
            "Should retry at count {}",
            tracker.get_count()
        );
        tracker.record_timeout();
        assert_eq!(tracker.get_count(), i + 1);
    }

    // After max timeouts, should not retry
    assert!(tracker.is_max_reached(), "Should stop retrying after max");

    // After success, should retry again
    tracker.record_success();
    assert!(
        !tracker.is_max_reached(),
        "Should retry after successful recovery"
    );
}

/// Test timeout behavior with zero-duration timeout
#[test]
fn test_zero_timeout() {
    let result = device_check_with_timeout_sync_custom(
        || {
            // Even instant operation should timeout with zero duration
            Ok::<i32, String>(42)
        },
        Duration::from_millis(0),
    );

    // Zero timeout should always timeout (recv_timeout immediately returns timeout)
    assert_eq!(result, None);
}

/// Integration test: TimeoutTracker simulating device monitoring scenario
#[test]
fn test_timeout_tracker_device_monitoring_scenario() {
    let mut tracker = TimeoutTracker::new();
    let device_available = Arc::new(AtomicBool::new(false));

    // Simulate device monitoring loop
    for iteration in 0..10 {
        let available = device_available.load(Ordering::SeqCst);

        if available {
            tracker.record_success();
            assert_eq!(tracker.get_count(), 0);
        } else {
            tracker.record_timeout();

            if tracker.is_max_reached() {
                // In real code, this would stop monitoring or alert user
                assert!(
                    tracker.get_count() >= 3,
                    "Should have at least 3 timeouts when max reached"
                );
                // Simulate device recovery
                device_available.store(true, Ordering::SeqCst);
            }
        }

        // Simulate device becoming unavailable after a while
        if iteration == 5 {
            device_available.store(false, Ordering::SeqCst);
        }
    }

    // Should end with device available and tracker reset
    assert!(device_available.load(Ordering::SeqCst));
}

/// Test timeout wrapper respects thread boundaries
#[test]
fn test_timeout_wrapper_thread_isolation() {
    let outer_value = Arc::new(AtomicU32::new(0));
    let inner_value = outer_value.clone();

    let result = device_check_with_timeout_sync_custom(
        move || {
            inner_value.store(42, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(10));
            Ok::<(), String>(())
        },
        Duration::from_millis(50),
    );

    assert_eq!(result, Some(Ok(())));
    // Value should be updated even though it ran in different thread
    assert_eq!(outer_value.load(Ordering::SeqCst), 42);
}

/// Test that multiple timeout trackers are independent
#[test]
fn test_multiple_timeout_trackers_independent() {
    let mut tracker1 = TimeoutTracker::new();
    let mut tracker2 = TimeoutTracker::new();

    tracker1.record_timeout();
    tracker1.record_timeout();
    assert_eq!(tracker1.get_count(), 2);
    assert_eq!(tracker2.get_count(), 0);

    tracker2.record_timeout();
    assert_eq!(tracker1.get_count(), 2);
    assert_eq!(tracker2.get_count(), 1);

    tracker1.record_success();
    assert_eq!(tracker1.get_count(), 0);
    assert_eq!(tracker2.get_count(), 1);
}
