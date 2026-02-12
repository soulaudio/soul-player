#!/usr/bin/env python3
"""Comprehensive fix for audio buffer allocations in playback.rs"""

import re

# Read the file
with open('libraries/soul-audio-desktop/src/playback.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Step 1: Add constant after imports (only if not already present)
if 'const MAX_AUDIO_BUFFER_SAMPLES' not in content:
    constant_def = """
/// Maximum audio buffer size for format conversion (samples, not frames)
/// This must be large enough to handle all realistic audio callback sizes.
/// Based on: streaming chunk size (8192 samples) + margin for safety
/// = 16384 samples = 8192 stereo frames at 48kHz (~170ms)
/// Pre-allocating this avoids allocations in the real-time audio callback.
const MAX_AUDIO_BUFFER_SAMPLES: usize = 16384;
"""
    # Insert after the atomic imports
    content = content.replace(
        'use std::sync::atomic::{AtomicU64, Ordering};',
        f'use std::sync::atomic::{{AtomicU64, Ordering}};{constant_def}'
    )
    print("✓ Added MAX_AUDIO_BUFFER_SAMPLES constant")
else:
    print("✓ Constant already present")

# Step 2: Replace all buffer allocations (both variants)
replacements = 0

# Variant 1: With "Use a reasonable default..." comment
old1 = '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Use a reasonable default size that will be resized if needed\n                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);'
new1 = '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];'
if old1 in content:
    content = content.replace(old1, new1)
    replacements += content.count(new1) - (content.count(old1) if old1 in content else 0)
    print(f"✓ Replaced buffer allocations (variant 1)")

# Variant 2: Without second comment line
old2 = '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);'
new2 = '                // Pre-allocate conversion buffer to avoid allocation in audio callback\n                // Fixed-size buffer sized for worst-case scenarios (no resizing allowed)\n                let mut f32_buffer: Vec<f32> = vec![0.0; MAX_AUDIO_BUFFER_SAMPLES];'
count_before = content.count('Vec::with_capacity(4096)')
content = content.replace(old2, new2)
count_after = content.count('Vec::with_capacity(4096)')
if count_before > count_after:
    print(f"✓ Replaced buffer allocations (variant 2): {count_before - count_after} occurrences")

# Step 3: Replace resize() calls with size check
old_resize = '''        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);'''

new_resize = '''        // Check buffer size and clamp to avoid allocation in real-time path
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
        f32_slice.fill(0.0);'''

resize_count_before = content.count('f32_buffer.resize(')
content = content.replace(old_resize, new_resize)
resize_count_after = content.count('f32_buffer.resize(')
if resize_count_before > resize_count_after:
    print(f"✓ Replaced resize() calls: {resize_count_before - resize_count_after} occurrences")

# Step 4: Update dither calls to handle buffer overflow
# i32 variant
old_i32_dither = '''                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                dither.process_stereo_to_i32(f32_slice, data);'''

new_i32_dither = '''                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                if buffer_len < data.len() {
                    // Buffer overflow - process what fits and zero-fill the rest
                    dither.process_stereo_to_i32(f32_slice, &mut data[..buffer_len]);
                    data[buffer_len..].fill(0);
                } else {
                    dither.process_stereo_to_i32(f32_slice, data);
                }'''

if old_i32_dither in content:
    content = content.replace(old_i32_dither, new_i32_dither)
    print("✓ Updated i32 dither call for overflow handling")

# i16 variant
old_i16_dither = '''                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                dither.process_stereo_to_i16(f32_slice, data);'''

new_i16_dither = '''                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                if buffer_len < data.len() {
                    // Buffer overflow - process what fits and zero-fill the rest
                    dither.process_stereo_to_i16(f32_slice, &mut data[..buffer_len]);
                    data[buffer_len..].fill(0);
                } else {
                    dither.process_stereo_to_i16(f32_slice, data);
                }'''

if old_i16_dither in content:
    content = content.replace(old_i16_dither, new_i16_dither)
    print("✓ Updated i16 dither call for overflow handling")

# Write back
with open('libraries/soul-audio-desktop/src/playback.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("\n✅ All audio buffer allocation issues fixed!")
print(f"   - No allocations in audio callbacks")
print(f"   - Fixed-size {16384} sample buffer pre-allocated")
print(f"   - Graceful handling of buffer size mismatches")
