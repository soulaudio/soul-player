//! Comprehensive test suite for soul-audio-desktop
//!
//! This module contains stress tests, endurance tests, and edge case tests
//! for the audio playback system. These tests are ignored by default and
//! must be run manually with the `--include-ignored` flag.
//!
//! ## Test Categories
//!
//! ### Stress Tests (Lock Contention)
//! - `lock_contention_stress_test.rs` - Concurrent command flooding
//! - Tests system behavior under extreme lock contention
//! - Validates no deadlocks occur under heavy concurrent load
//!
//! ### Endurance Tests
//! - `endurance_stress_test.rs` - Long-running stability tests
//! - Tests memory stability over extended periods (5 min - 1 hour)
//! - Validates no resource leaks (memory, file handles, threads)
//!
//! ### Error Recovery Tests
//! - `corrupted_file_recovery_test.rs` - Corrupted file handling
//! - Tests recovery from invalid/corrupted audio files
//! - Validates graceful error handling and system stability
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all stress tests
//! cargo test --test lock_contention_stress_test -- --include-ignored
//! cargo test --test endurance_stress_test -- --include-ignored
//! cargo test --test corrupted_file_recovery_test -- --include-ignored
//!
//! # Run specific test
//! cargo test --test lock_contention_stress_test test_extreme_command_flood -- --include-ignored
//!
//! # Run short endurance test (5 min)
//! cargo test --test endurance_stress_test test_continuous_playback_5_min -- --include-ignored
//!
//! # Run long endurance test (1 hour)
//! cargo test --test endurance_stress_test test_continuous_playback_1_hour -- --include-ignored
//! ```

pub mod lock_contention_stress_test;
pub mod endurance_stress_test;
pub mod corrupted_file_recovery_test;
