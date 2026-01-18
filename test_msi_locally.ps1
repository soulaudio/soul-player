# Local MSI Installation Test Script
# Run this in PowerShell as Administrator

Write-Host "=== Soul Player MSI Local Test ===" -ForegroundColor Cyan

# Find MSI file
$msi = Get-ChildItem -Path target\*\release\bundle\msi -Filter "*.msi" -Recurse | Select-Object -First 1

if (-not $msi) {
    Write-Error "MSI file not found. Did you run 'yarn build:desktop'?"
    exit 1
}

Write-Host "`nFound MSI: $($msi.FullName)" -ForegroundColor Green
Write-Host "Size: $([math]::Round($msi.Length / 1MB, 2)) MB"

# Uninstall any existing version first
Write-Host "`nChecking for existing installation..." -ForegroundColor Yellow
$existing = Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" | Where-Object { $_.DisplayName -like "*Soul Player*" }
if ($existing) {
    Write-Host "Found existing installation, uninstalling..."
    msiexec /x $existing.PSChildName /qn /norestart
    Start-Sleep -Seconds 5
}

# Install with full logging
Write-Host "`nInstalling MSI with verbose logging..." -ForegroundColor Yellow
$logFile = "soul-player-install.log"
Start-Process msiexec.exe -ArgumentList "/i", $msi.FullName, "/qn", "/norestart", "/L*V", $logFile -Wait -NoNewWindow

Write-Host "`nInstallation complete! Checking..." -ForegroundColor Green

# Check if binary exists
if (Test-Path "C:\Program Files\Soul Player\soul-player.exe") {
    Write-Host "✅ Binary installed successfully" -ForegroundColor Green
    
    # Try to run it
    Write-Host "`nTrying to launch app..." -ForegroundColor Yellow
    $process = Start-Process "C:\Program Files\Soul Player\soul-player.exe" -PassThru
    
    # Wait a few seconds to see if it crashes
    Start-Sleep -Seconds 3
    
    if ($process.HasExited) {
        Write-Host "❌ App crashed with exit code: $($process.ExitCode)" -ForegroundColor Red
        Write-Host "`nCheck the log file for details: $logFile"
        Write-Host "Also check Event Viewer: eventvwr.msc -> Windows Logs -> Application"
        
        # Show last errors from install log
        Write-Host "`nLast 30 lines of install log:" -ForegroundColor Yellow
        Get-Content $logFile | Select-Object -Last 30
    } else {
        Write-Host "✅ App is running!" -ForegroundColor Green
        Write-Host "Process ID: $($process.Id)"
        Write-Host "Close the app window when done testing."
    }
} else {
    Write-Host "❌ Binary not found after installation" -ForegroundColor Red
    Write-Host "Check log file: $logFile"
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
