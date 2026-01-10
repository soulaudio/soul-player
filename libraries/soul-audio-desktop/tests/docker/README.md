# Docker E2E Tests for Audio Device Switching

This directory contains Docker-based end-to-end tests for the soul-audio-desktop library's device switching functionality.

## Overview

The tests use **testcontainers-rs** to create isolated Linux containers with virtual audio devices (PulseAudio null sinks). This allows comprehensive testing of audio device enumeration and switching without requiring physical audio hardware.

## Test Architecture

### Docker Image: `soul-audio-test`

The test image (`Dockerfile.audio-test`) provides:
- **Base**: `rust:1.75-slim-bookworm` (Debian 12)
- **Audio System**: PulseAudio running in user mode (to avoid permission issues)
- **Virtual Devices**: 3 null sinks (virtual_output_1, virtual_output_2, virtual_output_3)
- **Utilities**: procps, alsa-utils, pulseaudio-utils for diagnostics

### Test Structure

Tests are located in `/tests/e2e_device_switching_docker.rs` and include:

1. **Container Health Check** (`test_docker_audio_container_health`)
   - Verifies PulseAudio starts correctly
   - Confirms all 3 virtual devices are present
   - Validates container environment

2. **Device Enumeration** (`test_device_enumeration_in_container`)
   - Tests device discovery in containerized environment
   - Validates virtual device metadata

3. **Host Integration Tests** (run outside container)
   - Backend enumeration
   - Device listing
   - Playback creation
   - Stress testing

## Running Tests

### Prerequisites

- **Docker Desktop** must be installed and running
- **Rust** toolchain with cargo
- **Linux container support** (WSL2 on Windows)

### Execute Tests

```bash
# Run all Docker E2E tests
cargo test --features docker-tests --test e2e_device_switching_docker

# Run specific test
cargo test --features docker-tests --test e2e_device_switching_docker test_docker_audio_container_health -- --nocapture

# Run with detailed output
RUST_LOG=debug cargo test --features docker-tests --test e2e_device_switching_docker -- --nocapture
```

### What Happens During Test Execution

1. **Image Build**: The Dockerfile is automatically built as `soul-audio-test:latest`
2. **Container Start**: testcontainers spins up a container from the image
3. **PulseAudio Init**: Container startup script launches PulseAudio with virtual devices
4. **Test Execution**: Tests execute commands inside the container via `docker exec`
5. **Cleanup**: Container is automatically removed when tests complete

## Technical Details

### PulseAudio Configuration

The container runs PulseAudio in **user mode** (not system mode):
- Avoids D-Bus/authentication complexity in containers
- Runs as root user with XDG_RUNTIME_DIR=/run/user/0
- Creates 3 null sinks on startup via default.pa configuration

### Virtual Audio Devices

Each virtual device is a PulseAudio null sink:
```
Sink #0: virtual_output_1 (module-null-sink.c, s16le 2ch 44100Hz)
Sink #1: virtual_output_2 (module-null-sink.c, s16le 2ch 44100Hz)
Sink #2: virtual_output_3 (module-null-sink.c, s16le 2ch 44100Hz)
```

These behave like real audio output devices for testing purposes, discarding audio data.

### Integration with testcontainers-rs

The tests use testcontainers 0.23 API:
```rust
use testcontainers::{core::WaitFor, runners::AsyncRunner, Image};

let container = AudioTestImage.start().await?;
// Container auto-cleans up when dropped
```

## Limitations

1. **Platform**: Tests only run on Linux containers (no Windows container support for audio)
2. **CI/CD**: Requires Docker-in-Docker or Docker socket mounting in CI environments
3. **Performance**: Building Docker images adds ~1-2 minutes to test execution
4. **Scope**: Tests device switching logic, not actual audio playback

## Troubleshooting

### "Docker not available"
- Ensure Docker Desktop is running
- Check `docker --version` works from command line

### "PulseAudio is not running"
- Container might not have procps installed
- Check container logs: `docker logs <container_id>`

### "Access denied" errors
- This occurred when using `--system` flag for PulseAudio
- Fixed by running in user mode (current configuration)

### Tests timeout
- Increase wait duration in `ready_conditions()` (currently 3 seconds)
- Check Docker Desktop resource limits

## Future Improvements

- [ ] Add Windows container tests (if audio simulation becomes feasible)
- [ ] Test ASIO backend simulation (Windows)
- [ ] Test JACK backend with virtual JACK server
- [ ] Measure actual device switching latency
- [ ] Add tests for buffer underruns during device switch

## References

- [testcontainers-rs documentation](https://rust.testcontainers.org/)
- [PulseAudio null-sink module](https://www.freedesktop.org/wiki/Software/PulseAudio/Documentation/User/Modules/#module-null-sink)
- [CPAL documentation](https://docs.rs/cpal)
