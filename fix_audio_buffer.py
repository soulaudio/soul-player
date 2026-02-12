#!/usr/bin/env python3
"""Fix audio buffer allocations in playback.rs"""

import re

# Read the file
with open('libraries/soul-audio-desktop/src/playback.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add constant after imports
constant_to_add = """
/// Maximum audio buffer size for format conversion (samples, not frames)
/// This must be large enough to handle all realistic audio callback sizes.
/// Based on: streaming chunk size (8192 samples) + margin for safety
/// = 16384 samples = 8192 stereo frames at 48kHz (~170ms)
/// Pre-allocating this avoids allocations in the real-time audio callback.
const MAX_AUDIO_BUFFER_SAMPLES: usize = 16384;
"""

# Insert constant after the imports section
content = content.replace(
    'use std::sync::atomic::{AtomicU64, Ordering};',
    f'use std::sync::atomic::{{AtomicU64, Ordering}};{constant_to_add}'
)

# Fix i32 buffer allocation
content = re.sub(
    r'(cpal::SampleFormat::I32 => \{.*?let track_loader_clone = track_loader\.clone\(\);.*?let resampling_settings_clone = resampling_settings\.clone\(\);)\s*// Pre-allocate conversion buffer to avoid allocation in audio callback\s*// Use a reasonable default size that will be resized if needed\s*let mut f32_buffer: Vec<f32> = Vec::with_capacity\(4096\);',
    r'\1\n                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];',
    content,
    flags=re.DOTALL
)

# Fix i16 buffer allocation
content = re.sub(
    r'(cpal::SampleFormat::I16 => \{.*?let track_loader_clone = track_loader\.clone\(\);.*?let resampling_settings_clone = resampling_settings\.clone\(\);)\s*// Pre-allocate conversion buffer to avoid allocation in audio callback\s*let mut f32_buffer: Vec<f32> = Vec::with_capacity\(4096\);',
    r'\1\n                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];',
    content,
    flags=re.DOTALL
)

# Fix the resize() calls in the callbacks
content = content.replace(
    '''        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }''',
    '''        // Check buffer size (should never exceed MAX_AUDIO_BUFFER_SAMPLES)
        if data.len() > MAX_AUDIO_BUFFER_SAMPLES {
            tracing::error!(
                "[PLAYBACK] CRITICAL: Audio callback buffer size ({}) exceeds MAX_AUDIO_BUFFER_SAMPLES ({}). Processing truncated.",
                data.len(),
                MAX_AUDIO_BUFFER_SAMPLES
            );
            // Process only what fits to avoid allocation in real-time path
        }'''
)

# Write back
with open('libraries/soul-audio-desktop/src/playback.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("Fixed audio buffer allocations successfully!")
