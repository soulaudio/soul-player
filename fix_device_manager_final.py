#!/usr/bin/env python3
"""Final cleanup - remove duplicate make_device_id method and update test"""

from pathlib import Path

PLAYBACK_FILE = Path(r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs")

def main():
    print(f"Reading {PLAYBACK_FILE}")
    content = PLAYBACK_FILE.read_text(encoding='utf-8')

    # Remove the duplicate make_device_id method (lines 2781-2799)
    old_method = '''    /// Create a unique device ID from backend and device name
    ///
    /// Device ID format: "{backend}::{device_name}"
    /// This provides a unique identifier that can be used to track
    /// which device is currently active and detect device removal events.
    ///
    /// # Example
    /// ```rust,ignore
    /// let device_id = DesktopPlayback::make_device_id(
    ///     AudioBackend::WASAPI,
    ///     "Speakers (Realtek Audio)"
    /// );
    /// // Returns: "WASAPI::Speakers (Realtek Audio)"
    /// ```
    pub fn make_device_id(backend: crate::AudioBackend, device_name: &str) -> String {
        format!("{}::{}", backend.name(), device_name)
    }

    '''

    if old_method in content:
        content = content.replace(old_method, '')
        print("[OK] Removed duplicate make_device_id method from DesktopPlayback")
    else:
        print("[SKIP] Duplicate make_device_id method not found (may already be removed)")

    # Update the test to use DeviceManager::make_device_id
    old_test_call = '''        let device_id = DesktopPlayback::make_device_id(
            crate::AudioBackend::Default,
            "Speakers (Realtek Audio)",
        );'''

    new_test_call = '''        let device_id = crate::device_manager::DeviceManager::make_device_id(
            crate::AudioBackend::Default,
            "Speakers (Realtek Audio)",
        );'''

    if old_test_call in content:
        content = content.replace(old_test_call, new_test_call)
        print("[OK] Updated test to use DeviceManager::make_device_id")
    else:
        print("[SKIP] Test already updated or not found")

    # Also update the second test call if it exists
    old_test_call2 = '''            let device_id = DesktopPlayback::make_device_id(crate::AudioBackend::Asio, "ASIO Device");'''
    new_test_call2 = '''            let device_id = crate::device_manager::DeviceManager::make_device_id(crate::AudioBackend::Asio, "ASIO Device");'''

    if old_test_call2 in content:
        content = content.replace(old_test_call2, new_test_call2)
        print("[OK] Updated second test call")

    # Write the file
    print(f"\nWriting updated file to {PLAYBACK_FILE}")
    PLAYBACK_FILE.write_text(content, encoding='utf-8')
    print("[OK] File updated successfully")

if __name__ == "__main__":
    main()
