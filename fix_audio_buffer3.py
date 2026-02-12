#!/usr/bin/env python3
"""Fix audio buffer size handling in callbacks"""

import re

# Read the file
with open('libraries/soul-audio-desktop/src/playback.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the buffer size check and usage pattern in both i32 and i16 callbacks
old_pattern = """        // Check buffer size (should never exceed MAX_AUDIO_BUFFER_SAMPLES)
        if data.len() > MAX_AUDIO_BUFFER_SAMPLES {
            tracing::error!(
                "[PLAYBACK] CRITICAL: Audio callback buffer size ({}) exceeds MAX_AUDIO_BUFFER_SAMPLES ({}). Processing truncated.",
                data.len(),
                MAX_AUDIO_BUFFER_SAMPLES
            );
            // Process only what fits to avoid allocation in real-time path
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);"""

new_pattern = """        // Check buffer size and clamp to avoid allocation in real-time path
        let buffer_len = data.len().min(MAX_AUDIO_BUFFER_SAMPLES);
        if data.len() > MAX_AUDIO_BUFFER_SAMPLES {
            tracing::error!(
                "[PLAYBACK] CRITICAL: Audio callback buffer size ({}) exceeds MAX_AUDIO_BUFFER_SAMPLES ({}). Processing truncated to {} samples.",
                data.len(),
                MAX_AUDIO_BUFFER_SAMPLES,
                buffer_len
            );
        }
        let f32_slice = &mut f32_buffer[..buffer_len];
        f32_slice.fill(0.0);"""

content = content.replace(old_pattern, new_pattern)

# Write back
with open('libraries/soul-audio-desktop/src/playback.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("Fixed buffer size handling in audio callbacks!")
