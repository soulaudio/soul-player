#!/usr/bin/env python3
"""Fix audio buffer allocations in playback.rs"""

import re

# Read the file
with open('libraries/soul-audio-desktop/src/playback.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix ALL occurrences of Vec::with_capacity(4096) buffer allocations
content = content.replace(
    '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Use a reasonable default size that will be resized if needed\n                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);',
    '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];'
)

# Also fix the version without the second comment line
content = content.replace(
    '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);',
    '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];'
)

# Write back
with open('libraries/soul-audio-desktop/src/playback.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("Fixed all audio buffer allocations!")
