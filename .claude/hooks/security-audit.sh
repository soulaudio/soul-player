#!/bin/bash
# Claude hook: Run security audit after cargo add/install commands
# This hook runs after Bash tool when modifying dependencies

set -e

# Read JSON input from stdin
INPUT=$(cat)

# Get the command from hook input
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Only run audit for cargo dependency management commands
if [[ ! "$COMMAND" =~ (cargo[[:space:]]+add|cargo[[:space:]]+install) ]]; then
  exit 0
fi

# Navigate to project directory
cd "$CLAUDE_PROJECT_DIR"

echo "::group::Running security audit after dependency change"

# Run cargo audit with project's ignore list (from CI workflow)
cargo audit --deny warnings \
  --ignore RUSTSEC-2023-0071 \
  --ignore RUSTSEC-2024-0370 \
  --ignore RUSTSEC-2024-0411 \
  --ignore RUSTSEC-2024-0412 \
  --ignore RUSTSEC-2024-0413 \
  --ignore RUSTSEC-2024-0414 \
  --ignore RUSTSEC-2024-0415 \
  --ignore RUSTSEC-2024-0416 \
  --ignore RUSTSEC-2024-0417 \
  --ignore RUSTSEC-2024-0418 \
  --ignore RUSTSEC-2024-0419 \
  --ignore RUSTSEC-2024-0420 \
  --ignore RUSTSEC-2024-0421 \
  --ignore RUSTSEC-2024-0422 \
  --ignore RUSTSEC-2024-0423 \
  --ignore RUSTSEC-2024-0424 \
  --ignore RUSTSEC-2024-0425 \
  --ignore RUSTSEC-2024-0426 \
  --ignore RUSTSEC-2024-0427 \
  --ignore RUSTSEC-2024-0428 \
  --ignore RUSTSEC-2024-0429 \
  --ignore RUSTSEC-2024-0436 \
  --ignore RUSTSEC-2024-0437 \
  --ignore RUSTSEC-2024-0438 \
  --ignore RUSTSEC-2025-0057 \
  --ignore RUSTSEC-2025-0075 \
  --ignore RUSTSEC-2025-0080 \
  --ignore RUSTSEC-2025-0081 \
  --ignore RUSTSEC-2025-0098 \
  --ignore RUSTSEC-2025-0100 \
  --ignore RUSTSEC-2025-0119 \
  --ignore RUSTSEC-2025-0134 \
  --ignore RUSTSEC-2024-0384 \
  --ignore RUSTSEC-2025-0111 \
  2>&1 || {
    echo "::error::Security audit found new vulnerabilities!"
    echo "Please review the output above and either fix the vulnerability or add to ignore list in CI workflow"
    exit 2
  }

echo "::endgroup::"
echo "Security audit passed"
exit 0
