use anyhow::Result;

use crate::check::{rust, typescript};
use crate::util::output::{print_complete, print_header};

/// Run full pre-commit pipeline
///
/// This replaces .husky/pre-commit with a Rust implementation.
/// Runs all quality checks in sequence:
/// 1. Rust formatting check
/// 2. Clippy lints
/// 3. Rust tests (with Windows file lock handling)
/// 4. TypeScript type checking (desktop, shared, marketing)
/// 5. ESLint (desktop, shared)
///
/// Early exit on first failure for fast feedback.
pub fn run() -> Result<()> {
    print_header("Pre-Commit Checks");
    println!("Running full quality check pipeline...\n");

    // ========================================================================
    // Rust Checks
    // ========================================================================

    // 1. Formatting
    rust::run_fmt(false)?;
    println!();

    // 2. Clippy
    rust::run_clippy(false)?;
    println!();

    // 3. Tests
    // Use --test-threads=1 to serialize test execution within each binary.
    // This prevents timing-sensitive tests from competing with each other for CPU,
    // avoiding flaky failures under concurrent debug build test loads.
    rust::run_tests(None, vec!["--".to_string(), "--test-threads=1".to_string()])?;
    println!();

    // ========================================================================
    // TypeScript Checks
    // ========================================================================

    // 4. TypeScript type checking
    typescript::run_typescript(None)?;
    println!();

    // 5. ESLint
    typescript::run_lint(false, None)?;
    println!();

    // ========================================================================
    // All checks passed
    // ========================================================================

    print_complete("All pre-commit checks passed!");
    println!("Safe to commit.\n");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precommit_structure() {
        // This test just verifies the function exists and has correct signature
        // We don't run actual checks in unit tests
        let _fn_ptr: fn() -> Result<()> = run;
    }
}
