# PowerShell script to refactor device switching to lock-free patterns
$filePath = "libraries\soul-audio-desktop\src\playback.rs"
$content = Get-Content $filePath -Raw

# Replace read-only access patterns
$content = $content -replace 'let state = self\.device_switch_state\.lock\(\)\.unwrap\(\);', 'let state = self.device_switch_state.load();'
$content = $content -replace '\*self\.current_backend\.lock\(\)\.unwrap\(\)', '*self.current_backend.load()'
$content = $content -replace 'self\.current_device\.lock\(\)\.unwrap\(\)\.clone\(\)', 'self.current_device.load().to_string()'
$content = $content -replace 'self\.current_device_id\.lock\(\)\.unwrap\(\)\.clone\(\)', 'self.current_device_id.load().as_ref().clone()'

# Replace write patterns - need to be more careful here
# Pattern: *self.current_backend.lock().unwrap() = backend;
$content = $content -replace '\*self\.current_backend\.lock\(\)\.unwrap\(\) = ([^;]+);', 'self.current_backend.store(Arc::new($1));'

# Pattern: *self.current_device.lock().unwrap() = actual_device_name.clone();
$content = $content -replace '\*self\.current_device\.lock\(\)\.unwrap\(\) = ([^;]+)\.clone\(\);', 'self.current_device.store(Arc::new(Arc::from($1.as_str())));'

# Pattern: *self.current_device_id.lock().unwrap() = new_device_id;
$content = $content -replace '\*self\.current_device_id\.lock\(\)\.unwrap\(\) = ([^;]+);', 'self.current_device_id.store(Arc::new($1));'

# Pattern: let mut state = self.device_switch_state.lock().unwrap();
# Followed by: *state = DeviceSwitchState::...;
# This is trickier - we'll handle it manually

# Write back
$content | Set-Content $filePath -NoNewline

Write-Host "Refactoring complete. Please review changes and handle state machine writes manually."
