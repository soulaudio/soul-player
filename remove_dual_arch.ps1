# PowerShell script to remove dual architecture systematically
$file = "D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs"
$content = Get-Content $file -Raw

# Step 1: Remove the cfg import for single-writer-manager at the top
$content = $content -replace '#\[cfg\(feature = "single-writer-manager"\)\]\r?\nconst DAC_KEEPALIVE_NOISE', 'const DAC_KEEPALIVE_NOISE'

# Step 2: Remove the cfg wrapper from the single_writer module
$content = $content -replace '#\[cfg\(feature = "single-writer-manager"\)\]\r?\nmod single_writer \{', 'mod single_writer {'

# Step 3: Update publish_snapshot to include new fields
$old_snapshot = @'
            let snapshot = PlaybackStateSnapshot {
                state: self.manager.get_state(),
                current_track: self.manager.get_current_track().cloned(),
                position: self.manager.get_position(),
                volume: self.manager.get_volume(),
                is_muted: self.manager.is_muted(),
                shuffle: self.manager.get_shuffle(),
                repeat: self.manager.get_repeat(),
                queue_length: self.manager.get_queue_length(),
                sample_rate: self.manager.get_sample_rate(),
                timestamp: std::time::Instant::now(),
            };
'@

$new_snapshot = @'
            let snapshot = PlaybackStateSnapshot {
                state: self.manager.get_state(),
                current_track: self.manager.get_current_track().cloned(),
                position: self.manager.get_position(),
                volume: self.manager.get_volume(),
                is_muted: self.manager.is_muted(),
                shuffle: self.manager.get_shuffle(),
                repeat: self.manager.get_repeat(),
                queue_length: self.manager.get_queue_length(),
                queue: self.manager.get_queue().iter().cloned().collect(),
                has_next: self.manager.has_next(),
                has_previous: self.manager.has_previous(),
                sample_rate: self.manager.get_sample_rate(),
                timestamp: std::time::Instant::now(),
            };
'@

$content = $content -replace [regex]::Escape($old_snapshot), $new_snapshot

# Save the file
Set-Content -Path $file -Value $content -NoNewline

Write-Host "Step 1 complete: Updated snapshot and removed some cfg attributes"
