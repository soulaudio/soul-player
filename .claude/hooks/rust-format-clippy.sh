#!/bin/bash
# Claude hook: Run format and clippy after Rust file modifications
# This hook runs after Edit/Write tools when modifying .rs files

set -e

# Read JSON input from stdin
INPUT=$(cat)

# Get the file path from hook input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# If no file_path in tool input, try getting from tool_response
if [ -z "$FILE_PATH" ]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_response.filePath // empty')
fi

# Exit if still no file path
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# Only process Rust files
if [[ ! "$FILE_PATH" =~ \.rs$ ]]; then
  exit 0
fi

# Navigate to project directory
cd "$CLAUDE_PROJECT_DIR"

# Run format (fix mode)
echo "::group::Running cargo fmt"
cargo fmt --all 2>&1 || true
echo "::endgroup::"

# Run clippy on libraries and bins only (faster)
echo "::group::Running cargo clippy"
cargo clippy --lib --bins --all-features -- -D warnings 2>&1 || {
  echo "::error::Clippy found issues"
  exit 2
}
echo "::endgroup::"

exit 0
