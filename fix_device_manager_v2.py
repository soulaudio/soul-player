#!/usr/bin/env python3
"""Fix remaining device manager issues in playback.rs"""

from pathlib import Path

PLAYBACK_FILE = Path(r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs")

def main():
    print(f"Reading {PLAYBACK_FILE}")
    content = PLAYBACK_FILE.read_text(encoding='utf-8')

    # Fix 1: Line 2188-2191 - Double parentheses on get_current_backend()
    content = content.replace(
        """                let backend = self
                    .device_manager.get_current_backend()()
                    .map(|g| *g)
                    .unwrap_or(crate::AudioBackend::Default);""",
        """                let backend = self.device_manager.get_current_backend();"""
    )
    print("[OK] Fixed line 2188-2191 - double parentheses")

    # Fix 2: Line 2192-2196 - Remove .current_device access
    content = content.replace(
        """                let device = self
                    .current_device
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_else(|_| String::from("<unknown>"));""",
        """                let device = self.device_manager.get_current_device();"""
    )
    print("[OK] Fixed line 2192-2196 - current_device access")

    # Fix 3: Lines 2581-2593 - Device manager updates (assignment to getters)
    content = content.replace(
        """        // Update current backend, device, and device ID
        {
            self.device_manager.get_current_backend() = backend;
            *self.device_manager.get_current_device() = actual_device_name.clone();

            // Update device ID for the new device
            let new_device_id = if is_silent_mode {
                None
            } else {
                Some(crate::device_manager::DeviceManager::make_device_id(backend, &actual_device_name))
            };
            *self.current_device_id.lock().unwrap() = new_device_id;
        }""",
        """        // Update device manager state
        self.device_manager.update_device(backend, &actual_device_name, is_silent_mode);"""
    )
    print("[OK] Fixed lines 2581-2593 - device manager updates")

    # Fix 4: Lines 2979-3008 - is_current_device method (delegate to DeviceManager)
    old_is_current = '''    pub fn is_current_device(&self, device_id_or_name: &str) -> bool {
        let current_device = self.device_manager.get_current_device();
        let current_device_id = self.current_device_id.lock().unwrap();

        // Check exact match with device name
        if *current_device == device_id_or_name {
            return true;
        }

        // Check if device_id contains our device name (handles WinRT full IDs)
        if device_id_or_name.contains(current_device.as_str()) {
            return true;
        }

        // Check against our stored device ID
        if let Some(ref stored_id) = *current_device_id {
            if stored_id == device_id_or_name {
                return true;
            }
            // Also check if the provided ID contains our stored ID or vice versa
            if device_id_or_name.contains(stored_id.as_str())
                || stored_id.contains(device_id_or_name)
            {
                return true;
            }
        }

        false
    }'''

    new_is_current = '''    pub fn is_current_device(&self, device_id_or_name: &str) -> bool {
        self.device_manager.is_current_device(device_id_or_name)
    }'''

    content = content.replace(old_is_current, new_is_current)
    print("[OK] Fixed is_current_device method")

    # Write the file
    print(f"\nWriting updated file to {PLAYBACK_FILE}")
    PLAYBACK_FILE.write_text(content, encoding='utf-8')
    print("[OK] File updated successfully")

if __name__ == "__main__":
    main()
