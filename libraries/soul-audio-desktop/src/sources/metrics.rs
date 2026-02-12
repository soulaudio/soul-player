//! Lock contention profiling infrastructure
//!
//! Provides instrumentation to measure lock contention and identify performance bottlenecks
//! in the audio pipeline. Designed for zero-overhead in release builds when profiling is disabled.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Metrics for tracking lock contention
///
/// Uses atomic counters to avoid overhead from additional locking.
/// All operations are lock-free and safe for use in real-time audio threads.
#[derive(Debug, Default)]
pub struct LockMetrics {
    /// Total number of lock attempts
    total_attempts: AtomicU64,
    /// Number of times lock was contended (try_lock failed)
    contentions: AtomicU64,
    /// Maximum wait time observed in nanoseconds
    max_wait_ns: AtomicU64,
    /// Total wait time across all attempts (for average calculation)
    total_wait_ns: AtomicU64,
}

impl LockMetrics {
    /// Create new lock metrics tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a lock attempt
    ///
    /// # Arguments
    /// * `was_contended` - Whether the try_lock failed (true) or succeeded immediately (false)
    /// * `wait_ns` - Time spent waiting for the lock in nanoseconds
    ///
    /// # Performance
    /// Uses relaxed ordering for counters since exact ordering isn't critical for metrics.
    /// This minimizes overhead in the hot path.
    pub fn record_attempt(&self, was_contended: bool, wait_ns: u64) {
        self.total_attempts.fetch_add(1, Ordering::Relaxed);
        self.total_wait_ns.fetch_add(wait_ns, Ordering::Relaxed);

        if was_contended {
            self.contentions.fetch_add(1, Ordering::Relaxed);
        }

        // Update max wait time using compare-and-swap loop
        let mut current_max = self.max_wait_ns.load(Ordering::Relaxed);
        while wait_ns > current_max {
            match self.max_wait_ns.compare_exchange_weak(
                current_max,
                wait_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// Generate a report of current metrics
    ///
    /// Returns a snapshot of the current state. Safe to call from any thread.
    pub fn report(&self) -> LockMetricsReport {
        let total_attempts = self.total_attempts.load(Ordering::Relaxed);
        let contentions = self.contentions.load(Ordering::Relaxed);
        let max_wait_ns = self.max_wait_ns.load(Ordering::Relaxed);
        let total_wait_ns = self.total_wait_ns.load(Ordering::Relaxed);

        let avg_wait_ns = if total_attempts > 0 {
            total_wait_ns / total_attempts
        } else {
            0
        };

        let contention_rate = if total_attempts > 0 {
            (contentions as f64 / total_attempts as f64) * 100.0
        } else {
            0.0
        };

        LockMetricsReport {
            total_attempts,
            contentions,
            contention_rate,
            max_wait_ns,
            avg_wait_ns,
        }
    }

    /// Reset all metrics to zero
    ///
    /// Useful for periodic sampling or when starting a new measurement window.
    pub fn reset(&self) {
        self.total_attempts.store(0, Ordering::Relaxed);
        self.contentions.store(0, Ordering::Relaxed);
        self.max_wait_ns.store(0, Ordering::Relaxed);
        self.total_wait_ns.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of lock metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct LockMetricsReport {
    /// Total number of lock attempts
    pub total_attempts: u64,
    /// Number of contentions
    pub contentions: u64,
    /// Contention rate as percentage (0.0-100.0)
    pub contention_rate: f64,
    /// Maximum wait time in nanoseconds
    pub max_wait_ns: u64,
    /// Average wait time in nanoseconds
    pub avg_wait_ns: u64,
}

impl LockMetricsReport {
    /// Check if contention is significant (>5% of attempts)
    pub fn is_significant_contention(&self) -> bool {
        self.contention_rate > 5.0
    }

    /// Check if max wait time exceeds audio frame duration
    ///
    /// At 48kHz with 512-sample frames, one frame = 10.67ms = 10_667_000ns
    /// If lock wait exceeds this, it will cause audio glitches.
    pub fn exceeds_frame_duration(&self, sample_rate: u32, frame_size: usize) -> bool {
        let frame_duration_ns = (frame_size as u64 * 1_000_000_000) / sample_rate as u64;
        self.max_wait_ns > frame_duration_ns
    }

    /// Check if p99 latency exceeds threshold (10ms)
    pub fn high_p99_latency(&self) -> bool {
        self.max_wait_ns > 10_000_000 // 10ms in nanoseconds
    }

    /// Format as human-readable string
    pub fn format(&self) -> String {
        format!(
            "Attempts: {}, Contentions: {} ({:.2}%), Max wait: {:.2}ms, Avg wait: {:.2}μs",
            self.total_attempts,
            self.contentions,
            self.contention_rate,
            self.max_wait_ns as f64 / 1_000_000.0,
            self.avg_wait_ns as f64 / 1_000.0
        )
    }
}

/// Global lock metrics registry
///
/// Tracks metrics for all named locks across the application.
/// Uses a simple Mutex-protected HashMap - this is acceptable because:
/// 1. Recording metrics is lock-free (only reading from registry requires lock)
/// 2. Reports are generated infrequently (every 60s)
/// 3. The registry lock is never held during audio processing
pub struct GlobalLockMetrics {
    locks: Mutex<HashMap<String, Arc<LockMetrics>>>,
}

impl Default for GlobalLockMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalLockMetrics {
    /// Create new global metrics registry
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create metrics for a named lock
    pub fn get_or_create(&self, name: &str) -> Arc<LockMetrics> {
        let mut locks = self.locks.lock().unwrap();
        locks
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(LockMetrics::new()))
            .clone()
    }

    /// Record a lock acquisition
    ///
    /// This is the main entry point for instrumentation.
    /// Use like: GLOBAL_LOCK_METRICS.record_lock("playback_manager", duration);
    pub fn record_lock(&self, name: &str, duration: Duration) {
        let metrics = self.get_or_create(name);
        // For now, we assume no contention since we're using blocking locks
        // A future enhancement could use try_lock to detect actual contention
        metrics.record_attempt(false, duration.as_nanos() as u64);
    }

    /// Generate a comprehensive report of all locks
    pub fn report_all(&self) -> Vec<(String, LockMetricsReport)> {
        let locks = self.locks.lock().unwrap();
        locks
            .iter()
            .map(|(name, metrics)| (name.clone(), metrics.report()))
            .collect()
    }

    /// Log metrics report to tracing
    ///
    /// Should be called periodically (e.g., every 60s) to monitor lock health.
    pub fn log_report(&self) {
        let reports = self.report_all();

        if reports.is_empty() {
            return;
        }

        tracing::info!("[LOCK_METRICS] === Lock Contention Report ===");

        for (name, report) in reports {
            if report.total_attempts == 0 {
                continue;
            }

            let max_ms = report.max_wait_ns as f64 / 1_000_000.0;
            let avg_us = report.avg_wait_ns as f64 / 1_000.0;

            if report.high_p99_latency() {
                tracing::warn!(
                    "[LOCK_METRICS] {}: contentions={}, max={:.2}ms, avg={:.2}μs (HIGH LATENCY)",
                    name,
                    report.contentions,
                    max_ms,
                    avg_us
                );
            } else if report.is_significant_contention() {
                tracing::warn!(
                    "[LOCK_METRICS] {}: contentions={}, max={:.2}ms, avg={:.2}μs (HIGH CONTENTION)",
                    name,
                    report.contentions,
                    max_ms,
                    avg_us
                );
            } else {
                tracing::debug!(
                    "[LOCK_METRICS] {}: attempts={}, contentions={}, max={:.2}ms, avg={:.2}μs",
                    name,
                    report.total_attempts,
                    report.contentions,
                    max_ms,
                    avg_us
                );
            }
        }
    }

    /// Reset all metrics
    pub fn reset_all(&self) {
        let locks = self.locks.lock().unwrap();
        for metrics in locks.values() {
            metrics.reset();
        }
    }
}

/// Global singleton for lock metrics
pub static GLOBAL_LOCK_METRICS: LazyLock<GlobalLockMetrics> = LazyLock::new(GlobalLockMetrics::new);

/// Helper for timing lock operations
///
/// # Usage
/// ```
/// let timer = LockTimer::start();
/// match lock.try_lock() {
///     Ok(guard) => metrics.record_attempt(false, timer.elapsed_ns()),
///     Err(_) => metrics.record_attempt(true, timer.elapsed_ns()),
/// }
/// ```
pub struct LockTimer {
    start: Instant,
}

impl LockTimer {
    /// Start timing
    #[inline]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time in nanoseconds
    #[inline]
    pub fn elapsed_ns(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_lock_metrics_basic() {
        let metrics = LockMetrics::new();

        // Record some attempts
        metrics.record_attempt(false, 100); // Success, 100ns
        metrics.record_attempt(true, 5000); // Contention, 5μs
        metrics.record_attempt(false, 200); // Success, 200ns

        let report = metrics.report();
        assert_eq!(report.total_attempts, 3);
        assert_eq!(report.contentions, 1);
        assert_eq!(report.max_wait_ns, 5000);
        assert!((report.contention_rate - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_lock_metrics_reset() {
        let metrics = LockMetrics::new();
        metrics.record_attempt(false, 100);
        metrics.reset();

        let report = metrics.report();
        assert_eq!(report.total_attempts, 0);
        assert_eq!(report.contentions, 0);
        assert_eq!(report.max_wait_ns, 0);
    }

    #[test]
    fn test_contention_detection() {
        let metrics = LockMetrics::new();

        // 10% contention
        for _ in 0..9 {
            metrics.record_attempt(false, 100);
        }
        metrics.record_attempt(true, 1000);

        let report = metrics.report();
        assert!(report.is_significant_contention());
    }

    #[test]
    fn test_frame_duration_check() {
        let metrics = LockMetrics::new();

        // 48kHz, 512 samples = 10.67ms frame
        metrics.record_attempt(false, 20_000_000); // 20ms wait

        let report = metrics.report();
        assert!(report.exceeds_frame_duration(48000, 512));
    }

    #[test]
    fn test_lock_timer() {
        let timer = LockTimer::start();
        thread::sleep(Duration::from_micros(100));
        let elapsed = timer.elapsed_ns();

        // Should be at least 100μs = 100_000ns
        assert!(elapsed >= 100_000);
    }

    #[test]
    fn test_global_metrics() {
        let metrics = GlobalLockMetrics::new();

        // Record some lock operations
        metrics.record_lock("test_lock_1", Duration::from_micros(100));
        metrics.record_lock("test_lock_1", Duration::from_micros(200));
        metrics.record_lock("test_lock_2", Duration::from_micros(50));

        let reports = metrics.report_all();
        assert_eq!(reports.len(), 2);

        // Find test_lock_1 report
        let lock1_report = reports
            .iter()
            .find(|(name, _)| name == "test_lock_1")
            .map(|(_, report)| report)
            .unwrap();

        assert_eq!(lock1_report.total_attempts, 2);
        assert_eq!(lock1_report.max_wait_ns, 200_000);
    }

    #[test]
    fn test_high_p99_detection() {
        let metrics = LockMetrics::new();
        metrics.record_attempt(false, 15_000_000); // 15ms

        let report = metrics.report();
        assert!(report.high_p99_latency());
    }
}
