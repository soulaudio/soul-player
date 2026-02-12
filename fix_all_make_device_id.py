#!/usr/bin/env python3
"""Replace all remaining DesktopPlayback::make_device_id calls"""

from pathlib import Path
import re

PLAYBACK_FILE = Path(r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs")

def main():
    print(f"Reading {PLAYBACK_FILE}")
    content = PLAYBACK_FILE.read_text(encoding='utf-8')

    # Replace all DesktopPlayback::make_device_id with crate::device_manager::DeviceManager::make_device_id
    original_content = content
    content = content.replace(
        'DesktopPlayback::make_device_id',
        'crate::device_manager::DeviceManager::make_device_id'
    )

    count = content.count('crate::device_manager::DeviceManager::make_device_id') - original_content.count('crate::device_manager::DeviceManager::make_device_id')
    print(f"[OK] Replaced {count} occurrences of DesktopPlayback::make_device_id")

    # Write the file
    print(f"\nWriting updated file to {PLAYBACK_FILE}")
    PLAYBACK_FILE.write_text(content, encoding='utf-8')
    print("[OK] File updated successfully")

if __name__ == "__main__":
    main()
