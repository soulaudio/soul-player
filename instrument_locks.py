#!/usr/bin/env python3
"""Script to instrument remaining lock operations in playback.rs"""

import re

# Define lock patterns and their instrumentation names
LOCK_REPLACEMENTS = [
    # Stream locks
    (r'self\.stream\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.stream, "stream_guard")'),

    # Backend locks
    (r'\*?self\.current_backend\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.current_backend, "current_backend")'),

    # Device locks
    (r'self\.current_device\.lock\(\)\.unwrap\(\)\.clone\(\)', 'lock_with_metrics!(self.current_device, "current_device").clone()'),
    (r'self\.current_device\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.current_device, "current_device")'),

    # Device ID locks
    (r'self\.current_device_id\.lock\(\)\.unwrap\(\)\.clone\(\)', 'lock_with_metrics!(self.current_device_id, "current_device_id").clone()'),
    (r'self\.current_device_id\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.current_device_id, "current_device_id")'),

    # Resampling settings locks
    (r'self\.resampling_settings\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.resampling_settings, "resampling_settings")'),

    # Manager locks (for device switch paths)
    (r'self\.manager\.lock\(\)\.unwrap\(\)', 'lock_with_metrics!(self.manager, "manager")'),
]

def main():
    file_path = r'D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs'

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    for pattern, replacement in LOCK_REPLACEMENTS:
        content = re.sub(pattern, replacement, content)

    if content != original:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✓ Instrumented locks in {file_path}")

        # Count changes
        changes = sum(1 for p, r in LOCK_REPLACEMENTS if re.search(p, original))
        print(f"  Made {changes} lock instrumentations")
    else:
        print("No changes needed")

if __name__ == '__main__':
    main()
