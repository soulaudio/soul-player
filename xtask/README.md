# Soul Player xtask

Development task automation for Soul Player, with a focus on audio E2E testing.

## Installation

The xtask crate is part of the workspace and built automatically. To run commands:

```bash
# From workspace root
cargo xtask <command>

# Or with alias (add to your shell profile)
alias xt="cargo xtask"
```

## Commands

### Audio E2E Testing

#### Run Complete Test Suite

```bash
# Full E2E test suite with device check
cargo xtask test audio e2e

# CI mode (skip device check, shorter timeouts)
cargo xtask test audio e2e --ci

# Run only initialization tests
cargo xtask test audio e2e --init-only

# Run only stutter detection tests
cargo xtask test audio e2e --stutter-only

# Export metrics to JSON
cargo xtask test audio e2e --export-metrics metrics.json

# Skip virtual device check (for debugging)
cargo xtask test audio e2e --skip-device-check
```

#### List Audio Devices

```bash
# List all available audio devices
cargo xtask test audio list-devices

# Verbose output with capabilities
cargo xtask test audio list-devices --verbose

# Filter by device name
cargo xtask test audio list-devices --filter BlackHole
```

#### Generate Test Assets

```bash
# Generate all test audio files
cargo xtask test audio generate-assets

# Custom output directory
cargo xtask test audio generate-assets --output custom/path

# Overwrite existing files
cargo xtask test audio generate-assets --force
```

#### CI Tests

```bash
# Run with custom timeout (default: 300s)
cargo xtask test audio ci --timeout 600

# Export CI metrics
cargo xtask test audio ci --export-metrics ci-metrics.json
```

### Import Testing

```bash
# Run import/re-import E2E tests
cargo xtask test import e2e

# CI mode
cargo xtask test import e2e --ci

# Filter by category
cargo xtask test import e2e --filter reimport

# Custom thread count
cargo xtask test import e2e --threads 4

# Run unit tests
cargo xtask test import unit
```

### Cache Testing

```bash
# Run cache invalidation E2E tests
cargo xtask test cache e2e

# Test specific cache type
cargo xtask test cache e2e --cache-type artwork

# Run integration tests
cargo xtask test cache integration
```

## Test Assets

The `test audio generate-assets` command creates standardized test files:

| File | Description | Use Case |
|------|-------------|----------|
| `1khz-sine-10s.wav` | 1kHz sine wave, 10s @ 44.1kHz | Initialization tests |
| `1khz-sine-30s.wav` | 1kHz sine wave, 30s @ 44.1kHz | Long-running tests |
| `440hz-sine-5s.wav` | 440Hz (A4), 5s @ 48kHz | Pitch verification |
| `silence-1s.wav` | 1s silence @ 44.1kHz | Gap detection |
| `silence-500ms.wav` | 500ms silence @ 44.1kHz | Short gap detection |
| `distinctive-intro.wav` | Unique pattern for stutter detection | False start detection |
| `sweep-1s.wav` | 100Hz-10kHz sweep @ 48kHz | Quality testing |

## Virtual Audio Device Setup

E2E tests require a virtual audio device. Install one for your platform:

### macOS

```bash
brew install blackhole-2ch
```

Or use [Loopback](https://rogueamoeba.com/loopback/) (commercial).

### Linux

```bash
# ALSA loopback module
sudo modprobe snd-aloop

# Or PulseAudio null sink
pactl load-module module-null-sink
```

### Windows

Install [VB-Cable](https://vb-audio.com/Cable/) (free) or [Virtual Audio Cable](https://vac.muzychenko.net/) (commercial).

```powershell
# With Chocolatey
choco install vb-cable
```

## Metrics Export

When using `--export-metrics`, a JSON file is created with test execution data:

```json
{
  "total_duration_secs": 45.23,
  "tests_run": 2,
  "tests_passed": 2,
  "tests_failed": 0,
  "virtual_device_available": true,
  "test_assets_present": true,
  "timestamp": "2026-02-11T10:30:00Z"
}
```

## Development

### Adding New Commands

1. Add command enum to `src/main.rs`
2. Create module in `src/` (e.g., `src/mynew.rs`)
3. Implement command logic
4. Add documentation to this README

### Project Structure

```
xtask/
├── Cargo.toml          # Dependencies
├── README.md           # This file
└── src/
    ├── main.rs         # CLI definition
    ├── audio.rs        # Audio E2E orchestration
    ├── devices.rs      # Device enumeration
    ├── generator.rs    # Test asset generation
    ├── import.rs       # Import test automation
    └── cache.rs        # Cache test automation
```

## Troubleshooting

### "No virtual audio device found"

Install a virtual audio device (see setup above), or use `--skip-device-check` for testing without audio hardware.

### "Test assets not found"

Run `cargo xtask test audio generate-assets` to create required test files.

### Tests timing out in CI

Increase timeout: `cargo xtask test audio ci --timeout 600`

### Device enumeration fails

Check audio system is initialized:
```bash
cargo test --package soul-audio-desktop test_real_enumerate_devices -- --nocapture
```

## Related Documentation

- [Audio E2E Testing Quickstart](../AUDIO_E2E_TESTING_QUICKSTART.md)
- [Audio E2E Testing Strategy](../docs/AUDIO_E2E_TESTING_STRATEGY.md)
- [E2E Test Suite](../tests/e2e/README.md)
