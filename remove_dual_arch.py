#!/usr/bin/env python3
"""
Script to remove dual architecture from playback.rs
Keeps only single-writer implementation
"""

import re

file_path = r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs"

with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

print("Original file size:", len(content))

# Step 1: Remove #[cfg(not(feature = "single-writer-manager"))] manager field from struct
content = re.sub(
    r'    /// Playback manager \(shared with audio thread\) - legacy Arc<Mutex<>> version\r?\n'
    r'    #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'    manager: Arc<Mutex<PlaybackManager>>,\r?\n\r?\n',
    '',
    content
)

# Step 2: Remove #[cfg(feature = "single-writer-manager")] from state_snapshot field
content = re.sub(
    r'    /// State snapshot for lock-free queries - single-writer version\r?\n'
    r'    #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'    state_snapshot:',
    '    /// State snapshot for lock-free queries\n'
    '    state_snapshot:',
    content
)

# Step 3: Remove all #[cfg(not(feature = "single-writer-manager"))] blocks in new() method
# This is the manager creation block
content = re.sub(
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        let manager = \{[^}]+\};',
    '',
    content,
    flags=re.DOTALL
)

# Step 4: Remove #[cfg(feature = "single-writer-manager")] from state_snapshot initialization
content = re.sub(
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        let state_snapshot = \{',
    '        let state_snapshot = {',
    content
)

# Step 5: Remove #[cfg(not(feature = "single-writer-manager"))] from create_audio_stream call
content = re.sub(
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        let \(stream_option, actual_device_name, sample_rate\) = Self::create_audio_stream\([^;]+\);',
    '',
    content,
    flags=re.DOTALL
)

# Step 6: Remove #[cfg(feature = "single-writer-manager")] from create_audio_stream_single_writer call
content = re.sub(
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        let \(stream_option, actual_device_name, sample_rate\) =\r?\n'
    r'            Self::create_audio_stream_single_writer\(',
    '        let (stream_option, actual_device_name, sample_rate) =\n'
    '            Self::create_audio_stream_single_writer(',
    content
)

print("\nStep 1-6 complete: Removed dual architecture from struct and new() method")

# Step 7: Update get_state() to only use state_snapshot
content = re.sub(
    r'    pub fn get_state\(\&self\) -> soul_playback::PlaybackState \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_state\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.state\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_state(&self) -> soul_playback::PlaybackState {\n'
    '        self.state_snapshot.load().state\n'
    '    }',
    content
)

# Step 8: Update get_current_track()
content = re.sub(
    r'    pub fn get_current_track\(\&self\) -> Option<QueueTrack> \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_current_track\(\)\.cloned\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.current_track\.clone\(\)\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_current_track(&self) -> Option<QueueTrack> {\n'
    '        self.state_snapshot.load().current_track.clone()\n'
    '    }',
    content
)

# Step 9: Update get_position()
content = re.sub(
    r'    pub fn get_position\(\&self\) -> std::time::Duration \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_position\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.position\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_position(&self) -> std::time::Duration {\n'
    '        self.state_snapshot.load().position\n'
    '    }',
    content
)

# Step 10: Update get_queue() - only uses manager currently, needs to use snapshot
content = re.sub(
    r'    pub fn get_queue\(\&self\) -> Vec<soul_playback::QueueTrack> \{\r?\n'
    r'        self\.manager\r?\n'
    r'            \.lock\(\)\.unwrap\(\)\r?\n'
    r'            \.get_queue\(\)\r?\n'
    r'            \.into_iter\(\)\r?\n'
    r'            \.cloned\(\)\r?\n'
    r'            \.collect\(\)\r?\n'
    r'    \}',
    '    pub fn get_queue(&self) -> Vec<soul_playback::QueueTrack> {\n'
    '        self.state_snapshot.load().queue.clone()\n'
    '    }',
    content
)

# Step 11: Update has_next()
content = re.sub(
    r'    pub fn has_next\(\&self\) -> bool \{\r?\n'
    r'        self\.manager\.lock\(\)\.unwrap\(\)\.has_next\(\)\r?\n'
    r'    \}',
    '    pub fn has_next(&self) -> bool {\n'
    '        self.state_snapshot.load().has_next\n'
    '    }',
    content
)

# Step 12: Update has_previous()
content = re.sub(
    r'    pub fn has_previous\(\&self\) -> bool \{\r?\n'
    r'        self\.manager\.lock\(\)\.unwrap\(\)\.has_previous\(\)\r?\n'
    r'    \}',
    '    pub fn has_previous(&self) -> bool {\n'
    '        self.state_snapshot.load().has_previous\n'
    '    }',
    content
)

# Step 13: Update get_volume()
content = re.sub(
    r'    pub fn get_volume\(\&self\) -> u8 \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_volume\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.volume\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_volume(&self) -> u8 {\n'
    '        self.state_snapshot.load().volume\n'
    '    }',
    content
)

# Step 14: Update get_shuffle_mode()
content = re.sub(
    r'    pub fn get_shuffle_mode\(\&self\) -> soul_playback::ShuffleMode \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_shuffle_mode\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.shuffle\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_shuffle_mode(&self) -> soul_playback::ShuffleMode {\n'
    '        self.state_snapshot.load().shuffle\n'
    '    }',
    content
)

# Step 15: Update get_repeat_mode()
content = re.sub(
    r'    pub fn get_repeat_mode\(\&self\) -> soul_playback::RepeatMode \{\r?\n'
    r'        #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.manager\.lock\(\)\.unwrap\(\)\.get_repeat\(\)\r?\n'
    r'        \}\r?\n\r?\n'
    r'        #\[cfg\(feature = "single-writer-manager"\)\]\r?\n'
    r'        \{\r?\n'
    r'            self\.state_snapshot\.load\(\)\.repeat\r?\n'
    r'        \}\r?\n'
    r'    \}',
    '    pub fn get_repeat_mode(&self) -> soul_playback::RepeatMode {\n'
    '        self.state_snapshot.load().repeat\n'
    '    }',
    content
)

# Step 16: Remove get_manager_mut() - only exists in legacy mode
content = re.sub(
    r'    /// Get mutable reference to PlaybackManager\r?\n'
    r'    ///\r?\n'
    r'    /// Note: Not available in single-writer mode \(manager is owned by audio callback\)\r?\n'
    r'    #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'    pub fn get_manager_mut\(\&self\) -> std::sync::MutexGuard<\'_, soul_playback::PlaybackManager> \{\r?\n'
    r'        self\.manager\.lock\(\)\.unwrap\(\)\r?\n'
    r'    \}\r?\n\r?\n',
    '',
    content
)

# Step 17: Remove get_playback_manager() - only exists in legacy mode
content = re.sub(
    r'    /// Get the playback manager \(for batch loading\)\r?\n'
    r'    ///\r?\n'
    r'    /// Note: Not available in single-writer mode \(manager is owned by audio callback\)\r?\n'
    r'    #\[cfg\(not\(feature = "single-writer-manager"\)\)\]\r?\n'
    r'    pub fn get_playback_manager\(\&self\) -> &Arc<Mutex<soul_playback::PlaybackManager>> \{\r?\n'
    r'        &self\.manager\r?\n'
    r'    \}\r?\n\r?\n',
    '',
    content
)

print("Step 7-17 complete: Updated all getter methods to use state_snapshot")

# Save the file
with open(file_path, 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print(f"\nFile updated successfully! New size: {len(content)}")
print(f"Removed {38312 - len(content)} characters")
