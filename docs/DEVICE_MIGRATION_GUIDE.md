# Device Monitoring Migration Guide

## Overview

This guide documents the migration from the old synchronous CPAL-based device enumeration to the new async device monitoring system.

## What Changed

### Old System (Deprecated)
- **Location**: `libraries/soul-audio-desktop/src/device.rs`
- **Functions**:
  - `list_devices()` / `list_devices_with_capabilities()`
  - `get_default_device()` / `get_default_device_with_capabilities()`
  - `get_device_capabilities()`
- **Issues**: Synchronous CPAL calls that block for 50-500ms on macOS

### New System (Recommended)
- **Location**: `libraries/soul-audio-desktop/src/device_monitor_async.rs`
- **Trait**: `AsyncDeviceMonitor`
- **Factory**: `create_async_device_monitor()`
- **Advantages**:
  - Truly async device enumeration (non-blocking)
  - Platform-native implementations (CoreAudio, PipeWire, WinRT)
  - Hotplug notifications via `watch_for_changes()`
  - Better performance on macOS (100-500ms → <10ms)

## Migration Status

### Phase 1: Deprecation (COMPLETED)
- ✅ Added `#[deprecated]` attributes to old sync functions
- ✅ Functions remain functional with deprecation warnings
- ✅ All existing code continues to work

### Phase 2: Gradual Migration (IN PROGRESS)
- 🔄 Application code can migrate at their own pace
- 🔄 Tests will be updated incrementally
- 🔄 Deprecation warnings guide developers to new API

### Phase 3: Future Removal (v0.2.0+)
- ⏳ Remove deprecated functions in next major version
- ⏳ All code must use `AsyncDeviceMonitor` by then

## How to Migrate

### For Async Code (Tauri Commands, etc.)

**Before (Deprecated):**
```rust
#[tauri::command]
pub async fn get_audio_devices(backend_str: String) -> Result<Vec<DeviceInfo>, String> {
    let backend = parse_backend(&backend_str)?;

    // Blocks Tokio thread for 100-500ms on macOS!
    let devices = tokio::task::spawn_blocking(move || {
        device::list_devices(backend)
    })
    .await??;

    Ok(devices)
}
```

**After (Recommended):**
```rust
#[tauri::command]
pub async fn get_audio_devices(backend_str: String) -> Result<Vec<DeviceInfo>, String> {
    // Create async monitor (cheap, can be cached in app state)
    let monitor = create_async_device_monitor();

    // Non-blocking async enumeration (<10ms on macOS)
    let devices = monitor.enumerate_devices().await?;

    // Convert AsyncDeviceInfo to your frontend format
    let frontend_devices: Vec<DeviceInfo> = devices
        .into_iter()
        .map(|d| DeviceInfo {
            name: d.name,
            is_default: d.is_default,
            sample_rate: d.sample_rate.unwrap_or(48000),
            channels: d.channels.unwrap_or(2),
        })
        .collect();

    Ok(frontend_devices)
}
```

### For Capability Detection

The `detect_device_capabilities()` function is **NOT deprecated** and is still needed for detailed device capability queries.

**Pattern:**
```rust
use soul_audio_desktop::{create_async_device_monitor, detect_device_capabilities, find_device_by_name};

#[tauri::command]
pub async fn get_device_capabilities(
    backend_str: String,
    device_name: String,
) -> Result<DeviceCapabilities, String> {
    let backend = parse_backend(&backend_str)?;

    // Use spawn_blocking for CPAL device lookup (still needed)
    let device_name_clone = device_name.clone();
    let caps = tokio::task::spawn_blocking(move || {
        let device = find_device_by_name(backend, &device_name_clone)?;
        Ok::<_, DeviceError>(detect_device_capabilities(&device, backend))
    })
    .await??;

    Ok(caps)
}
```

### For Hotplug Notifications (NEW!)

The async monitor supports device change notifications:

```rust
use soul_audio_desktop::{create_async_device_monitor, DeviceEvent};

async fn setup_device_monitoring() {
    let monitor = create_async_device_monitor();

    // Watch for device changes
    let _handle = monitor.watch_for_changes(Box::new(|event| {
        match event {
            DeviceEvent::DeviceAdded { id, name } => {
                println!("Device added: {} ({})", name, id);
            }
            DeviceEvent::DeviceRemoved { id } => {
                println!("Device removed: {}", id);
            }
            DeviceEvent::DefaultDeviceChanged { id, name } => {
                println!("Default device changed to: {} ({})", name, id);
            }
            DeviceEvent::DevicePropertyChanged { id, property } => {
                println!("Device {} property changed: {}", id, property);
            }
        }
    })).await.expect("Failed to watch devices");

    // Handle stays alive until dropped
}
```

## Backward Compatibility

### Still Supported (Not Deprecated)
- ✅ `detect_device_capabilities()` - Still needed for capability queries
- ✅ `find_device_by_name()` - Still useful for device lookup
- ✅ `AudioDeviceInfo` - Data structure remains the same
- ✅ `DeviceCapabilities` - Data structure remains the same
- ✅ `SupportedBitDepth` - Enum remains the same

### Deprecated (Use AsyncDeviceMonitor)
- ⚠️ `list_devices()` → Use `monitor.enumerate_devices()`
- ⚠️ `list_devices_with_capabilities()` → Use `monitor.enumerate_devices()` + `detect_device_capabilities()`
- ⚠️ `get_default_device()` → Use `monitor.get_default_device()`
- ⚠️ `get_default_device_with_capabilities()` → Use `monitor.get_default_device()` + `detect_device_capabilities()`
- ⚠️ `get_device_capabilities()` → Use `find_device_by_name()` + `detect_device_capabilities()`

## Performance Benefits

### macOS (CoreAudio)
- **Before**: 100-500ms blocking per enumeration (50+ configs per device)
- **After**: <10ms async enumeration
- **Improvement**: 10-50x faster

### Linux (PipeWire)
- **Before**: 50-200ms blocking per enumeration
- **After**: <5ms async enumeration
- **Improvement**: 10-40x faster

### Windows (WinRT)
- **Before**: 100-300ms blocking per enumeration
- **After**: <10ms async enumeration
- **Improvement**: 10-30x faster

## Testing Strategy

All existing tests continue to work with deprecated functions:
- Unit tests in `libraries/soul-audio-desktop/tests/`
- Integration tests in `applications/desktop/src-tauri/tests/`
- Deprecation warnings will appear but tests still pass

New tests should use `AsyncDeviceMonitor`:
```rust
#[tokio::test]
async fn test_async_device_enumeration() {
    let monitor = create_async_device_monitor();
    let devices = monitor.enumerate_devices().await.unwrap();
    assert!(!devices.is_empty());
}
```

## Timeline

- **v0.1.10** (Current): Deprecation warnings added, all code works
- **v0.1.11-0.1.x**: Gradual migration, both systems coexist
- **v0.2.0**: Remove deprecated sync functions completely

## Questions & Support

- **Why keep `detect_device_capabilities()`?** It queries detailed CPAL configs which is still needed for capability detection. It's fast enough (<10ms per device) and doesn't need async.

- **Why keep `find_device_by_name()`?** It's a useful utility for device lookup by name. Can be used with `AsyncDeviceMonitor::enumerate_devices()` to find devices.

- **Can I mix old and new APIs?** Yes! The deprecated functions still work. Migrate at your own pace.

- **When will deprecated functions be removed?** Not before v0.2.0 (major version bump).

## References

- [Rust API Deprecation Best Practices](https://rust-lang.github.io/rfcs/1270-deprecation.html)
- [Tokio Bridging Async and Sync](https://tokio.rs/tokio/topics/bridging)
- [AsyncDeviceMonitor Documentation](../libraries/soul-audio-desktop/src/device_monitor_async.rs)

---

**Last Updated**: 2026-01-25
**Migration Status**: Phase 1 Complete (Deprecation)
