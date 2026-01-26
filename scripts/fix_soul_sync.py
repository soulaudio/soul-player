#!/usr/bin/env python3
"""
Fix soul-sync library SQLx type annotations
"""

import re
from pathlib import Path

def fix_file(file_path):
    """Add type annotations to soul-sync files"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    modified = False

    for i in range(len(lines)):
        line = lines[i]

        # Check if this is a let statement with sqlx::query!
        if 'let' in line and '= sqlx::query!(' in line:
            # Check if it already has a type annotation
            if ': Vec<_>' not in line and ': Option<_>' not in line and ': sqlx::' not in line:
                # Find the pattern: let VARNAME = sqlx::query!(
                match = re.search(r'(\s+let\s+)(\w+)(\s*=\s*sqlx::query!\()', line)
                if match:
                    # Look ahead to see what method is called
                    lookahead = ''.join(lines[i:min(i+15, len(lines))])

                    if '.fetch_one(' in lookahead:
                        # fetch_one doesn't need type annotation
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

        # Fix closure type annotations
        if '.and_then(|' in line and 'match ' in line:
            # Pattern: .and_then(|x| match x.as_str()
            match = re.search(r'\.and_then\(\|(\w+)\|\s+match\s+\1\.as_str\(\)', line)
            if match:
                var_name = match.group(1)
                lines[i] = line.replace(f'|{var_name}|', f'|{var_name}: String|')
                modified = True

    if modified:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        return True
    return False

def main():
    sync_dir = Path('libraries/soul-sync/src')

    if not sync_dir.exists():
        print(f"Error: {sync_dir} not found")
        return

    rust_files = list(sync_dir.rglob('*.rs'))

    fixed = 0
    for f in rust_files:
        if fix_file(f):
            print(f"✓ {f}")
            fixed += 1

    print(f"\nFixed {fixed} files")

if __name__ == '__main__':
    main()
