# xtask Usage Examples

Quick examples for common xtask workflows.

## Quick Start

```bash
# Build xtask
cargo build --package xtask

# Run with cargo run
cargo run --package xtask -- <command>
```

## Audio Testing Workflows

### 1. First-Time Setup

```bash
# Generate test audio assets
cargo run --package xtask -- test audio generate-assets

# Check what audio devices are available
cargo run --package xtask -- test audio list-devices --verbose
```

### 2. Run Complete E2E Test Suite

```bash
# Full test suite (requires virtual audio device)
cargo run --package xtask -- test audio e2e

# Skip device check for debugging
cargo run --package xtask -- test audio e2e --skip-device-check

# Export metrics for analysis
cargo run --package xtask -- test audio e2e --export-metrics metrics.json
```

### 3. Run Specific Tests

```bash
# Only initialization tests
cargo run --package xtask -- test audio e2e --init-only

# Only stutter detection tests
cargo run --package xtask -- test audio e2e --stutter-only
```

### 4. CI/CD Integration

```bash
# CI mode with timeout
cargo run --package xtask -- test audio ci --timeout 300

# Export CI metrics
cargo run --package xtask -- test audio ci --timeout 300 --export-metrics ci-metrics.json
```

### 5. Device Management

```bash
# List all devices
cargo run --package xtask -- test audio list-devices

# Show detailed capabilities
cargo run --package xtask -- test audio list-devices --verbose

# Filter by name (e.g., find BlackHole)
cargo run --package xtask -- test audio list-devices --filter BlackHole
```

### 6. Regenerate Test Assets

```bash
# Generate in default location
cargo run --package xtask -- test audio generate-assets

# Custom output directory
cargo run --package xtask -- test audio generate-assets --output custom/path

# Force overwrite existing files
cargo run --package xtask -- test audio generate-assets --force
```

## Import Testing

```bash
# Run import E2E tests
cargo run --package xtask -- test import e2e

# Run with specific filter
cargo run --package xtask -- test import e2e --filter reimport

# Run unit tests
cargo run --package xtask -- test import unit
```

## Cache Testing

```bash
# Run cache E2E tests
cargo run --package xtask -- test cache e2e

# Test specific cache type
cargo run --package xtask -- test cache e2e --cache-type artwork

# Run integration tests
cargo run --package xtask -- test cache integration
```

## Shell Aliases (Optional)

Add to your `.bashrc` or `.zshrc`:

```bash
# Short alias for xtask
alias xt="cargo run --package xtask --"

# Audio-specific aliases
alias xt-audio="cargo run --package xtask -- test audio"
alias xt-audio-e2e="cargo run --package xtask -- test audio e2e"
alias xt-audio-gen="cargo run --package xtask -- test audio generate-assets"
```

Then use:

```bash
xt test audio e2e
xt-audio list-devices --verbose
xt-audio-gen --force
```

## PowerShell Aliases (Windows)

Add to your PowerShell profile:

```powershell
function xt { cargo run --package xtask -- $args }
function xt-audio { cargo run --package xtask -- test audio $args }
function xt-audio-e2e { cargo run --package xtask -- test audio e2e $args }
```

## GitHub Actions Integration

Example workflow:

```yaml
name: Audio E2E Tests

on:
  pull_request:
    branches: [main]

jobs:
  audio-e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 10

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install virtual audio device
        run: sudo modprobe snd-aloop

      - name: Generate test assets
        run: cargo run --package xtask -- test audio generate-assets

      - name: Run audio E2E tests (CI mode)
        run: cargo run --package xtask -- test audio ci --timeout 300 --export-metrics metrics.json

      - name: Upload metrics
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: audio-e2e-metrics
          path: metrics.json
```

## Local Development Workflow

Typical workflow when developing audio features:

```bash
# 1. Generate test assets (first time only)
cargo run --package xtask -- test audio generate-assets

# 2. Check available devices
cargo run --package xtask -- test audio list-devices

# 3. Run tests during development
cargo run --package xtask -- test audio e2e --init-only

# 4. Full test before commit
cargo run --package xtask -- test audio e2e

# 5. Export metrics for analysis
cargo run --package xtask -- test audio e2e --export-metrics latest-metrics.json
```

## Troubleshooting

### "No virtual audio device found"

```bash
# macOS
brew install blackhole-2ch

# Linux
sudo modprobe snd-aloop

# Windows (as Admin)
choco install vb-cable
```

### Test assets missing

```bash
cargo run --package xtask -- test audio generate-assets --force
```

### Device enumeration fails

```bash
# Check audio system directly
cargo test --package soul-audio-desktop test_real_enumerate_devices -- --nocapture
```

### Tests timing out

```bash
# Increase timeout
cargo run --package xtask -- test audio ci --timeout 600
```

## Metrics Analysis

After running tests with `--export-metrics`, analyze the JSON:

```bash
# View metrics
cat metrics.json | jq .

# Extract specific values
cat metrics.json | jq '.total_duration_secs'
cat metrics.json | jq '.tests_passed / .tests_run * 100'  # Pass rate

# Compare multiple runs
jq -s '[.[] | {timestamp, duration: .total_duration_secs, passed: .tests_passed}]' \
  metrics-*.json
```
