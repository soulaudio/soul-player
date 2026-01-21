#!/bin/bash

# Script to replace console.log/warn/error with debug.log/warn/error
# across all TypeScript files in applications/shared/src

set -e

SHARED_SRC="applications/shared/src"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Starting console.log replacement in ${SHARED_SRC}...${NC}"

# Find all .ts and .tsx files, excluding test files and node_modules
FILES=$(find "$SHARED_SRC" -type f \( -name "*.ts" -o -name "*.tsx" \) \
  ! -name "*.test.ts" \
  ! -name "*.test.tsx" \
  ! -path "*/node_modules/*" \
  ! -path "*/__tests__/*")

# Count files to process
TOTAL_FILES=$(echo "$FILES" | wc -l)
echo -e "${GREEN}Found $TOTAL_FILES files to process${NC}"

# Counter for files modified
MODIFIED=0

# Process each file
for file in $FILES; do
  # Check if file contains console.log, console.warn, or console.error
  if grep -q "console\.\(log\|warn\|error\)" "$file"; then
    echo "Processing: $file"

    # Check if file already imports debug utility
    if ! grep -q "import.*debug.*from.*utils/debug" "$file"; then
      # Determine import path depth
      DEPTH=$(echo "$file" | sed "s|$SHARED_SRC/||" | tr -cd '/' | wc -c)

      # Build relative path to utils/debug
      if [ "$DEPTH" -eq 0 ]; then
        IMPORT_PATH="./utils/debug"
      elif [ "$DEPTH" -eq 1 ]; then
        IMPORT_PATH="../utils/debug"
      elif [ "$DEPTH" -eq 2 ]; then
        IMPORT_PATH="../../utils/debug"
      elif [ "$DEPTH" -eq 3 ]; then
        IMPORT_PATH="../../../utils/debug"
      else
        IMPORT_PATH="../../../../utils/debug"
      fi

      # Add import statement after last import line
      # Find the last line that starts with 'import'
      LAST_IMPORT_LINE=$(grep -n "^import" "$file" | tail -1 | cut -d: -f1)

      if [ -n "$LAST_IMPORT_LINE" ]; then
        # Insert debug import after last import
        sed -i "${LAST_IMPORT_LINE}a\\import { debug } from '${IMPORT_PATH}';" "$file"
        echo "  ✓ Added debug import"
      fi
    fi

    # Replace console.log -> debug.log
    sed -i 's/console\.log(/debug.log(/g' "$file"

    # Replace console.warn -> debug.warn
    sed -i 's/console\.warn(/debug.warn(/g' "$file"

    # Keep console.error as debug.error (errors should still log in production)
    sed -i 's/console\.error(/debug.error(/g' "$file"

    ((MODIFIED++))
    echo "  ✓ Replaced console statements"
  fi
done

echo -e "${GREEN}✓ Complete! Modified $MODIFIED files${NC}"
