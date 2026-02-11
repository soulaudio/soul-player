use anyhow::Result;

use crate::check::{rust, typescript};
use crate::util::output::{print_complete, print_error, print_header};

/// Run CI-optimized checks
///
/// Similar to precommit but optimized for CI environments:
/// - Structured output for CI logs
/// - Fail-fast disabled (collect all errors)
/// - No Windows file lock workarounds (CI has clean environment)
///
/// Runs all quality checks:
/// 1. Rust formatting check
/// 2. Clippy lints
/// 3. Rust tests
/// 4. TypeScript type checking
/// 5. ESLint
pub fn run() -> Result<()> {
    print_header("CI Quality Checks");
    println!("Running CI-optimized check pipeline...\n");

    let mut failures = Vec::new();

    // ========================================================================
    // Rust Checks
    // ========================================================================

    // 1. Formatting
    if let Err(e) = rust::run_fmt(false) {
        failures.push(format!("Rust formatting: {}", e));
    } else {
        println!();
    }

    // 2. Clippy
    if let Err(e) = rust::run_clippy(false) {
        failures.push(format!("Clippy: {}", e));
    } else {
        println!();
    }

    // 3. Tests
    if let Err(e) = rust::run_tests(None, vec![]) {
        failures.push(format!("Rust tests: {}", e));
    } else {
        println!();
    }

    // ========================================================================
    // TypeScript Checks
    // ========================================================================

    // 4. TypeScript type checking
    if let Err(e) = typescript::run_typescript(None) {
        failures.push(format!("TypeScript: {}", e));
    } else {
        println!();
    }

    // 5. ESLint
    if let Err(e) = typescript::run_lint(false, None) {
        failures.push(format!("ESLint: {}", e));
    } else {
        println!();
    }

    // ========================================================================
    // Report Results
    // ========================================================================

    if failures.is_empty() {
        print_complete("All CI checks passed!");
        Ok(())
    } else {
        print_header("CI Check Failures");
        for failure in &failures {
            print_error(failure);
        }
        println!();
        anyhow::bail!("{} check(s) failed", failures.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_structure() {
        // This test just verifies the function exists and has correct signature
        // We don't run actual checks in unit tests
        let _fn_ptr: fn() -> Result<()> = run;
    }
}
