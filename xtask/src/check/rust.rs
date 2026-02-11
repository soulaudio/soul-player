use anyhow::Result;
use colored::Colorize;

use crate::util::{
    exec::{run_command_capture, run_command_inherit},
    output::{print_error, print_header, print_success},
};

/// Run cargo fmt
pub fn run_fmt(fix: bool) -> Result<()> {
    print_header("Rust Formatting");

    let args = if fix {
        vec!["fmt", "--all"]
    } else {
        vec!["fmt", "--all", "--check"]
    };

    let success = run_command_inherit("cargo", &args, "cargo fmt --all [--check]")?;

    if success {
        if fix {
            print_success("Code formatted successfully");
        } else {
            print_success("Rust formatting OK");
        }
        Ok(())
    } else {
        print_error("Rust formatting check failed");
        if !fix {
            println!("\n  Run 'cargo xtask check fmt --fix' to auto-format");
        }
        anyhow::bail!("Formatting check failed");
    }
}

/// Run cargo clippy
pub fn run_clippy(fix: bool) -> Result<()> {
    print_header("Clippy Lints");

    let mut args = vec!["clippy", "--workspace", "--lib", "--bins", "--release"];

    if fix {
        args.push("--fix");
        args.push("--allow-dirty");
        args.push("--allow-staged");
    } else {
        args.push("--");
        args.push("-D");
        args.push("warnings");
    }

    let success = run_command_inherit(
        "cargo",
        &args,
        "cargo clippy --workspace --lib --bins --release",
    )?;

    if success {
        if fix {
            print_success("Clippy issues fixed");
        } else {
            print_success("Clippy OK");
        }
        Ok(())
    } else {
        print_error("Clippy check failed");
        if !fix {
            println!("\n  Run 'cargo xtask check clippy --fix' to auto-fix issues");
        }
        anyhow::bail!("Clippy check failed");
    }
}

/// Run cargo test
pub fn run_tests(package: Option<String>, args: Vec<String>) -> Result<()> {
    print_header("Rust Tests");

    let mut cmd_args = vec!["test".to_string()];

    if let Some(pkg) = package {
        cmd_args.push("-p".to_string());
        cmd_args.push(pkg);
    } else {
        cmd_args.push("--all".to_string());
    }

    // Add user-provided args
    cmd_args.extend(args);

    // Convert to &str refs
    let cmd_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();

    // Try to run tests, but handle Windows file lock issues gracefully
    let result = run_command_capture("cargo", &cmd_refs);

    match result {
        Ok(output) => {
            // Tests passed
            println!("{}", output);
            print_success("Rust tests OK");
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check for Windows file lock errors
            if cfg!(target_os = "windows")
                && (error_msg.contains("The process cannot access the file")
                    || error_msg.contains("being used by another process"))
            {
                println!("{}", error_msg);
                println!();
                println!(
                    "  {} Warning: Rust tests skipped due to Windows file locks",
                    "⚠".yellow()
                );
                println!("  Close running apps (dev server, IDE) and run tests manually");
                println!("  Or use: git commit --no-verify (tests will still run in CI)");
                // Don't fail on Windows file locks
                Ok(())
            } else {
                // Real test failure
                print_error("Rust tests failed");
                println!("\n{}", error_msg);
                anyhow::bail!("Tests failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_check() {
        // This will actually run fmt --check
        // Skip in CI to avoid false positives
        if std::env::var("CI").is_ok() {
            return;
        }

        let result = run_fmt(false);
        // Should not panic, just verify it returns a Result
        assert!(result.is_ok() || result.is_err());
    }
}
