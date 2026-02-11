//! System dependencies installation

use anyhow::{Context, Result};

use crate::util::{exec, output, platform};

pub fn run(manual: bool) -> Result<()> {
    output::print_header("System Dependencies Setup");

    let platform = platform::Platform::current();
    let package_manager = platform::PackageManager::detect()?;

    match platform {
        platform::Platform::MacOS => install_macos(manual, package_manager.as_ref()),
        platform::Platform::Linux => install_linux(manual, package_manager.as_ref()),
        platform::Platform::Windows => install_windows(manual, package_manager.as_ref()),
    }
}

pub fn run_all(yes: bool) -> Result<()> {
    output::print_header("Complete First-Time Setup");

    println!();
    println!("This will:");
    println!("  1. Install system dependencies");
    println!("  2. Setup environment (.env)");
    println!("  3. Setup database (SQLx)");
    println!("  4. Install Yarn dependencies");
    println!("  5. Setup git hooks");
    println!();

    if !yes {
        print!("Continue? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            output::print_info("Setup cancelled");
            return Ok(());
        }
    }

    // Step 1: Install system dependencies (manual mode to avoid sudo prompts)
    println!();
    output::print_header("Step 1/5: System Dependencies");
    run(true)?;

    // Step 2: Setup environment
    println!();
    output::print_header("Step 2/5: Environment Setup");
    super::env::run(false)?;

    // Step 3: Setup SQLx
    println!();
    output::print_header("Step 3/5: Database Setup");
    super::sqlx::run(false, false)?;

    // Step 4: Yarn install
    println!();
    output::print_header("Step 4/5: Yarn Dependencies");
    install_yarn_dependencies()?;

    // Step 5: Git hooks
    println!();
    output::print_header("Step 5/5: Git Hooks");
    super::hooks::run()?;

    // Final summary
    println!();
    output::print_complete("Complete setup finished!");

    println!();
    println!("Next steps:");
    println!("  cargo xtask dev desktop  # Run desktop app");
    println!("  cargo xtask check ci     # Run CI checks");
    println!();

    Ok(())
}

fn install_macos(manual: bool, _package_manager: Option<&platform::PackageManager>) -> Result<()> {
    // Check for Xcode Command Line Tools
    output::print_step("Checking Xcode Command Line Tools");
    let xcode_check = std::process::Command::new("xcode-select")
        .arg("-p")
        .output();

    match xcode_check {
        Ok(output) if output.status.success() => {
            output::print_success("Xcode Command Line Tools installed");
        }
        _ => {
            output::print_warning("Xcode Command Line Tools not found");
            if manual {
                println!("    Run: xcode-select --install");
            } else {
                output::print_step("Installing Xcode Command Line Tools");
                let success = exec::run_command_inherit(
                    "xcode-select",
                    &["--install"],
                    "Installing Xcode tools",
                )?;
                if !success {
                    output::print_warning(
                        "Xcode tools installation initiated - please complete and re-run",
                    );
                }
            }
        }
    }

    // Check for Homebrew
    output::print_step("Checking Homebrew");
    if !exec::command_exists("brew") {
        output::print_warning("Homebrew not found");
        if manual {
            println!("    Install from: https://brew.sh/");
        } else {
            anyhow::bail!("Homebrew required but not found. Install from: https://brew.sh/");
        }
        return Ok(());
    }
    output::print_success("Homebrew installed");

    // Install dependencies
    let packages = &["pkg-config", "cmake", "sqlite"];

    if manual {
        println!();
        output::print_info("Install dependencies with:");
        println!("  brew install {}", packages.join(" "));
    } else {
        output::print_step("Installing dependencies via Homebrew");
        let mut cmd = vec!["brew", "install"];
        cmd.extend(packages.iter().map(|s| *s));

        let success = exec::run_command_inherit(
            "brew",
            &packages.iter().map(|s| *s).collect::<Vec<_>>(),
            "Installing Homebrew packages",
        )?;

        if success {
            output::print_success("Dependencies installed");
        } else {
            output::print_warning("Some dependencies may have failed to install");
        }
    }

    install_cargo_tools(manual)?;

    output::print_complete("macOS setup complete!");
    Ok(())
}

fn install_linux(manual: bool, package_manager: Option<&platform::PackageManager>) -> Result<()> {
    let pm = package_manager
        .context("No package manager detected. Please install dependencies manually.")?;

    let packages: Vec<&str> = match pm {
        platform::PackageManager::Apt => vec![
            "build-essential",
            "libssl-dev",
            "pkg-config",
            "libasound2-dev",
            "libgtk-3-dev",
            "libwebkit2gtk-4.1-dev",
            "libappindicator3-dev",
            "librsvg2-dev",
            "patchelf",
            "cmake",
            "clang",
            "sqlite3",
        ],
        platform::PackageManager::Pacman => vec![
            "base-devel",
            "openssl",
            "pkg-config",
            "alsa-lib",
            "gtk3",
            "webkit2gtk",
            "libappindicator-gtk3",
            "librsvg",
            "cmake",
            "clang",
            "sqlite",
        ],
        platform::PackageManager::Dnf => vec![
            "gcc",
            "gcc-c++",
            "openssl-devel",
            "pkg-config",
            "alsa-lib-devel",
            "gtk3-devel",
            "webkit2gtk4.1-devel",
            "libappindicator-gtk3-devel",
            "librsvg2-devel",
            "cmake",
            "clang",
            "sqlite",
        ],
        _ => anyhow::bail!("Unsupported package manager: {:?}", pm),
    };

    if manual {
        println!();
        output::print_info("Install dependencies with:");
        let cmd = pm.install_command(&packages);
        println!("  {}", cmd.join(" "));
    } else {
        output::print_step("Installing Linux dependencies");
        let cmd_vec = pm.install_command(&packages);
        let (program, args_strings) = cmd_vec.split_first().context("Empty install command")?;

        // Convert Vec<String> to Vec<&str> for exec::run_command_inherit
        let args_refs: Vec<&str> = args_strings.iter().map(|s| s.as_str()).collect();

        let success = exec::run_command_inherit(program, &args_refs, "Installing system packages")?;

        if success {
            output::print_success("Dependencies installed");
        } else {
            output::print_warning("Some dependencies may have failed to install");
        }
    }

    install_cargo_tools(manual)?;

    output::print_complete("Linux setup complete!");
    Ok(())
}

fn install_windows(manual: bool, package_manager: Option<&platform::PackageManager>) -> Result<()> {
    output::print_info("Windows requires manual installation of some dependencies");
    println!();

    // Check Visual Studio Build Tools
    output::print_step("Checking Visual Studio Build Tools");
    let vswhere = std::path::Path::new(
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    );
    if vswhere.exists() {
        output::print_success("Visual Studio Build Tools installed");
    } else {
        output::print_warning("Visual Studio Build Tools not found");
        if manual {
            println!("    Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/");
            if let Some(pm) = package_manager {
                if matches!(pm, platform::PackageManager::Winget) {
                    println!("    Or: winget install Microsoft.VisualStudio.2022.BuildTools");
                } else if matches!(pm, platform::PackageManager::Chocolatey) {
                    println!("    Or: choco install visualstudio2022buildtools");
                }
            }
        }
    }

    // Check CMake
    output::print_step("Checking CMake");
    if exec::command_exists("cmake") {
        output::print_success("CMake installed");
    } else {
        output::print_warning("CMake not found");
        if manual {
            println!("    Download: https://cmake.org/download/");
            if let Some(pm) = package_manager {
                if matches!(pm, platform::PackageManager::Winget) {
                    println!("    Or: winget install Kitware.CMake");
                } else if matches!(pm, platform::PackageManager::Chocolatey) {
                    println!("    Or: choco install cmake");
                }
            }
        }
    }

    // Check LLVM/Clang
    output::print_step("Checking LLVM/Clang");
    if exec::command_exists("clang") {
        output::print_success("LLVM/Clang installed");
    } else {
        output::print_warning("LLVM/Clang not found (required for ASIO support)");
        if manual {
            println!("    Download: https://releases.llvm.org/");
            if let Some(pm) = package_manager {
                if matches!(pm, platform::PackageManager::Winget) {
                    println!("    Or: winget install LLVM.LLVM");
                } else if matches!(pm, platform::PackageManager::Chocolatey) {
                    println!("    Or: choco install llvm");
                }
            }
        }
    }

    // Check LIBCLANG_PATH
    output::print_step("Checking LIBCLANG_PATH environment variable");
    if std::env::var("LIBCLANG_PATH").is_ok() {
        output::print_success("LIBCLANG_PATH is set");
    } else {
        output::print_warning("LIBCLANG_PATH not set");
        if manual {
            println!(r#"    Set: $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin""#);
            println!(
                r#"    Or permanently: [System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")"#
            );
        }
    }

    // Check WebView2
    output::print_step("Checking WebView2 Runtime");
    let _webview2_paths = [
        r"HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        r"HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ];

    // Note: We can't easily check registry from Rust in cross-platform way
    output::print_info("WebView2 is usually pre-installed on Windows 10/11");
    if manual {
        println!("    Download if needed: https://developer.microsoft.com/en-us/microsoft-edge/webview2/");
    }

    install_cargo_tools(manual)?;

    output::print_complete("Windows dependency check complete!");

    if manual {
        println!();
        output::print_info("After installing dependencies, run: cargo xtask setup all");
    }

    Ok(())
}

fn install_cargo_tools(manual: bool) -> Result<()> {
    let tools = [
        ("cargo-audit", "cargo-audit"),
        (
            "sqlx-cli",
            "sqlx-cli --no-default-features --features sqlite",
        ),
        ("wasm-pack", "wasm-pack"),
    ];

    if manual {
        println!();
        output::print_info("Install Cargo tools:");
        for (name, _) in &tools {
            if exec::command_exists(name) {
                println!("  [OK] {} already installed", name);
            } else {
                println!("  cargo install {} --locked", name);
            }
        }
    } else {
        output::print_step("Installing Cargo tools");
        for (name, install_args) in &tools {
            if exec::command_exists(name) {
                output::print_success(&format!("{} already installed", name));
            } else {
                output::print_step(&format!("Installing {}", name));
                let args: Vec<&str> = vec!["install"];
                let mut all_args = args;
                all_args.extend(install_args.split_whitespace());
                all_args.push("--locked");

                let success =
                    exec::run_command("cargo", &all_args, &format!("Installing {}", name))?;

                if success {
                    output::print_success(&format!("{} installed", name));
                } else {
                    output::print_warning(&format!("{} installation failed (non-critical)", name));
                }
            }
        }
    }

    Ok(())
}

fn install_yarn_dependencies() -> Result<()> {
    output::print_step("Enabling Corepack for Yarn");

    // Try to enable corepack
    let corepack_enabled = if exec::command_exists("corepack") {
        let success = exec::run_command("corepack", &["enable"], "Enabling Corepack")?;

        if success {
            output::print_success("Corepack enabled");
            true
        } else {
            // Try with sudo on Unix
            #[cfg(unix)]
            {
                let success = exec::run_command(
                    "sudo",
                    &["corepack", "enable"],
                    "Enabling Corepack (with sudo)",
                )?;

                if success {
                    output::print_success("Corepack enabled (with sudo)");
                    true
                } else {
                    output::print_warning("Failed to enable Corepack");
                    false
                }
            }
            #[cfg(not(unix))]
            {
                output::print_warning("Failed to enable Corepack");
                false
            }
        }
    } else {
        output::print_warning("Corepack not found (Node.js may be too old)");
        false
    };

    if corepack_enabled || exec::command_exists("yarn") {
        output::print_step("Installing Yarn dependencies");
        let success =
            exec::run_command_inherit("yarn", &["install"], "Installing Yarn dependencies")?;

        if success {
            output::print_success("Yarn dependencies installed");
        } else {
            output::print_warning("Yarn install failed");
        }
    } else {
        output::print_warning("Yarn not available - skip yarn install");
        output::print_info("Install Node.js 20+ and run: corepack enable && yarn install");
    }

    Ok(())
}
