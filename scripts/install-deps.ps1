# Install system dependencies for Soul Player development (Windows)
# Usage: .\scripts\install-deps.ps1
# Run as Administrator for winget installations

param(
    [switch]$SkipRust,
    [switch]$SkipNode,
    [switch]$AutoInstall
)

# Use Continue to handle native command stderr gracefully
$ErrorActionPreference = "Continue"

Write-Host "=== Soul Player Development Environment Setup (Windows) ===" -ForegroundColor Cyan
Write-Host ""

# Check if running as admin
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

function Test-Command {
    param([string]$Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

function Write-Status {
    param([string]$Message, [string]$Status)
    if ($Status -eq "OK") {
        Write-Host "[OK] " -ForegroundColor Green -NoNewline
    } elseif ($Status -eq "MISSING") {
        Write-Host "[MISSING] " -ForegroundColor Red -NoNewline
    } elseif ($Status -eq "WARNING") {
        Write-Host "[WARNING] " -ForegroundColor Yellow -NoNewline
    }
    Write-Host $Message
}

# ============================================================================
# Check Prerequisites
# ============================================================================

Write-Host "=== Checking Prerequisites ===" -ForegroundColor Yellow
Write-Host ""

# Rust
if (-not $SkipRust) {
    if (Test-Command "rustc") {
        $rustVersion = rustc --version
        Write-Status "Rust: $rustVersion" "OK"
    } else {
        Write-Status "Rust not found" "MISSING"
        Write-Host "   Install from: https://rustup.rs/" -ForegroundColor Gray
        if ($AutoInstall) {
            Write-Host "   Installing Rust..." -ForegroundColor Cyan
            Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
            & "$env:TEMP\rustup-init.exe" -y
            $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
        }
    }
}

# Node.js
if (-not $SkipNode) {
    if (Test-Command "node") {
        $nodeVersion = node --version
        Write-Status "Node.js: $nodeVersion" "OK"
    } else {
        Write-Status "Node.js not found" "MISSING"
        Write-Host "   Install from: https://nodejs.org/ (LTS recommended)" -ForegroundColor Gray
    }
}

# ============================================================================
# System Dependencies
# ============================================================================

Write-Host ""
Write-Host "=== Checking System Dependencies ===" -ForegroundColor Yellow
Write-Host ""

$missingDeps = @()

# CMake
if (Test-Command "cmake") {
    $cmakeVersion = cmake --version | Select-Object -First 1
    Write-Status "CMake: $cmakeVersion" "OK"
} else {
    Write-Status "CMake not found (required for r8brain resampler)" "MISSING"
    $missingDeps += "cmake"
}

# LLVM/Clang
if (Test-Command "clang") {
    $clangVersion = clang --version | Select-Object -First 1
    Write-Status "Clang: $clangVersion" "OK"
} else {
    Write-Status "LLVM/Clang not found (required for ASIO support)" "MISSING"
    $missingDeps += "llvm"
}

# LIBCLANG_PATH
if ($env:LIBCLANG_PATH) {
    Write-Status "LIBCLANG_PATH: $env:LIBCLANG_PATH" "OK"
} else {
    Write-Status "LIBCLANG_PATH not set" "WARNING"
    Write-Host "   Set to: C:\Program Files\LLVM\bin" -ForegroundColor Gray
}

# Visual Studio Build Tools
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsInstall = & $vsWhere -latest -property installationPath 2>&1 | Where-Object { $_ -is [string] }
    if ($vsInstall) {
        Write-Status "Visual Studio: $vsInstall" "OK"
    } else {
        Write-Status "Visual Studio Build Tools not found" "MISSING"
        $missingDeps += "vs-buildtools"
    }
} else {
    Write-Status "Visual Studio Build Tools not found" "MISSING"
    $missingDeps += "vs-buildtools"
}

# WebView2
$webview2 = Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if ($webview2) {
    Write-Status "WebView2: $($webview2.pv)" "OK"
} else {
    $webview2User = Get-ItemProperty -Path "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
    if ($webview2User) {
        Write-Status "WebView2: $($webview2User.pv)" "OK"
    } else {
        Write-Status "WebView2 Runtime not detected (may be pre-installed)" "WARNING"
    }
}

# ============================================================================
# Install Missing Dependencies
# ============================================================================

if ($missingDeps.Count -gt 0) {
    Write-Host ""
    Write-Host "=== Missing Dependencies ===" -ForegroundColor Yellow
    Write-Host ""

    if ($AutoInstall -and $isAdmin) {
        Write-Host "Installing missing dependencies via winget..." -ForegroundColor Cyan

        foreach ($dep in $missingDeps) {
            switch ($dep) {
                "cmake" {
                    Write-Host "Installing CMake..." -ForegroundColor Cyan
                    winget install Kitware.CMake --accept-source-agreements --accept-package-agreements
                }
                "llvm" {
                    Write-Host "Installing LLVM..." -ForegroundColor Cyan
                    winget install LLVM.LLVM --accept-source-agreements --accept-package-agreements
                }
                "vs-buildtools" {
                    Write-Host "Installing Visual Studio Build Tools..." -ForegroundColor Cyan
                    winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --accept-package-agreements
                }
            }
        }
    } else {
        Write-Host "To install missing dependencies, run the following commands:" -ForegroundColor Cyan
        Write-Host ""

        if ($missingDeps -contains "cmake") {
            Write-Host "  winget install Kitware.CMake" -ForegroundColor White
        }
        if ($missingDeps -contains "llvm") {
            Write-Host "  winget install LLVM.LLVM" -ForegroundColor White
        }
        if ($missingDeps -contains "vs-buildtools") {
            Write-Host "  winget install Microsoft.VisualStudio.2022.BuildTools" -ForegroundColor White
        }

        Write-Host ""
        Write-Host "Or run this script with -AutoInstall as Administrator:" -ForegroundColor Gray
        Write-Host "  Start-Process powershell -Verb RunAs -ArgumentList '-File', '.\scripts\install-deps.ps1', '-AutoInstall'" -ForegroundColor Gray
    }
}

# ============================================================================
# Environment Variables
# ============================================================================

Write-Host ""
Write-Host "=== Environment Variables ===" -ForegroundColor Yellow
Write-Host ""

if (-not $env:LIBCLANG_PATH) {
    $llvmPath = "C:\Program Files\LLVM\bin"
    if (Test-Path $llvmPath) {
        Write-Host "Setting LIBCLANG_PATH for this session..." -ForegroundColor Cyan
        $env:LIBCLANG_PATH = $llvmPath

        Write-Host ""
        Write-Host "To set permanently, run:" -ForegroundColor Yellow
        Write-Host '  [System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")' -ForegroundColor White
    }
}

# ============================================================================
# Cargo Tools
# ============================================================================

Write-Host ""
Write-Host "=== Installing Cargo Tools ===" -ForegroundColor Yellow
Write-Host ""

if (Test-Command "cargo") {
    # Temporarily allow errors for cargo install (it outputs progress to stderr)
    $prevErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"

    Write-Host "Installing cargo-audit..." -ForegroundColor Cyan
    & cargo install cargo-audit --locked 2>&1 | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) {
            Write-Host $_.ToString() -ForegroundColor Gray
        } else {
            Write-Host $_ -ForegroundColor Gray
        }
    }

    Write-Host "Installing sqlx-cli..." -ForegroundColor Cyan
    & cargo install sqlx-cli --no-default-features --features sqlite --locked 2>&1 | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) {
            Write-Host $_.ToString() -ForegroundColor Gray
        } else {
            Write-Host $_ -ForegroundColor Gray
        }
    }

    Write-Host "Installing wasm-pack (for marketing demo)..." -ForegroundColor Cyan
    & cargo install wasm-pack --locked 2>&1 | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) {
            Write-Host $_.ToString() -ForegroundColor Gray
        } else {
            Write-Host $_ -ForegroundColor Gray
        }
    }

    $ErrorActionPreference = $prevErrorAction
}

# ============================================================================
# Corepack / Yarn
# ============================================================================

Write-Host ""
Write-Host "=== Setting up Yarn ===" -ForegroundColor Yellow
Write-Host ""

if (Test-Command "corepack") {
    Write-Host "Enabling Corepack for Yarn 4.x..." -ForegroundColor Cyan
    & corepack enable 2>&1 | Out-Null
}

# ============================================================================
# Summary
# ============================================================================

Write-Host ""
Write-Host "==============================================" -ForegroundColor Green
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Restart your terminal (to pick up PATH changes)"
Write-Host "  2. yarn install        # Install Node dependencies"
Write-Host "  3. .\scripts\setup-sqlx.ps1  # Setup database (if exists)"
Write-Host "  4. yarn dev:desktop    # Run desktop app"
Write-Host ""

if (-not $env:LIBCLANG_PATH) {
    Write-Host "IMPORTANT: Set LIBCLANG_PATH permanently:" -ForegroundColor Red
    Write-Host '  [System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")' -ForegroundColor White
    Write-Host ""
}

Write-Host "See README.md for more commands." -ForegroundColor Gray
Write-Host "==============================================" -ForegroundColor Green
