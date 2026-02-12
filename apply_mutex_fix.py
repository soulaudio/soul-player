#!/usr/bin/env python3
"""
Apply mutex fix to playback.rs audio callbacks.
Replaces blocking lock().unwrap() with non-blocking try_lock() pattern.
"""

import re

file_path = r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs"

# Read the file
with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# Pattern to replace (i32 callback)
i32_old = r'''        f32_slice\.fill\(0\.0\);

        // Acquire manager lock ONCE for the entire callback
        // This reduces latency by avoiding multiple lock/unlock cycles
        let mut mgr = manager\.lock\(\)\.unwrap\(\);

        // Process any pending commands while holding the lock
        while let Ok\(command\) = command_rx\.try_recv\(\) \{
            tracing::trace!\("\[audio_callback_i32\] Received command: \{:?\}", command\);'''

i32_new = '''        f32_slice.fill(0.0);

        // Acquire manager lock using try_lock to avoid blocking the real-time audio thread
        // If the lock is contended (another thread holds it), output DAC keepalive noise
        // instead of blocking. This prevents audio glitches from mutex contention.
        let Ok(mut mgr) = manager.try_lock() else {
            // Lock contention - output DAC keepalive noise to prevent glitches
            // This is better than blocking which could cause a much longer glitch
            const DAC_KEEPALIVE: f32 = 0.000016; // -96dB - inaudible but keeps DAC active
            for sample in &mut *data {
                // Simple LFSR noise to keep DAC active
                *error_noise_state ^= *error_noise_state << 13;
                *error_noise_state ^= *error_noise_state >> 17;
                *error_noise_state ^= *error_noise_state << 5;
                let noise_f32 = ((*error_noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE;
                *sample = (noise_f32 * 2147483647.0) as i32;
            }
            return;
        };

        // Process any pending commands while holding the lock
        while let Ok(command) = command_rx.try_recv() {
            tracing::trace!("[audio_callback_i32] Received command: {:?}", command);'''

# Pattern to replace (i16 callback)
i16_old = r'''        f32_slice\.fill\(0\.0\);

        // Acquire manager lock ONCE for the entire callback
        // This reduces latency by avoiding multiple lock/unlock cycles
        let mut mgr = manager\.lock\(\)\.unwrap\(\);

        // Process any pending commands while holding the lock
        while let Ok\(command\) = command_rx\.try_recv\(\) \{
            if let Err\(e\) =
                Self::process_command_with_lock\(command, &mut mgr, event_tx, track_loader\)'''

i16_new = '''        f32_slice.fill(0.0);

        // Acquire manager lock using try_lock to avoid blocking the real-time audio thread
        // If the lock is contended (another thread holds it), output DAC keepalive noise
        // instead of blocking. This prevents audio glitches from mutex contention.
        let Ok(mut mgr) = manager.try_lock() else {
            // Lock contention - output DAC keepalive noise to prevent glitches
            // This is better than blocking which could cause a much longer glitch
            const DAC_KEEPALIVE: f32 = 0.000016; // -96dB - inaudible but keeps DAC active
            for sample in &mut *data {
                // Simple LFSR noise to keep DAC active
                *error_noise_state ^= *error_noise_state << 13;
                *error_noise_state ^= *error_noise_state >> 17;
                *error_noise_state ^= *error_noise_state << 5;
                let noise_f32 = ((*error_noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE;
                *sample = (noise_f32 * 32767.0) as i16;
            }
            return;
        };

        // Process any pending commands while holding the lock
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) =
                Self::process_command_with_lock(command, &mut mgr, event_tx, track_loader)'''

# Apply replacements
content_new = re.sub(i32_old, i32_new, content)
content_new = re.sub(i16_old, i16_new, content_new)

# Write back
with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content_new)

print("Mutex fix applied successfully!")
