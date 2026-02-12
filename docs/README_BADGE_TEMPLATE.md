# README Badge Template

Add these badges to your main README.md to display Audio E2E test status.

## Audio E2E Tests Badge

**Markdown:**
```markdown
[![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml)
```

**HTML:**
```html
<a href="https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml">
  <img src="https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg" alt="Audio E2E Tests">
</a>
```

**Badge with Branch:**
```markdown
[![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg?branch=main)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml)
```

## Platform-Specific Badges

If you want individual platform status:

**Linux:**
```markdown
![Linux](https://img.shields.io/badge/Linux-E2E%20Passing-success?logo=linux)
```

**macOS:**
```markdown
![macOS](https://img.shields.io/badge/macOS-E2E%20Passing-success?logo=apple)
```

**Windows:**
```markdown
![Windows](https://img.shields.io/badge/Windows-E2E%20Passing-success?logo=windows)
```

## Recommended README Section

Add this section to your README.md:

```markdown
## Testing

Soul Player has comprehensive test coverage including:

- **Unit Tests:** Core library functionality
- **Integration Tests:** Cross-component interactions
- **Audio E2E Tests:** Real-world device handling and playback scenarios

[![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml)

### Audio E2E Testing

Our audio E2E tests verify:
- Device hotplug detection and switching
- Playback resilience during device changes
- Platform-specific audio API behavior (ALSA, CoreAudio, WASAPI)
- Virtual device configuration and performance

Tests run automatically on all platforms using virtual audio devices:
- **Linux:** ALSA snd-aloop
- **macOS:** BlackHole 2ch
- **Windows:** VB-Cable

**Quick Start:**
```bash
# Setup virtual audio (one-time)
./scripts/setup-virtual-audio.sh  # Linux/macOS
.\scripts\setup-virtual-audio.ps1 # Windows

# Run tests
cargo test --release -p soul-audio-desktop --test device_hotplug_e2e
```

For detailed documentation, see [Audio E2E Testing Guide](docs/AUDIO_E2E_TESTING.md).
```

## Badge Customization

### Custom Colors

```markdown
![Audio E2E](https://img.shields.io/badge/Audio%20E2E-Passing-brightgreen?style=flat-square)
![Audio E2E](https://img.shields.io/badge/Audio%20E2E-Passing-brightgreen?style=for-the-badge)
```

### With Event Info

```markdown
[![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg?event=push)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml)
```

### With Specific Branch

```markdown
[![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg?branch=develop)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml?query=branch%3Adevelop)
```

## Complete Badge Set Example

For a comprehensive status display:

```markdown
## Build Status

| Category | Status |
|----------|--------|
| CI Pipeline | [![CI](https://github.com/soulaudio/soul-player/actions/workflows/ci.yml/badge.svg)](https://github.com/soulaudio/soul-player/actions/workflows/ci.yml) |
| Audio E2E Tests | [![Audio E2E Tests](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml/badge.svg)](https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml) |
| Security Audit | [![Security](https://github.com/soulaudio/soul-player/actions/workflows/security.yml/badge.svg)](https://github.com/soulaudio/soul-player/actions/workflows/security.yml) |
| Code Coverage | [![codecov](https://codecov.io/gh/soulaudio/soul-player/branch/main/graph/badge.svg)](https://codecov.io/gh/soulaudio/soul-player) |
```

---

**Note:** Replace `soulaudio/soul-player` with your actual GitHub repository path.
