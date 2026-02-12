//! Circuit Breaker for Track Loading
//!
//! Implements fault tolerance for track loading failures using a circuit breaker pattern.
//! Prevents playback from getting stuck on repeatedly failing tracks.
//!
//! # Strategy
//!
//! - **3 consecutive failures** → skip to next track
//! - **10 failures in 60s** → open circuit, pause playback
//! - **After 30s in Open** → transition to HalfOpen, try one track
//! - **Success in HalfOpen** → close circuit
//!
//! # Exponential Backoff
//!
//! - 1st retry: immediate
//! - 2nd retry: 1s delay
//! - 3rd retry: 2s delay
//! - 4th+: skip to next track

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Circuit breaker thresholds
const CONSECUTIVE_FAILURE_THRESHOLD: u32 = 3;
const WINDOW_FAILURE_THRESHOLD: u32 = 10;
const FAILURE_WINDOW: Duration = Duration::from_secs(60);
const OPEN_TO_HALFOPEN_TIMEOUT: Duration = Duration::from_secs(30);

/// Backoff delays for retries
const BACKOFF_DELAYS: [Duration; 3] = [
    Duration::from_secs(0), // 1st retry: immediate
    Duration::from_secs(1), // 2nd retry: 1s
    Duration::from_secs(2), // 3rd retry: 2s
];

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - track loading allowed
    Closed,
    /// Too many failures - track loading blocked, playback paused
    Open,
    /// Testing if service recovered - trying one track
    HalfOpen,
}

/// Circuit breaker for track loading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current circuit state
    state: CircuitState,
    /// Number of consecutive failures (for current track)
    consecutive_failures: u32,
    /// Number of failures in the current time window
    failures_in_window: u32,
    /// When the failure window started
    window_start: Option<Instant>,
    /// When the circuit was opened (for timeout to HalfOpen)
    opened_at: Option<Instant>,
    /// Last retry attempt time (for backoff)
    last_retry: Option<Instant>,
    /// Current track being retried (to detect track changes, Arc for efficiency)
    current_track_id: Option<Arc<str>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker in Closed state
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            failures_in_window: 0,
            window_start: None,
            opened_at: None,
            last_retry: None,
            current_track_id: None,
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Check if track loading is allowed
    ///
    /// Returns true if circuit is Closed or HalfOpen.
    /// Returns false if circuit is Open.
    pub fn is_loading_allowed(&self) -> bool {
        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                // Check if we should transition to HalfOpen
                if let Some(opened_at) = self.opened_at {
                    if opened_at.elapsed() >= OPEN_TO_HALFOPEN_TIMEOUT {
                        // Caller should call try_halfopen() to transition
                        return false; // Still not allowed until explicit transition
                    }
                }
                false
            }
        }
    }

    /// Check if we should transition from Open to HalfOpen
    ///
    /// Returns true if circuit is Open and timeout has elapsed.
    pub fn should_try_halfopen(&self) -> bool {
        if self.state != CircuitState::Open {
            return false;
        }

        if let Some(opened_at) = self.opened_at {
            opened_at.elapsed() >= OPEN_TO_HALFOPEN_TIMEOUT
        } else {
            false
        }
    }

    /// Transition from Open to HalfOpen (testing recovery)
    ///
    /// Should be called after `should_try_halfopen()` returns true.
    pub fn try_halfopen(&mut self) {
        if self.state == CircuitState::Open {
            tracing::info!("[CircuitBreaker] Transitioning Open → HalfOpen (testing recovery)");
            self.state = CircuitState::HalfOpen;
            self.opened_at = None;
        }
    }

    /// Check if retry is allowed (based on backoff delay)
    ///
    /// Returns true if enough time has passed since last retry.
    pub fn is_retry_allowed(&self, track_id: &str) -> bool {
        // If track changed, reset retry state
        if self.current_track_id.as_deref() != Some(track_id) {
            return true;
        }

        // Check backoff delay
        if let Some(last_retry) = self.last_retry {
            let retry_index = (self.consecutive_failures - 1) as usize;
            let delay = BACKOFF_DELAYS
                .get(retry_index)
                .copied()
                .unwrap_or(Duration::MAX); // No more retries after 3rd

            last_retry.elapsed() >= delay
        } else {
            true // First attempt
        }
    }

    /// Get backoff delay for next retry
    ///
    /// Returns None if no more retries should be attempted.
    pub fn next_retry_delay(&self) -> Option<Duration> {
        let retry_index = self.consecutive_failures as usize;
        BACKOFF_DELAYS.get(retry_index).copied()
    }

    /// Record a track loading failure
    ///
    /// Returns true if we should skip to the next track.
    /// Returns false if we should retry the current track.
    pub fn record_failure(&mut self, track_id: Arc<str>) -> CircuitBreakerAction {
        // If track changed, reset consecutive failure counter
        if self.current_track_id.as_deref() != Some(&track_id) {
            self.consecutive_failures = 0;
            self.current_track_id = Some(track_id.clone());
        }

        // Get retry delay BEFORE incrementing consecutive_failures
        // (delay is based on current failure count, not the next one)
        let retry_delay = BACKOFF_DELAYS
            .get(self.consecutive_failures as usize)
            .copied();

        self.consecutive_failures += 1;
        self.last_retry = Some(Instant::now());

        // Update failure window
        let now = Instant::now();
        if let Some(window_start) = self.window_start {
            if window_start.elapsed() > FAILURE_WINDOW {
                // Window expired - start new window
                self.window_start = Some(now);
                self.failures_in_window = 1;
            } else {
                // Within window - increment counter
                self.failures_in_window += 1;
            }
        } else {
            // First failure - start window
            self.window_start = Some(now);
            self.failures_in_window = 1;
        }

        tracing::warn!(
            track_id = %track_id,
            consecutive = self.consecutive_failures,
            window = self.failures_in_window,
            "[CircuitBreaker] Track loading failed"
        );

        // Check if we should open the circuit (too many failures in window)
        if self.failures_in_window >= WINDOW_FAILURE_THRESHOLD && self.state != CircuitState::Open {
            tracing::error!(
                failures = self.failures_in_window,
                window_secs = FAILURE_WINDOW.as_secs(),
                "[CircuitBreaker] Opening circuit: too many failures in time window"
            );
            self.state = CircuitState::Open;
            self.opened_at = Some(now);
            return CircuitBreakerAction::OpenCircuit;
        }

        // Check if we should skip to next track (consecutive failures)
        if self.consecutive_failures >= CONSECUTIVE_FAILURE_THRESHOLD {
            tracing::warn!(
                track_id = %track_id,
                consecutive = self.consecutive_failures,
                "[CircuitBreaker] Skipping track due to consecutive failures"
            );
            self.consecutive_failures = 0; // Reset for next track
            self.current_track_id = None;
            return CircuitBreakerAction::SkipTrack;
        }

        // Check if we should retry with backoff
        if let Some(delay) = retry_delay {
            CircuitBreakerAction::RetryWithDelay(delay)
        } else {
            // No more retries - skip track
            tracing::warn!(
                track_id = %track_id,
                "[CircuitBreaker] No more retries available, skipping track"
            );
            self.consecutive_failures = 0;
            self.current_track_id = None;
            CircuitBreakerAction::SkipTrack
        }
    }

    /// Record a successful track load
    ///
    /// Resets consecutive failure counter and closes circuit if in HalfOpen state.
    pub fn record_success(&mut self, track_id: Arc<str>) {
        tracing::info!(
            track_id = %track_id,
            state = ?self.state,
            "[CircuitBreaker] Track loaded successfully"
        );

        // Reset consecutive failures
        self.consecutive_failures = 0;
        self.current_track_id = Some(track_id);
        self.last_retry = None;

        // If in HalfOpen, transition to Closed (recovery confirmed)
        if self.state == CircuitState::HalfOpen {
            tracing::info!("[CircuitBreaker] HalfOpen → Closed (recovery confirmed)");
            self.state = CircuitState::Closed;
            self.opened_at = None;
            // Reset window counters on recovery
            self.failures_in_window = 0;
            self.window_start = None;
        }
    }

    /// Reset the circuit breaker to initial state
    ///
    /// Useful for user-initiated actions (e.g., manual track selection).
    pub fn reset(&mut self) {
        tracing::info!("[CircuitBreaker] Manual reset");
        self.state = CircuitState::Closed;
        self.consecutive_failures = 0;
        self.failures_in_window = 0;
        self.window_start = None;
        self.opened_at = None;
        self.last_retry = None;
        self.current_track_id = None;
    }

    /// Get current consecutive failure count
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Get failures in current window
    pub fn failures_in_window(&self) -> u32 {
        self.failures_in_window
    }
}

/// Action to take after recording a failure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerAction {
    /// Retry loading the track after the specified delay
    RetryWithDelay(Duration),
    /// Skip to the next track (too many consecutive failures)
    SkipTrack,
    /// Open the circuit and pause playback (too many failures in window)
    OpenCircuit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_consecutive_failures_skip_track() {
        let mut cb = CircuitBreaker::new();
        assert_eq!(cb.state(), CircuitState::Closed);

        // First failure - retry immediately
        let action = cb.record_failure("track1".into());
        assert_eq!(action, CircuitBreakerAction::RetryWithDelay(Duration::ZERO));
        assert_eq!(cb.consecutive_failures(), 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        // Second failure - retry after 1s
        let action = cb.record_failure("track1".into());
        assert_eq!(
            action,
            CircuitBreakerAction::RetryWithDelay(Duration::from_secs(1))
        );
        assert_eq!(cb.consecutive_failures(), 2);

        // Third failure - skip track
        let action = cb.record_failure("track1".into());
        assert_eq!(action, CircuitBreakerAction::SkipTrack);
        assert_eq!(cb.consecutive_failures(), 0); // Reset after skip
    }

    #[test]
    fn test_window_failures_open_circuit() {
        let mut cb = CircuitBreaker::new();

        // Simulate 10 failures across different tracks
        for i in 0..9 {
            let track_id = format!("track{}", i % 3).into(); // Alternate between 3 tracks
            let action = cb.record_failure(track_id);
            // Should not open circuit yet
            assert!(
                !matches!(action, CircuitBreakerAction::OpenCircuit),
                "Circuit should not open before threshold"
            );
            assert_eq!(cb.state(), CircuitState::Closed);
        }

        // 10th failure - should open circuit
        let action = cb.record_failure("track_final".into());
        assert_eq!(action, CircuitBreakerAction::OpenCircuit);
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_loading_allowed());
    }

    #[test]
    fn test_success_resets_consecutive() {
        let mut cb = CircuitBreaker::new();

        // Record 2 failures
        cb.record_failure("track1".into());
        cb.record_failure("track1".into());
        assert_eq!(cb.consecutive_failures(), 2);

        // Success resets consecutive counter
        cb.record_success("track1".into());
        assert_eq!(cb.consecutive_failures(), 0);
    }

    #[test]
    fn test_track_change_resets_consecutive() {
        let mut cb = CircuitBreaker::new();

        // Record 2 failures on track1
        cb.record_failure("track1".into());
        cb.record_failure("track1".into());
        assert_eq!(cb.consecutive_failures(), 2);

        // Different track resets consecutive counter
        let action = cb.record_failure("track2".into());
        assert_eq!(action, CircuitBreakerAction::RetryWithDelay(Duration::ZERO));
        assert_eq!(cb.consecutive_failures(), 1);
    }

    #[test]
    fn test_halfopen_to_closed_on_success() {
        let mut cb = CircuitBreaker::new();

        // Manually transition to HalfOpen (simulating timeout)
        cb.state = CircuitState::HalfOpen;

        // Success in HalfOpen should close circuit
        cb.record_success("track1".into());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_retry_not_allowed_during_backoff() {
        let mut cb = CircuitBreaker::new();

        // First failure
        cb.record_failure("track1".into());

        // Retry immediately allowed (0s backoff)
        assert!(cb.is_retry_allowed("track1"));

        // Second failure
        cb.record_failure("track1".into());

        // Retry not allowed immediately (1s backoff)
        assert!(!cb.is_retry_allowed("track1"));

        // After 1s, retry allowed
        sleep(Duration::from_millis(1100));
        assert!(cb.is_retry_allowed("track1"));
    }

    #[test]
    fn test_reset_clears_all_state() {
        let mut cb = CircuitBreaker::new();

        // Record some failures
        cb.record_failure("track1".into());
        cb.record_failure("track1".into());
        cb.state = CircuitState::Open;
        cb.opened_at = Some(Instant::now());

        // Reset should clear everything
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.consecutive_failures(), 0);
        assert_eq!(cb.failures_in_window(), 0);
        assert!(cb.opened_at.is_none());
    }

    #[test]
    fn test_window_expiry_resets_counter() {
        let mut cb = CircuitBreaker::new();

        // Start a failure window
        cb.record_failure("track1".into());
        assert_eq!(cb.failures_in_window(), 1);

        // Manually expire the window
        cb.window_start = Some(Instant::now() - FAILURE_WINDOW - Duration::from_secs(1));

        // Next failure should start new window
        cb.record_failure("track2".into());
        assert_eq!(cb.failures_in_window(), 1); // Reset to 1
    }

    #[test]
    fn test_should_try_halfopen_after_timeout() {
        let mut cb = CircuitBreaker::new();

        // Manually open circuit
        cb.state = CircuitState::Open;
        cb.opened_at = Some(Instant::now() - OPEN_TO_HALFOPEN_TIMEOUT - Duration::from_secs(1));

        // Should transition to HalfOpen
        assert!(cb.should_try_halfopen());

        cb.try_halfopen();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.is_loading_allowed());
    }
}
