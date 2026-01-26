#!/usr/bin/env python3
"""
Comprehensively fix ALL sqlx::query! calls missing type annotations.
"""

import re
from pathlib import Path

def fix_file_comprehensive(file_path):
    """Add type annotations to all sqlx::query! calls"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    modified = False

    for i in range(len(lines)):
        line = lines[i]

        # Check if this is a let statement with sqlx::query!
        if 'let' in line and '= sqlx::query!(' in line:
            # Check if it already has a type annotation
            if ': Vec<_>' not in line and ': Option<_>' not in line and ': sqlx::' not in line:
                # Need to add type annotation
                # Find the pattern: let VARNAME = sqlx::query!(
                match = re.search(r'(\s+let\s+)(\w+)(\s*=\s*sqlx::query!\()', line)
                if match:
                    # Look ahead to see what method is called
                    lookahead = ''.join(lines[i:min(i+15, len(lines))])

                    if '.fetch_one(' in lookahead:
                        # fetch_one doesn't need type annotation - it can be inferred
                        # Skip this one
                        pass
                    elif '.fetch_optional(' in lookahead:
                        # Use Option<_>
                        lines[i] = line.replace(match.group(0), f"{match.group(1)}{match.group(2)}: Option<_>{match.group(3)}")
                        modified = True
                    elif '.execute(' in lookahead and '.fetch' not in lookahead:
                        # Use SqliteQueryResult
                        lines[i] = line.replace(match.group(0), f"{match.group(1)}{match.group(2)}: sqlx::sqlite::SqliteQueryResult{match.group(3)}")
                        modified = True
                    else:
                        # Default to Vec<_> for .fetch_all()
                        lines[i] = line.replace(match.group(0), f"{match.group(1)}{match.group(2)}: Vec<_>{match.group(3)}")
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
        if fix_file_comprehensive(f):
            print(f"✓ {f}")
            fixed += 1

    print(f"\nFixed {fixed} files")
    print("\nTrying to compile...")

if __name__ == '__main__':
    main()
