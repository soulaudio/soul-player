# PowerShell script to remove duplicated code from playback.rs

$filePath = "D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs"
$content = Get-Content -Path $filePath -Raw

# 1. Add import after the existing imports
$pattern1 = '(use crate::error::Result;\s+use std::sync::atomic::\{AtomicU64, Ordering\};)'
$replacement1 = '$1' + "`nuse crate::stream_manager::{get_stream_config, CallbackDropGuard, StreamStartEnvelope, GLOBAL_I32_CALLBACK_COUNTER};"
$content = $content -replace $pattern1, $replacement1

# 2. Remove GLOBAL_I32_CALLBACK_COUNTER definition
$pattern2 = '\r?\n/// Global counter for I32 \(ASIO\) callbacks - used for diagnostics[\s\S]*?static GLOBAL_I32_CALLBACK_COUNTER: AtomicU64 = AtomicU64::new\(0\);'
$content = $content -replace $pattern2, ''

# 3. Remove StreamStartEnvelope struct and impl
$pattern3 = '\r?\n/// Stream-level fade envelope[\s\S]*?impl StreamStartEnvelope \{[\s\S]*?    \}\s*\}'
$content = $content -replace $pattern3, ''

# 4. Remove CallbackDropGuard struct and Drop impl
$pattern4 = '\r?\n/// Drop guard for detecting[\s\S]*?impl Drop for CallbackDropGuard \{[\s\S]*?    \}\s*\}'
$content = $content -replace $pattern4, ''

# 5. Remove get_stream_config function
$pattern5 = '\r?\n    /// Get stream configuration[\s\S]*?    fn get_stream_config\(device: &Device\)[\s\S]*?        Ok\(\(stream_config, sample_format\)\)\s*\n    \}'
$content = $content -replace $pattern5, ''

# 6. Replace Self::get_stream_config with get_stream_config
$content = $content -replace 'Self::get_stream_config', 'get_stream_config'

# Write the modified content back
Set-Content -Path $filePath -Value $content -NoNewline

Write-Host "Successfully removed duplicated code from playback.rs"
Write-Host "Changes:"
Write-Host "- Added import from stream_manager"
Write-Host "- Removed GLOBAL_I32_CALLBACK_COUNTER"
Write-Host "- Removed StreamStartEnvelope"
Write-Host "- Removed CallbackDropGuard"
Write-Host "- Removed get_stream_config function"
Write-Host "- Updated function calls to use module version"
