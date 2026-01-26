#!/usr/bin/env python3
"""
Remove incorrect Option<_> annotations from fetch_one() calls.
fetch_one() returns a record directly, not Option<Record>.
"""

import re
from pathlib import Path

def fix_file(file_path):
    """Remove Option<_> from fetch_one calls"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    modified = False

    for i in range(len(lines)):
        line = lines[i]

        # Check if this line has "let X: Option<_> = sqlx::query!"
        if 'let' in line and ': Option<_>' in line and '= sqlx::query!(' in line:
            # Look ahead to see if it's fetch_one
            lookahead = ''.join(lines[i:min(i+15, len(lines))])

            if '.fetch_one(' in lookahead:
                # Remove the ": Option<_>" part
                lines[i] = line.replace(': Option<_>', '')
                modified = True

    if modified:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        return True
    return False

def main():
    storage_dir = Path('libraries/soul-storage/src')
    rust_files = list(storage_dir.rglob('*.rs'))

    fixed = 0
    for f in rust_files:
        if fix_file(f):
            print(f"✓ {f}")
            fixed += 1

    print(f"\nFixed {fixed} files")

if __name__ == '__main__':
    main()
