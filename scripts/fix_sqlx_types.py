#!/usr/bin/env python3
"""
Fix SQLx type annotation issues in soul-storage library.
This script adds explicit type annotations where the Rust compiler needs them.
"""

import re
import sys
from pathlib import Path

def fix_fetch_all_pattern(content):
    """Fix patterns like: let rows = sqlx::query!(...).fetch_all(pool)"""
    # Pattern: let VARNAME = sqlx::query!
    pattern = r'(\s+let\s+)(\w+)(\s*=\s*sqlx::query!\()'

    def replacer(match):
        indent = match.group(1)
        var_name = match.group(2)
        rest = match.group(3)

        # Check if already has type annotation
        if ': Vec<_>' in match.group(0) or ': Option<_>' in match.group(0) or ': sqlx::' in match.group(0):
            return match.group(0)

        # Add Vec<_> type annotation (will be refined later if needed)
        return f"{indent}{var_name}: Vec<_>{rest}"

    return re.sub(pattern, replacer, content)

def fix_fetch_optional_pattern(content):
    """Fix patterns that use fetch_optional to use Option<_> instead of Vec<_>"""
    # Find lines with Vec<_> that have fetch_optional
    lines = content.split('\n')
    result = []

    for i, line in enumerate(lines):
        # If this line has Vec<_> and in the next few lines we see fetch_optional
        if ': Vec<_>' in line:
            # Look ahead to find if this uses fetch_optional
            lookahead = '\n'.join(lines[i:min(i+10, len(lines))])
            if '.fetch_optional' in lookahead:
                line = line.replace(': Vec<_>', ': Option<_>')
        result.append(line)

    return '\n'.join(result)

def fix_execute_pattern(content):
    """Fix patterns that use execute to use SqliteQueryResult"""
    lines = content.split('\n')
    result = []

    for i, line in enumerate(lines):
        # If this line has Vec<_> and in the next few lines we see execute
        if ': Vec<_>' in line:
            # Look ahead to find if this uses execute
            lookahead = '\n'.join(lines[i:min(i+10, len(lines))])
            if '.execute' in lookahead and '.fetch' not in lookahead:
                line = line.replace(': Vec<_>', ': sqlx::sqlite::SqliteQueryResult')
        result.append(line)

    return '\n'.join(result)

def fix_file(file_path):
    """Fix a single file"""
    print(f"Processing {file_path}...")

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # Apply fixes in order
    content = fix_fetch_all_pattern(content)
    content = fix_fetch_optional_pattern(content)
    content = fix_execute_pattern(content)

    # Only write if content changed
    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  ✓ Fixed {file_path}")
        return True
    else:
        print(f"  - No changes needed for {file_path}")
        return False

def main():
    storage_dir = Path('libraries/soul-storage/src')

    if not storage_dir.exists():
        print(f"Error: {storage_dir} not found")
        sys.exit(1)

    print("Fixing SQLx type annotation issues...")
    print()

    # Get all Rust files
    rust_files = list(storage_dir.rglob('*.rs'))

    fixed_count = 0
    for rust_file in rust_files:
        if fix_file(rust_file):
            fixed_count += 1

    print()
    print(f"Fixed {fixed_count} files out of {len(rust_files)} total files")
    print()
    print("Testing compilation...")

if __name__ == '__main__':
    main()
