#!/usr/bin/env python3
"""
Fix all remaining self.manager references in playback.rs
"""

import re

file_path = r"D:\dev\soulaudio\soul-player\libraries\soul-audio-desktop\src\playback.rs"

with open(file_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

print(f"Processing {len(lines)} lines...")

# Track changes
changes = 0

for i, line in enumerate(lines):
    if 'self.manager' in line and 'self.manager.get' in line:
        # These are the getter methods that need updating
        if 'self.manager.lock().unwrap().get_state()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_state()', 'self.state_snapshot.load().state')
            changes += 1
            print(f"Line {i+1}: Fixed get_state()")
        elif 'self.manager.lock().unwrap().get_current_track()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_current_track().cloned()', 'self.state_snapshot.load().current_track.clone()')
            changes += 1
            print(f"Line {i+1}: Fixed get_current_track()")
        elif 'self.manager.lock().unwrap().get_position()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_position()', 'self.state_snapshot.load().position')
            changes += 1
            print(f"Line {i+1}: Fixed get_position()")
        elif 'self.manager.lock().unwrap().get_volume()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_volume()', 'self.state_snapshot.load().volume')
            changes += 1
            print(f"Line {i+1}: Fixed get_volume()")
        elif 'self.manager.lock().unwrap().get_shuffle_mode()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_shuffle_mode()', 'self.state_snapshot.load().shuffle')
            changes += 1
            print(f"Line {i+1}: Fixed get_shuffle_mode()")
        elif 'self.manager.lock().unwrap().get_repeat()' in line:
            lines[i] = line.replace('self.manager.lock().unwrap().get_repeat()', 'self.state_snapshot.load().repeat')
            changes += 1
            print(f"Line {i+1}: Fixed get_repeat()")

# Handle multi-line get_queue pattern
new_lines = []
skip_until = -1
for i, line in enumerate(lines):
    if i < skip_until:
        continue

    if 'self.manager' in line and '.get_queue()' in lines[min(i+2, len(lines)-1)]:
        # This is the multi-line get_queue pattern
        new_lines.append('        self.state_snapshot.load().queue.clone()\n')
        # Skip the next 4 lines
        skip_until = i + 5
        changes += 1
        print(f"Line {i+1}: Fixed multi-line get_queue()")
    else:
        new_lines.append(line)

lines = new_lines

# Remove get_manager_mut and get_playback_manager methods entirely
final_lines = []
skip_until = -1
for i, line in enumerate(lines):
    if i < skip_until:
        continue

    # Check for get_manager_mut method
    if 'pub fn get_manager_mut(&self)' in line:
        # Skip this method and the next 2 lines (method body + closing brace + blank line)
        skip_until = i + 3
        # Also skip the doc comment lines before it
        if i >= 3 and '/// Get mutable reference to PlaybackManager' in final_lines[-3]:
            final_lines = final_lines[:-3]
        changes += 1
        print(f"Line {i+1}: Removed get_manager_mut() method")
        continue

    # Check for get_playback_manager method
    if 'pub fn get_playback_manager(&self)' in line:
        # Skip this method and the next 2 lines
        skip_until = i + 3
        # Also skip the doc comment lines before it
        if i >= 3 and '/// Get the playback manager' in final_lines[-3]:
            final_lines = final_lines[:-3]
        changes += 1
        print(f"Line {i+1}: Removed get_playback_manager() method")
        continue

    final_lines.append(line)

# Write back
with open(file_path, 'w', encoding='utf-8', newline='') as f:
    f.writelines(final_lines)

print(f"\nMade {changes} changes")
print("Done!")
