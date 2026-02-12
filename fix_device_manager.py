#!/usr/bin/env python3
"""Replace raw mutex device state fields with DeviceManager in playback.rs"""

import re
from pathlib import Path

PLAYBACK_FILE = Path(r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs")

def main():
    print(f"Reading {PLAYBACK_FILE}")
    content = PLAYBACK_FILE.read_text(encoding='utf-8')
    original_content = content

    # Fix 1: Replace initialization code (lines 635-647)
    old_init = r"""        let stream = Arc::new\(Mutex::new\(stream_option\)\);
        let current_backend = Arc::new\(Mutex::new\(backend\)\);
        let current_device = Arc::new\(Mutex::new\(actual_device_name\.clone\(\)\)\);

        // Create device ID \(backend \+ device name as unique identifier\)
        let device_id = if is_silent_mode \{
            None
        \} else \{
            Some\(Self::make_device_id\(backend, &actual_device_name\)\)
        \};
        let current_device_id = Arc::new\(Mutex::new\(device_id\)\);

        let current_sample_rate = Arc::new\(std::sync::atomic::AtomicU32::new\(sample_rate\)\);
        let resampling_settings = Arc::new\(Mutex::new\(ResamplingSettings::default\(\)\)\);"""

    new_init = """        let stream = Arc::new(Mutex::new(stream_option));

        // Create device manager and update with initial device state
        let device_manager = Arc::new(crate::device_manager::DeviceManager::new());
        device_manager.update_device(backend, &actual_device_name, is_silent_mode);

        let current_sample_rate = Arc::new(std::sync::atomic::AtomicU32::new(sample_rate));
        let resampling_settings = Arc::new(Mutex::new(ResamplingSettings::default()));"""

    content = re.sub(old_init, new_init, content, count=1)
    if content != original_content:
        print("[OK] Fixed initialization code")
        original_content = content

    # Fix 2: Replace struct initialization (lines 669-683)
    old_struct_init = r"""        Ok\(Self \{
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            current_backend,
            current_device,
            current_device_id,
            current_sample_rate,
            resampling_settings,
            track_loader,
            device_switch_state: Arc::new\(Mutex::new\(DeviceSwitchState::Idle\)\),
            device_switch_config: DeviceSwitchConfig::default\(\),
        \}\)"""

    new_struct_init = """        Ok(Self {
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            device_manager,
            current_sample_rate,
            resampling_settings,
            track_loader,
            device_switch_state: Arc::new(Mutex::new(DeviceSwitchState::Idle)),
            device_switch_config: DeviceSwitchConfig::default(),
        })"""

    content = re.sub(old_struct_init, new_struct_init, content, count=1)
    if content != original_content:
        print("[OK] Fixed struct initialization")
        original_content = content

    # Fix 3: Replace get_current_backend method
    content = re.sub(
        r'    pub fn get_current_backend\(&self\) -> crate::AudioBackend \{\s*\*self\.current_backend\.lock\(\)\.unwrap\(\)\s*\}',
        '''    pub fn get_current_backend(&self) -> crate::AudioBackend {
        self.device_manager.get_current_backend()
    }''',
        content,
        flags=re.MULTILINE
    )
    if content != original_content:
        print("[OK] Fixed get_current_backend method")
        original_content = content

    # Fix 4: Replace get_current_device method
    content = re.sub(
        r'    pub fn get_current_device\(&self\) -> String \{\s*self\.current_device\.lock\(\)\.unwrap\(\)\.clone\(\)\s*\}',
        '''    pub fn get_current_device(&self) -> String {
        self.device_manager.get_current_device()
    }''',
        content,
        flags=re.MULTILINE
    )
    if content != original_content:
        print("[OK] Fixed get_current_device method")
        original_content = content

    # Fix 5: Replace get_current_device_id method
    content = re.sub(
        r'    pub fn get_current_device_id\(&self\) -> Option<String> \{\s*self\.current_device_id\.lock\(\)\.unwrap\(\)\.clone\(\)\s*\}',
        '''    pub fn get_current_device_id(&self) -> Option<String> {
        self.device_manager.get_current_device_id()
    }''',
        content,
        flags=re.MULTILINE
    )
    if content != original_content:
        print("[OK] Fixed get_current_device_id method")
        original_content = content

    # Fix 6: Replace is_current_device method
    old_is_current = r'''    pub fn is_current_device\(&self, device_id_or_name: &str\) -> bool \{
        let current_device = self\.current_device\.lock\(\)\.unwrap\(\);
        let current_device_id = self\.current_device_id\.lock\(\)\.unwrap\(\);

        // Check exact match with device name
        if \*current_device == device_id_or_name \{
            return true;
        \}

        // Check if device_id contains our device name \(handles WinRT full IDs\)
        if device_id_or_name\.contains\(current_device\.as_str\(\)\) \{
            return true;
        \}

        // Check against our stored device ID
        if let Some\(ref stored_id\) = \*current_device_id \{
            if stored_id == device_id_or_name \{
                return true;
            \}
            // Also check if the provided ID contains our stored ID or vice versa
            if device_id_or_name\.contains\(stored_id\) \|\| stored_id\.contains\(device_id_or_name\) \{
                return true;
            \}
        \}

        false
    \}'''

    new_is_current = '''    pub fn is_current_device(&self, device_id_or_name: &str) -> bool {
        self.device_manager.is_current_device(device_id_or_name)
    }'''

    content = re.sub(old_is_current, new_is_current, content, flags=re.MULTILINE)
    if content != original_content:
        print("[OK] Fixed is_current_device method")
        original_content = content

    # Fix 7: Replace device updates in switch_device method (around line 2543-2552)
    old_switch_update = r'''\*self\.current_backend\.lock\(\)\.unwrap\(\) = backend;
            \*self\.current_device\.lock\(\)\.unwrap\(\) = actual_device_name\.clone\(\);

            // Update device ID
            let new_device_id = if is_silent_mode \{
                None
            \} else \{
                Some\(Self::make_device_id\(backend, &actual_device_name\)\)
            \};
            \*self\.current_device_id\.lock\(\)\.unwrap\(\) = new_device_id;'''

    new_switch_update = '''// Update device manager state
            self.device_manager.update_device(backend, &actual_device_name, is_silent_mode);'''

    content = re.sub(old_switch_update, new_switch_update, content)
    if content != original_content:
        print("[OK] Fixed switch_device update calls")
        original_content = content

    # Fix 8: Replace all self.current_backend.lock().unwrap() reads with self.device_manager.get_current_backend()
    content = re.sub(
        r'\*self\.current_backend\.lock\(\)\.unwrap\(\)',
        'self.device_manager.get_current_backend()',
        content
    )
    if content != original_content:
        print("[OK] Fixed current_backend reads")
        original_content = content

    # Fix 9: Replace all self.current_device.lock().unwrap().clone() with self.device_manager.get_current_device()
    content = re.sub(
        r'self\.current_device\.lock\(\)\.unwrap\(\)\.clone\(\)',
        'self.device_manager.get_current_device()',
        content
    )
    if content != original_content:
        print("[OK] Fixed current_device reads")
        original_content = content

    # Fix 10: Replace any remaining self.current_device.lock().unwrap()
    content = re.sub(
        r'self\.current_device\.lock\(\)\.unwrap\(\)',
        'self.device_manager.get_current_device()',
        content
    )

    # Fix 11: Replace .current_backend field access in command processing
    content = re.sub(
        r'\.current_backend\s*\.lock',
        '.device_manager.get_current_backend()',
        content
    )
    if content != original_content:
        print("[OK] Fixed .current_backend field accesses")
        original_content = content

    # Fix 12: Replace .current_device field access
    content = re.sub(
        r'\.current_device\s*\.lock\(\)\.unwrap\(\)\s*\.map\(\|g\| g\.clone\(\)\)',
        '.device_manager.get_current_device()',
        content
    )
    if content != original_content:
        print("[OK] Fixed .current_device field accesses")
        original_content = content

    # Fix 13: Remove the make_device_id static method (now in DeviceManager)
    old_make_device_id = r'''    /// Create a unique device ID from backend and device name
    ///
    /// Device ID format: "\{backend\}::\{device_name\}"
    /// This provides a unique identifier that can be used to track
    /// which device is currently active and detect device removal events\.
    ///
    /// # Example
    /// ```rust,ignore
    /// let device_id = DesktopPlayback::make_device_id\(
    ///     AudioBackend::Default,
    ///     "Speakers \(Realtek Audio\)"
    /// \);
    /// // Returns: "WASAPI::Speakers \(Realtek Audio\)" on Windows
    /// // Returns: "CoreAudio::Speakers \(Realtek Audio\)" on macOS
    /// // Returns: "ALSA::Speakers \(Realtek Audio\)" on Linux
    /// ```
    pub fn make_device_id\(backend: crate::AudioBackend, device_name: &str\) -> String \{
        format!\("\{\}::\{\}", backend\.name\(\), device_name\)
    \}'''

    content = re.sub(old_make_device_id, '', content, flags=re.MULTILINE)
    if content != original_content:
        print("[OK] Removed duplicate make_device_id method")
        original_content = content

    # Fix 14: Update calls to Self::make_device_id to DeviceManager::make_device_id
    content = re.sub(
        r'Self::make_device_id\(',
        'crate::device_manager::DeviceManager::make_device_id(',
        content
    )
    if content != original_content:
        print("[OK] Updated make_device_id calls to use DeviceManager")

    # Write the file
    print(f"\nWriting updated file to {PLAYBACK_FILE}")
    PLAYBACK_FILE.write_text(content, encoding='utf-8')
    print("[OK] File updated successfully")

if __name__ == "__main__":
    main()
