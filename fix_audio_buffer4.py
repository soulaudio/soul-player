#!/usr/bin/env python3
"""Fix audio buffer overflow handling in i32 and i16 callbacks"""

import re

# Read the file
with open('libraries/soul-audio-desktop/src/playback.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix i32 callback - process_stereo_to_i32 call
old_i32_pattern = """                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                dither.process_stereo_to_i32(f32_slice, data);"""

new_i32_pattern = """                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                if buffer_len < data.len() {
                    // Buffer overflow - process what fits and zero-fill the rest
                    dither.process_stereo_to_i32(f32_slice, &mut data[..buffer_len]);
                    data[buffer_len..].fill(0);
                } else {
                    dither.process_stereo_to_i32(f32_slice, data);
                }"""

content = content.replace(old_i32_pattern, new_i32_pattern)

# Fix i16 callback - process_stereo_to_i16 call
old_i16_pattern = """                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                dither.process_stereo_to_i16(f32_slice, data);"""

new_i16_pattern = """                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                if buffer_len < data.len() {
                    // Buffer overflow - process what fits and zero-fill the rest
                    dither.process_stereo_to_i16(f32_slice, &mut data[..buffer_len]);
                    data[buffer_len..].fill(0);
                } else {
                    dither.process_stereo_to_i16(f32_slice, data);
                }"""

content = content.replace(old_i16_pattern, new_i16_pattern)

# Write back
with open('libraries/soul-audio-desktop/src/playback.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("Fixed buffer overflow handling in i32 and i16 callbacks!")
