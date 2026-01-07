#!/bin/bash

echo "=== Environment Variable Check ==="
echo ""
echo "Checking for DATABASE_PATH..."
if [ -n "$DATABASE_PATH" ]; then
    echo "✗ DATABASE_PATH is set to: $DATABASE_PATH"
    echo "  This may override the default database location"
else
    echo "✓ DATABASE_PATH is not set"
fi

echo ""
echo "Checking .env files..."
find . -name ".env" -type f | while read -r envfile; do
    echo ""
    echo "File: $envfile"
    grep -v "^#" "$envfile" | grep -v "^$" | grep DATABASE_PATH || echo "  (no uncommented DATABASE_PATH)"
done

echo ""
echo "=== Done ==="
