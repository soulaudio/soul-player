# Async Device Monitoring Architecture

## Overview

Soul Player implements industry-standard async device monitoring using platform-native APIs for non-blocking device enumeration and real-time hotplug notifications.

## Architecture

```text
┌─────────────────────────────────────────────────────────┐
│         AsyncDeviceMonitor (Trait)                      │
├─────────────────────────────────────────────────────────┤
│  - async enumerate_devices()                            │
│  - async get_default_device()                           │
│  - async watch_for_changes(callback)                    │
│  - async is_device_available(device_id)                 │
│  - platform_name() -> &str                              │
└─────────────────────────────────────────────────────────┘
          │                 │                 │
     ┌────┴────┐       ┌────┴────┐       ┌────┴────┐
     │ macOS   │       │ Linux   │       │Windows  │
     │CoreAudio│       │PipeWire │       │  WinRT  │
     └─────────┘       └─────────┘       └─────────┘
          │                 │                 │
     ┌────┴───────────────  ┴─────────────────┴────┐
     │         CPAL Fallback (All Platforms)        │
     └──────────────────────────────────────────────┘
```

## Platform Implementations

### macOS: CoreAudio Native Async

**File**: `device_monitor_macos.rs`

**Technology**: CoreAudio Hardware Abstraction Layer (HAL)

**Performance**:
- Device enumeration: ~5-10ms (async, non-blocking)
- Hotplug: Real-time notifications via property listeners
- Zero polling overhead

**APIs Used**:
- `AudioObjectGetPropertyData` - Device enumeration
- `AudioObjectAddPropertyListener` - Hotplug notifications (Phase 3.2)
- `kAudioHardwarePropertyDevices` - Device list changes
- `kAudioHardwarePropertyDefaultOutputDevice` - Default device changes

**Current Status**: Phase 3.1-3.2 - Fast async enumeration + Real-time property listeners ✅ COMPLETE

**References**:
- [CoreAudio Documentation](https://developer.apple.com/documentation/coreaudio)
- Chrome WebRTC uses same approach
- VLC uses similar property listener pattern

---

### Linux: PipeWire Native Async

**File**: `device_monitor_linux.rs`

**Technology**: PipeWire Registry API

**Performance**:
- Device enumeration: ~10-20ms (async, non-blocking)
- Hotplug: Real-time notifications via registry events
- Zero polling overhead

**APIs Used**:
- `pw::registry::Registry` - Device enumeration
- Registry global events - Hotplug notifications (Phase 4.2)
- Node property events - Device property changes

**Current Status**: Phase 4.1 - Fast async enumeration ✅ | Phase 4.2 - Registry listeners ✅ COMPLETE

**System Requirements**:
```bash
# Ubuntu/Debian
sudo apt install libpipewire-0.3-dev

# Fedora
sudo dnf install pipewire-devel
```

**References**:
- [PipeWire Documentation](https://docs.pipewire.org/)
- Chrome uses PulseAudio/PipeWire
- Firefox uses PulseAudio via cubeb

---

### Windows: WinRT Native Async

**File**: `device_monitor_windows.rs`

**Technology**: Windows Runtime (WinRT) Device APIs

**Performance**:
- Device enumeration: ~10-30ms (async, non-blocking)
- Hotplug: Real-time notifications via DeviceWatcher
- Zero polling overhead

**APIs Used**:
- `MediaDevice::GetAudioRenderSelector()` - Device selector
- `DeviceInformation::FindAllAsync()` - Async enumeration
- `DeviceWatcher` - Hotplug notifications (Phase 5.2)

**Current Status**: Phase 5.1 - Fast async enumeration ✅ | Phase 5.2 - DeviceWatcher integration ✅ COMPLETE

**References**:
- [WinRT Device Enumeration](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration)
- Chrome uses WinRT device watchers
- Chromium source: Similar device monitoring approach

---

### CPAL Fallback (All Platforms)

**File**: `device_monitor_cpal_fallback.rs`

**Technology**: CPAL synchronous APIs wrapped in `tokio::task::spawn_blocking`

**Performance**:
- Device enumeration: ~50-500ms (async via spawn_blocking)
- Hotplug: Polling-based (2-second intervals)
- Works everywhere CPAL supports

**When Used**:
- When `native-device-monitor` feature is disabled (default)
- On unsupported platforms (FreeBSD, OpenBSD, etc.)
- As fallback if native implementation unavailable

**Trade-offs**:
- ✅ Works on all platforms
- ✅ Reliable (battle-tested CPAL)
- ✅ Non-blocking (via spawn_blocking)
- ❌ Slower than native (blocking syscalls)
- ❌ Polling overhead for hotplug

---

## Usage

### Basic Enumeration

```rust
use soul_audio_desktop::create_async_device_monitor;

#[tokio::main]
async fn main() {
    let monitor = create_async_device_monitor();

    // Enumerate devices (non-blocking)
    let devices = monitor.enumerate_devices().await.unwrap();
    for device in devices {
        println!("{}: {} ({}Hz, {}ch)",
            device.id,
            device.name,
            device.sample_rate.unwrap_or(0),
            device.channels.unwrap_or(0)
        );
    }

    // Get default device
    let default = monitor.get_default_device().await.unwrap();
    println!("Default: {}", default.name);

    // Check which implementation is active
    println!("Platform: {}", monitor.platform_name());
}
```

### Hotplug Notifications

```rust
use soul_audio_desktop::{create_async_device_monitor, DeviceEvent};

#[tokio::main]
async fn main() {
    let monitor = create_async_device_monitor();

    // Watch for device changes
    let handle = monitor.watch_for_changes(Box::new(|event| {
        match event {
            DeviceEvent::DeviceAdded { id, name } => {
                println!("Device added: {} ({})", name, id);
            }
            DeviceEvent::DeviceRemoved { id } => {
                println!("Device removed: {}", id);
            }
            DeviceEvent::DefaultDeviceChanged { id, name } => {
                println!("Default changed: {} ({})", name, id);
            }
            DeviceEvent::DevicePropertyChanged { id, property } => {
                println!("Property '{}' changed on {}", property, id);
            }
        }
    })).await.unwrap();

    // Keep watching...
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    // Stop watching
    drop(handle);
}
```

---

## Feature Flags

### Default Configuration (CPAL Fallback)

```toml
[dependencies]
soul-audio-desktop = "0.1.9"
```

**Uses**: CPAL fallback on all platforms
**Pros**: No system dependencies, works everywhere
**Cons**: Slower enumeration, polling-based hotplug

### Native Monitoring (Recommended for Production)

```toml
[dependencies]
soul-audio-desktop = { version = "0.1.9", features = ["native-device-monitor"] }
```

**Uses**:
- macOS → CoreAudio native async
- Linux → PipeWire native async
- Windows → WinRT native async
- Other → CPAL fallback

**Pros**: Fast enumeration, real-time hotplug, industry-standard
**Cons**: Requires system libraries on Linux

---

## System Requirements

### macOS
- ✅ CoreAudio framework (built-in, no installation needed)
- ✅ Works on macOS 10.13+ (High Sierra and later)

### Linux
- Requires PipeWire development headers:
  ```bash
  # Ubuntu/Debian
  sudo apt install libpipewire-0.3-dev

  # Fedora
  sudo dnf install pipewire-devel

  # Arch
  sudo pacman -S pipewire
  ```
- Falls back to CPAL if PipeWire unavailable

### Windows
- ✅ Windows SDK (built-in on Windows 10+)
- ✅ Works on Windows 10 version 1809+ (October 2018 Update)

---

## Performance Comparison

| Platform | Implementation | Enumeration | Hotplug | Polling | Status |
|----------|----------------|-------------|---------|---------|--------|
| macOS | CoreAudio Native | ~5-10ms | **Real-time (~1ms)** | **None** | **Phase 3.2 ✅** |
| macOS | CPAL Fallback | ~50-500ms | 2s poll | Yes | ✅ |
| Linux | PipeWire Native | ~10-20ms | **Real-time (~0ms)** | **None** | **Phase 4.2 ✅** |
| Linux | CPAL Fallback | ~50-500ms | 2s poll | Yes | ✅ |
| Windows | WinRT Native | ~10-30ms | **Real-time (~0ms)** | **None** | **Phase 5.2 ✅** |
| Windows | CPAL Fallback | ~50-500ms | 2s poll | Yes | ✅ |

---

## Industry Standards Comparison

### Chrome (Chromium)
- macOS: CoreAudio property listeners ✅ (Same approach)
- Linux: PulseAudio/PipeWire notifications ✅ (Similar)
- Windows: WinRT DeviceWatcher ✅ (Same approach)

### Firefox (cubeb)
- macOS: CoreAudio property listeners ✅ (Same approach)
- Linux: PulseAudio event subscriptions ✅ (Similar to PipeWire)
- Windows: WASAPI device notifications ✅ (Lower-level than WinRT)

### Spotify
- macOS: CoreAudio ✅
- Linux: PulseAudio/PipeWire ✅
- Windows: WASAPI ✅

**Conclusion**: Soul Player's approach matches industry standards for async device monitoring.

---

## Timeout Protection

All device operations are protected by timeout wrappers to prevent indefinite hangs when audio services freeze (see `device_check_timeout.rs`).

**Timeout Duration**: 5 seconds (configurable)

**Protection Points**:
- Device enumeration
- Default device queries
- Sample rate checks
- Availability checks

**Behavior on Timeout**:
- Returns error instead of hanging
- Emits timeout event to frontend
- Logs warning with device details
- Allows graceful degradation

---

## Implementation Status

### ✅ Phase 3.1-3.2: macOS CoreAudio Real-Time Hotplug (COMPLETE)
- Fast async device enumeration (~5-10ms)
- Non-blocking API calls
- Real-time property listeners via `AudioObjectAddPropertyListener`
- Zero polling overhead
- Automatic cleanup with `AudioObjectRemovePropertyListener`
- Comprehensive tracing logs

### ✅ Phase 4.2: Linux PipeWire Registry Listeners (COMPLETE)
- Real-time device add/remove notifications
- PipeWire registry event listeners implemented
- Event forwarding via tokio channels
- Zero polling overhead
- Comprehensive tracing logs

### ✅ Phase 5.2: Windows WinRT DeviceWatcher (COMPLETE)
- Real-time device add/remove notifications via DeviceWatcher
- Added/Removed/Updated event handlers
- Automatic default device change detection
- Zero polling overhead
- Comprehensive tracing logs

---

## Testing

### Unit Tests
```bash
# Test CPAL fallback (works everywhere)
cargo test --package soul-audio-desktop --lib device_monitor

# Test native implementations (requires system libs)
cargo test --package soul-audio-desktop --lib device_monitor --features native-device-monitor
```

### Integration Tests
```bash
# Test device enumeration
cargo test --package soul-audio-desktop enumerate_devices

# Test platform-specific implementations
cargo test --package soul-audio-desktop --features native-device-monitor
```

---

## Troubleshooting

### Linux: "Failed to create PipeWire mainloop"
**Cause**: PipeWire development headers not installed
**Fix**: `sudo apt install libpipewire-0.3-dev`

### macOS: "OSStatus -50" (Bad Parameter)
**Cause**: Invalid device ID or permission issue
**Fix**: Check System Preferences → Security & Privacy → Microphone

### Windows: "Failed to enumerate devices"
**Cause**: Windows SDK not available
**Fix**: Update Windows to version 1809+ (October 2018 Update)

### All Platforms: Timeout after 5 seconds
**Cause**: Audio service frozen or unresponsive
**Fix**: Restart audio service or reboot system

---

## References

- [CPAL Documentation](https://docs.rs/cpal/)
- [CoreAudio Documentation](https://developer.apple.com/documentation/coreaudio)
- [PipeWire Documentation](https://docs.pipewire.org/)
- [WinRT Device Enumeration](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration)
- [Chromium Audio Implementation](https://source.chromium.org/chromium/chromium/src/+/main:media/audio/)
- [Firefox cubeb Library](https://github.com/mozilla/cubeb)

---

**Last Updated**: 2026-01-25
**Implementation Status**:
- ✅ Phase 1: Timeout protection (COMPLETE)
- ✅ Phase 2: Async abstraction layer (COMPLETE)
- ✅ Phase 3.1-3.2: macOS CoreAudio real-time hotplug (COMPLETE)
- ✅ Phase 4.1-4.2: Linux PipeWire real-time hotplug (COMPLETE)
- ✅ Phase 5.1-5.2: Windows WinRT real-time hotplug (COMPLETE)
- ✅ Phase 6: Playback system integration (COMPLETE)
- ✅ Comprehensive tracing logs (COMPLETE)
- ✅ All tests passing (49 tests: 19 unit + 12 integration + 18 E2E)
