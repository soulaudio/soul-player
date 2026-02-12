#!/usr/bin/env python3
"""
Script to replace timestamp patterns with helper functions
"""

import os
import re
from pathlib import Path

SOUL_STORAGE = Path(r"D:\dev\soulaudio\soul-player\libraries\soul-storage\src")

# Files to update
FILES_TO_UPDATE = [
    "managed_library_settings/mod.rs",
    "playback_state/mod.rs",
    "scan_progress/mod.rs",
    "sources/mod.rs",
    "window_state/mod.rs",
    "loudness/mod.rs",
    "shortcuts/mod.rs",
    "settings/mod.rs",
    "tracks/mod.rs",
    "external_file_settings/mod.rs",
    "library_sources/mod.rs",
    "users/mod.rs",
]

def update_file(file_path):
    """Update a single file with timestamp helpers"""
    print(f"Updating {file_path}...")

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # Check if we need the import
    needs_now_import = 'chrono::Utc::now().timestamp()' in content
    needs_iso_import = 'DateTime::from_timestamp' in content and '.to_rfc3339()' in content
    needs_datetime_import = 'DateTime::from_timestamp' in content and not '.to_rfc3339()' in content

    # Add imports if needed
    if needs_now_import or needs_iso_import or needs_datetime_import:
        # Find the use statements section
        use_match = re.search(r'((?:use [^;]+;[\s]*)+)', content)
        if use_match:
            use_section = use_match.group(1)
            new_imports = []

            if needs_now_import:
                new_imports.append('now_timestamp')
            if needs_iso_import:
                new_imports.append('timestamp_to_iso8601')
            if needs_datetime_import:
                new_imports.append('timestamp_to_datetime')

            import_line = f"use crate::utils::time::{{{', '.join(new_imports)}}};\n"

            # Insert after the first use statement
            insert_pos = use_match.start(1)
            content = content[:insert_pos] + import_line + content[insert_pos:]

    # Replace patterns
    # Pattern 1: chrono::Utc::now().timestamp() -> now_timestamp()
    content = re.sub(
        r'chrono::Utc::now\(\)\.timestamp\(\)',
        'now_timestamp()',
        content
    )

    # Pattern 2: DateTime::from_timestamp(..., 0).map(|dt| dt.to_rfc3339()).unwrap_or_default()
    # This is complex - match variable assignments
    content = re.sub(
        r'chrono::DateTime::from_timestamp\(([^,]+),\s*0\)\s*\.map\(\|dt\|\s*dt\.to_rfc3339\(\)\)\s*\.unwrap_or_default\(\)',
        r'timestamp_to_iso8601(\1)',
        content
    )

    # Pattern 3: DateTime::from_timestamp in row mapping (without to_rfc3339)
    # Leave these for manual review as they might not need conversion

    if content != original:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  [+] Updated {file_path}")
        return True
    else:
        print(f"  [-] No changes needed for {file_path}")
        return False

def main():
    updated_count = 0

    for rel_path in FILES_TO_UPDATE:
        file_path = SOUL_STORAGE / rel_path
        if file_path.exists():
            if update_file(file_path):
                updated_count += 1
        else:
            print(f"  [!] File not found: {file_path}")

    print(f"\n[+] Updated {updated_count} files")

if __name__ == '__main__':
    main()
