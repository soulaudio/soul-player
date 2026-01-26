#!/usr/bin/env python3
"""
Fix SQLx type annotation issues - Version 2
Better detection of query patterns by looking at the actual method called.
"""

import re
from pathlib import Path

def fix_file_advanced(file_path):
    """Fix a file by analyzing the actual query patterns"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # Find all let statements with sqlx::query!
    # Pattern: capture from 'let varname = sqlx::query!' through the next .await?;
    pattern = r'(let\s+(\w+))\s*:\s*Vec<_>\s*(=\s*sqlx::query!\(.*?\))(.*?)\.await\?;'

    def fix_query(match):
        let_part = match.group(1)  # "let varname"
        var_name = match.group(2)   # varname
        query_part = match.group(3) # = sqlx::query!(...)
        rest = match.group(4)        # everything between ) and .await?;

        # Check what method is being called
        if '.fetch_one(' in rest:
            # fetch_one returns a single record (not wrapped in Option)
            return f"{let_part}: Option<_> {query_part}{rest}.await?;"
        elif '.fetch_optional(' in rest:
            # fetch_optional returns Option<Record>
            return f"{let_part}: Option<_> {query_part}{rest}.await?;"
        elif '.execute(' in rest:
            # execute returns SqliteQueryResult
            return f"{let_part}: sqlx::sqlite::SqliteQueryResult {query_part}{rest}.await?;"
        elif '.fetch_all(' in rest:
            # fetch_all returns Vec<Record> - keep as is
            return match.group(0)
        else:
            # Unknown pattern, keep as is
            return match.group(0)

    content = re.sub(pattern, fix_query, content, flags=re.DOTALL)

    if content != original:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    return False

def main():
    storage_dir = Path('libraries/soul-storage/src')
    rust_files = list(storage_dir.rglob('*.rs'))

    fixed = 0
    for f in rust_files:
        if fix_file_advanced(f):
            print(f"✓ Fixed {f}")
            fixed += 1

    print(f"\nFixed {fixed} files")

if __name__ == '__main__':
    main()
