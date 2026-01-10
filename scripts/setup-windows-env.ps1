# Soul Player - Windows Environment Setup
# This script configures your PowerShell profile for optimal Rust builds
# and installs required dependencies (LLVM for ASIO support)

Write-Host "Soul Player - Windows Environment Setup" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# ==================================================
# Step 1: Install LLVM (required for ASIO/bindgen)
# ==================================================
Write-Host "Checking LLVM installation (required for ASIO audio support)..." -ForegroundColor Yellow

$llvmPath = "C:\Program Files\LLVM\bin"
$libclangExists = Test-Path "$llvmPath\libclang.dll"

if ($libclangExists) {
    Write-Host "✓ LLVM is already installed at $llvmPath" -ForegroundColor Green
} else {
    Write-Host "LLVM not found. Installing via winget..." -ForegroundColor Yellow

    # Check if winget is available
    $wingetAvailable = Get-Command winget -ErrorAction SilentlyContinue

    if ($wingetAvailable) {
        Write-Host "Installing LLVM (this may take a few minutes)..." -ForegroundColor Cyan
        winget install LLVM.LLVM --accept-package-agreements --accept-source-agreements

        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ LLVM installed successfully!" -ForegroundColor Green
        } else {
            Write-Host "⚠ LLVM installation may have failed. Please install manually:" -ForegroundColor Red
            Write-Host "  winget install LLVM.LLVM" -ForegroundColor White
            Write-Host "  OR download from: https://github.com/llvm/llvm-project/releases" -ForegroundColor White
        }
    } else {
        # Try chocolatey as fallback
        $chocoAvailable = Get-Command choco -ErrorAction SilentlyContinue
        if ($chocoAvailable) {
            Write-Host "Installing LLVM via Chocolatey..." -ForegroundColor Cyan
            choco install llvm -y
        } else {
            Write-Host "⚠ Neither winget nor chocolatey found. Please install LLVM manually:" -ForegroundColor Red
            Write-Host "  Download from: https://github.com/llvm/llvm-project/releases" -ForegroundColor White
            Write-Host "  Or install winget/chocolatey first" -ForegroundColor White
        }
    }
}

# Set LIBCLANG_PATH environment variable
$currentLibclangPath = [Environment]::GetEnvironmentVariable("LIBCLANG_PATH", "User")
if ($currentLibclangPath -ne $llvmPath) {
    Write-Host "Setting LIBCLANG_PATH environment variable..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("LIBCLANG_PATH", $llvmPath, "User")
    $env:LIBCLANG_PATH = $llvmPath
    Write-Host "✓ LIBCLANG_PATH set to $llvmPath" -ForegroundColor Green
} else {
    Write-Host "✓ LIBCLANG_PATH is already configured" -ForegroundColor Green
}

# ==================================================
# Step 2: Configure Cargo target directory
# ==================================================
Write-Host "`nConfiguring Cargo build settings..." -ForegroundColor Yellow

# Check if profile exists
if (!(Test-Path -Path $PROFILE)) {
    Write-Host "Creating PowerShell profile at: $PROFILE" -ForegroundColor Yellow
    New-Item -ItemType File -Path $PROFILE -Force | Out-Null
}

# Check if already configured
$existingContent = Get-Content $PROFILE -Raw -ErrorAction SilentlyContinue
if ($existingContent -match 'CARGO_TARGET_DIR.*target-windows') {
    Write-Host "✓ CARGO_TARGET_DIR is already configured!" -ForegroundColor Green
    Write-Host "`nCurrent setting:" -ForegroundColor Cyan
    Write-Host '  $env:CARGO_TARGET_DIR = "target-windows"' -ForegroundColor White
} else {
    Write-Host "Adding CARGO_TARGET_DIR to your PowerShell profile..." -ForegroundColor Yellow

    # Add to profile
    $setupBlock = @"

# ==================================================
# Soul Player - Rust Build Configuration
# ==================================================
# Use Windows-specific target directory to avoid conflicts with WSL
`$env:CARGO_TARGET_DIR = "target-windows"

"@
    Add-Content -Path $PROFILE -Value $setupBlock

    Write-Host "✓ Configuration added to profile!" -ForegroundColor Green
}

# Apply to current session
$env:CARGO_TARGET_DIR = "target-windows"

Write-Host "`n✓ Setup complete!" -ForegroundColor Green
Write-Host "`nThe following has been configured:" -ForegroundColor Cyan
Write-Host "  • CARGO_TARGET_DIR = target-windows" -ForegroundColor White
Write-Host "  • Profile location: $PROFILE" -ForegroundColor Gray

Write-Host "`nThis means:" -ForegroundColor Cyan
Write-Host "  ✓ No more cargo clean needed when switching from WSL" -ForegroundColor Green
Write-Host "  ✓ Your yarn dev:desktop commands will work perfectly" -ForegroundColor Green
Write-Host "  ✓ Build artifacts won't conflict between Windows and WSL" -ForegroundColor Green

Write-Host "`nYou can now run:" -ForegroundColor Cyan
Write-Host "  yarn dev:desktop" -ForegroundColor White
Write-Host "  yarn dev:marketing" -ForegroundColor White

Write-Host "`nNote: New PowerShell windows will automatically use this setting." -ForegroundColor Gray
Write-Host "      Current window is already configured!" -ForegroundColor Gray
