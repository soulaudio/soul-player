#!/bin/bash
# Script to fix SQLx type annotation issues in soul-storage

set -e

STORAGE_DIR="libraries/soul-storage/src"

echo "Fixing SQLx type annotation issues in soul-storage..."

# Function to add type annotation to a line
fix_file() {
    local file=$1
    echo "Processing $file..."

    # Fix patterns like: let rows = sqlx::query!(...).fetch_all(pool)
    perl -i -0pe 's/(\s+)let (rows|row|result|album|artist|genre|track|playlist|source|device|setting|shortcut|context|state|queue_item|stats)(\s*=\s*sqlx::query!\()/\1let \2: Vec<_>\3/g' "$file"

    # Fix patterns like: let row = sqlx::query!(...).fetch_optional(pool)
    perl -i -0pe 's/(\s+)let (row|item|record|entry)(\s*:\s*Vec<_>\s*=\s*sqlx::query!\(.*?\)\s*\.\s*fetch_optional)/\1let \2: Option<_> =sqlx::query!(\3/gs' "$file"

    # Fix patterns like: let result = sqlx::query!(...).execute(pool)
    perl -i -0pe 's/(\s+)let result(\s*=\s*sqlx::query!\(.*?\)\s*\.\s*execute)/\1let result: sqlx::sqlite::SqliteQueryResult\2/gs' "$file"
}

# Process all Rust files in soul-storage
find "$STORAGE_DIR" -name "*.rs" -type f | while read file; do
    fix_file "$file"
done

echo "Done! Now trying to compile..."
cargo build --package soul-storage 2>&1 | tail -20
