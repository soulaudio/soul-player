# Local MSI Installation Test Script
# Run this in PowerShell as Administrator

param(
    [switch]$Debug,
    [string]$MsiPath
)

Write-Host "=== Soul Player MSI Local Test ===" -ForegroundColor Cyan

# Find MSI file - check multiple locations
if ($MsiPath) {
    if (Test-Path $MsiPath) {
        $msi = Get-Item $MsiPath
        Write-Host "Using specified MSI: $MsiPath" -ForegroundColor Gray
    } else {
        Write-Error "Specified MSI path not found: $MsiPath"
        exit 1
    }
} else {
    $searchPaths = @(
        "target\*\release\bundle\msi",
        "$env:USERPROFILE\Downloads\windows-*\msi",
        "$env:USERPROFILE\Downloads\*.msi"
    )

    $msi = $null
    foreach ($path in $searchPaths) {
        $msi = Get-ChildItem -Path $path -Filter "*.msi" -Recurse -ErrorAction SilentlyContinue |
               Where-Object { $_.Name -like "*Soul Player*" } |
               Select-Object -First 1
        if ($msi) {
            Write-Host "Found MSI in: $($path)" -ForegroundColor Gray
            break
        }
    }

    if (-not $msi) {
        Write-Error "MSI file not found. Searched in:"
        foreach ($path in $searchPaths) {
            Write-Host "  - $path" -ForegroundColor Gray
        }
        Write-Host "`nOptions:" -ForegroundColor Yellow
        Write-Host "1. Run 'yarn build:desktop' to build locally"
        Write-Host "2. Download artifacts from GitHub Actions to ~/Downloads"
        Write-Host "3. Specify path: .\test-msi-install.ps1 -MsiPath 'C:\path\to\file.msi'"
        exit 1
    }
}

Write-Host "`nFound MSI: $($msi.FullName)" -ForegroundColor Green
Write-Host "Size: $([math]::Round($msi.Length / 1MB, 2)) MB"

# Uninstall any existing version first
Write-Host "`nChecking for existing installation..." -ForegroundColor Yellow
$existing = Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -like "*Soul Player*" }
if ($existing) {
    Write-Host "Found existing installation, uninstalling..." -ForegroundColor Yellow
    $productCode = $existing.PSChildName
    Start-Process msiexec.exe -ArgumentList "/x", $productCode, "/qn", "/norestart" -Wait -NoNewWindow
    Start-Sleep -Seconds 5
    Write-Host "Uninstall complete" -ForegroundColor Green
}

# Install with full logging
Write-Host "`nInstalling MSI with verbose logging..." -ForegroundColor Yellow
$logFile = Join-Path $PSScriptRoot "soul-player-install.log"
$installArgs = @("/i", $msi.FullName, "/qn", "/norestart", "/L*V", $logFile)
Write-Host "Install command: msiexec.exe $($installArgs -join ' ')"

$installProcess = Start-Process msiexec.exe -ArgumentList $installArgs -Wait -NoNewWindow -PassThru
Write-Host "Install process exited with code: $($installProcess.ExitCode)"

if ($installProcess.ExitCode -ne 0) {
    Write-Host "❌ Installation failed!" -ForegroundColor Red
    Write-Host "`nLast 50 lines of install log:" -ForegroundColor Yellow
    Get-Content $logFile | Select-Object -Last 50
    exit 1
}

Write-Host "`n✅ Installation complete!" -ForegroundColor Green

# Check if binary exists
$exePath = "C:\Program Files\Soul Player\soul-player.exe"
if (Test-Path $exePath) {
    Write-Host "✅ Binary found at: $exePath" -ForegroundColor Green

    # Check file properties
    $fileInfo = Get-Item $exePath
    Write-Host "Binary size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB"
    Write-Host "Modified: $($fileInfo.LastWriteTime)"

    # List all files in install directory
    Write-Host "`nInstalled files:" -ForegroundColor Yellow
    Get-ChildItem "C:\Program Files\Soul Player" -Recurse | ForEach-Object {
        Write-Host "  $($_.FullName)"
    }

    # Check WebView2
    Write-Host "`nChecking WebView2..." -ForegroundColor Yellow
    $webView2Path = "C:\Program Files (x86)\Microsoft\EdgeWebView\Application"
    if (Test-Path $webView2Path) {
        Write-Host "✅ WebView2 found at: $webView2Path" -ForegroundColor Green
        $version = Get-ChildItem $webView2Path | Where-Object { $_.PSIsContainer } | Select-Object -First 1 -ExpandProperty Name
        Write-Host "Version: $version"
    } else {
        Write-Host "⚠️  WebView2 not found at expected location" -ForegroundColor Yellow
    }

    # Try to run it with debugging
    Write-Host "`n=== Attempting to launch app ===" -ForegroundColor Cyan
    Write-Host "Note: App will launch in a new window. Check for errors there." -ForegroundColor Yellow

    if ($Debug) {
        # Run with console attached
        Write-Host "Running with debug output..." -ForegroundColor Yellow
        $env:RUST_BACKTRACE = "full"
        $env:RUST_LOG = "debug"

        # Create a batch file to run with console visible
        $batchFile = Join-Path $PSScriptRoot "run-debug.bat"
        @"
@echo off
echo Starting Soul Player with debug logging...
echo.
set RUST_BACKTRACE=full
set RUST_LOG=debug
"$exePath"
echo.
echo App exited with code: %ERRORLEVEL%
pause
"@ | Out-File $batchFile -Encoding ASCII

        Write-Host "Created debug launcher: $batchFile" -ForegroundColor Green
        Write-Host "Run this batch file to see console output" -ForegroundColor Yellow
        Start-Process $batchFile
    } else {
        # Normal launch
        $process = Start-Process $exePath -PassThru

        # Wait a few seconds to see if it crashes
        Start-Sleep -Seconds 5

        if ($process.HasExited) {
            Write-Host "❌ App crashed with exit code: $($process.ExitCode)" -ForegroundColor Red
            Write-Host "`nPossible issues:" -ForegroundColor Yellow
            Write-Host "1. Run with -Debug flag to see console output: .\test-msi-install.ps1 -Debug"
            Write-Host "2. Check Event Viewer: eventvwr.msc -> Windows Logs -> Application"
            Write-Host "3. Check app data folder for errors:"
            Write-Host "   - %APPDATA%\Soul Player"
            Write-Host "   - %LOCALAPPDATA%\Soul Player"

            # Check app data directories
            $appData = "$env:APPDATA\Soul Player"
            $localAppData = "$env:LOCALAPPDATA\Soul Player"

            if (Test-Path $appData) {
                Write-Host "`nFound app data at: $appData" -ForegroundColor Yellow
                Get-ChildItem $appData -Recurse | ForEach-Object {
                    Write-Host "  $($_.FullName)"
                }
            }

            if (Test-Path $localAppData) {
                Write-Host "`nFound local app data at: $localAppData" -ForegroundColor Yellow
                Get-ChildItem $localAppData -Recurse | ForEach-Object {
                    Write-Host "  $($_.FullName)"
                }
            }

            # Check for log files
            $logLocations = @(
                "$appData\logs",
                "$localAppData\logs",
                "C:\Program Files\Soul Player\logs"
            )

            foreach ($logDir in $logLocations) {
                if (Test-Path $logDir) {
                    Write-Host "`nFound logs at: $logDir" -ForegroundColor Yellow
                    Get-ChildItem $logDir -Filter "*.log" | ForEach-Object {
                        Write-Host "`n--- $($_.Name) ---" -ForegroundColor Cyan
                        Get-Content $_.FullName | Select-Object -Last 50
                    }
                }
            }
        } else {
            Write-Host "✅ App is running!" -ForegroundColor Green
            Write-Host "Process ID: $($process.Id)"
            Write-Host "Memory: $([math]::Round($process.WorkingSet64 / 1MB, 2)) MB"
            Write-Host "`nClose the app window when done testing."
        }
    }
} else {
    Write-Host "❌ Binary not found after installation" -ForegroundColor Red
    Write-Host "Expected at: $exePath"
    Write-Host "`nCheck install log: $logFile"
    Write-Host "`nLast 50 lines of install log:" -ForegroundColor Yellow
    Get-Content $logFile | Select-Object -Last 50
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Install log saved to: $logFile" -ForegroundColor Gray
