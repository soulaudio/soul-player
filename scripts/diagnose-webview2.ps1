# Diagnose WebView2 Installation
# Run this to check WebView2 status

Write-Host "=== WebView2 Diagnostics ===" -ForegroundColor Cyan
Write-Host ""

# Check registry for WebView2 installations
Write-Host "Checking WebView2 Registry Keys..." -ForegroundColor Yellow

$registryPaths = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)

$found = $false
foreach ($path in $registryPaths) {
    if (Test-Path $path) {
        $props = Get-ItemProperty $path
        Write-Host "✅ Found at: $path" -ForegroundColor Green
        Write-Host "   Version: $($props.pv)"
        Write-Host "   Name: $($props.name)"
        $found = $true
    }
}

if (-not $found) {
    Write-Host "❌ WebView2 not found in registry" -ForegroundColor Red
}

Write-Host ""

# Check file system locations
Write-Host "Checking WebView2 File Locations..." -ForegroundColor Yellow

$filePaths = @(
    "C:\Program Files (x86)\Microsoft\EdgeWebView\Application",
    "C:\Program Files\Microsoft\EdgeWebView\Application",
    "$env:LOCALAPPDATA\Microsoft\EdgeWebView\Application"
)

$foundFiles = $false
foreach ($path in $filePaths) {
    if (Test-Path $path) {
        Write-Host "✅ Found at: $path" -ForegroundColor Green
        $versions = Get-ChildItem $path -Directory | Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' }
        foreach ($version in $versions) {
            Write-Host "   Version: $($version.Name)"
            $exePath = Join-Path $version.FullName "msedgewebview2.exe"
            if (Test-Path $exePath) {
                Write-Host "   Executable: ✅ Found" -ForegroundColor Green
            }
        }
        $foundFiles = $true
    }
}

if (-not $foundFiles) {
    Write-Host "❌ WebView2 runtime files not found" -ForegroundColor Red
}

Write-Host ""

# Check installed programs
Write-Host "Checking Installed Programs..." -ForegroundColor Yellow
$webview2Programs = Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -like "*WebView2*" -or $_.DisplayName -like "*Edge WebView*" }

if ($webview2Programs) {
    foreach ($program in $webview2Programs) {
        Write-Host "✅ $($program.DisplayName)" -ForegroundColor Green
        Write-Host "   Version: $($program.DisplayVersion)"
        Write-Host "   Install Location: $($program.InstallLocation)"
    }
} else {
    Write-Host "❌ No WebView2 programs found in uninstall registry" -ForegroundColor Red
}

Write-Host ""

# Check Soul Player installation
Write-Host "Checking Soul Player Installation..." -ForegroundColor Yellow
$soulPlayerPath = "C:\Program Files\Soul Player"

if (Test-Path $soulPlayerPath) {
    Write-Host "✅ Soul Player found at: $soulPlayerPath" -ForegroundColor Green
    Write-Host ""
    Write-Host "Installed files:" -ForegroundColor Gray
    Get-ChildItem $soulPlayerPath -Recurse | ForEach-Object {
        Write-Host "   $($_.FullName.Replace($soulPlayerPath, ''))"
    }
} else {
    Write-Host "❌ Soul Player not installed" -ForegroundColor Red
}

Write-Host ""

# Check Event Viewer for recent errors
Write-Host "Checking Recent Event Viewer Errors..." -ForegroundColor Yellow
try {
    $events = Get-WinEvent -FilterHashtable @{
        LogName = 'Application'
        Level = 2  # Error
        StartTime = (Get-Date).AddHours(-1)
    } -MaxEvents 20 -ErrorAction SilentlyContinue |
    Where-Object { $_.Message -like "*soul-player*" -or $_.Message -like "*WebView2*" -or $_.ProviderName -like "*soul*" }

    if ($events) {
        foreach ($event in $events) {
            Write-Host "---" -ForegroundColor Gray
            Write-Host "Time: $($event.TimeCreated)"
            Write-Host "Source: $($event.ProviderName)"
            Write-Host "ID: $($event.Id)"
            Write-Host "Message: $($event.Message)"
        }
    } else {
        Write-Host "No relevant errors found in last hour" -ForegroundColor Gray
    }
} catch {
    Write-Host "Could not access Event Viewer: $_" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Diagnostics Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Recommendations:" -ForegroundColor Yellow

if (-not $found -or -not $foundFiles) {
    Write-Host "1. WebView2 is NOT properly installed" -ForegroundColor Red
    Write-Host "   Download from: https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section"
    Write-Host "   Install the 'Evergreen Standalone Installer'"
} else {
    Write-Host "1. WebView2 IS installed correctly" -ForegroundColor Green
    Write-Host "   The issue might be with how the app is looking for it"
    Write-Host "   Check if the app is using the correct WebView2 mode in tauri.conf.json"
}

Write-Host ""
