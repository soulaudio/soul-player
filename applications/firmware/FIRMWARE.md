# Soul Player ESP32-S3 DAP Firmware

**Digital Audio Player (DAP) Firmware** - Embedded application for ESP32-S3 hardware platform.

---

## Overview

This firmware implements a standalone DAP with:
- **E-ink display** for battery-efficient UI
- **Physical controls** (buttons + optional rotary encoder)
- **I2S DAC** output via headphone jack
- **SD card storage** for music library and database
- **USB-C charging** with battery management
- **Bluetooth LE** for pairing, sync, and OTA updates
- **Offline-first** operation with optional server sync

**Runtime**: Embassy async executor on ESP32-S3 (Xtensa, std support)

---

## Hardware Specifications

### Target Platform
- **MCU**: ESP32-S3-WROOM-1 (16MB Flash, 8MB PSRAM)
- **CPU**: Dual-core Xtensa LX7 @ 240MHz
- **RAM**: 512KB SRAM + 8MB PSRAM
- **Flash**: 16MB (partitioned for dual OTA)

### Peripherals
| Component | Interface | Pin Allocation | Driver Crate |
|-----------|-----------|----------------|--------------|
| E-ink Display (2.13" 250x122) | SPI | MOSI=11, SCK=12, CS=10, DC=9, RST=8, BUSY=7 | `epd-waveshare` |
| I2S DAC (PCM5102A) | I2S | BCK=4, WS=5, DATA=6 | `esp-hal::i2s` |
| SD Card | SDMMC 4-bit | CMD=15, CLK=14, D0-D3=2,4,12,13 | `embedded-sdmmc` |
| Buttons (6x) | GPIO | Play/Pause=18, Next=19, Prev=20, Vol+/=21,22, Menu=23 | `esp-hal::gpio` |
| Rotary Encoder (optional) | GPIO | A=24, B=25 | Custom debouncing |
| Battery Charger (BQ24072) | I2C | SDA=26, SCL=27 | Custom driver |
| Fuel Gauge (MAX17048) | I2C | SDA=26, SCL=27 | `max17048` |
| USB-C | USB OTG | D+/D- (internal) | `esp-hal::usb_serial_jtag` |
| Bluetooth LE | Internal | - | `esp-wifi::ble` |

### Power Budget
- **Active playback**: ~150mA @ 3.7V (~0.55W)
- **Idle (display on)**: ~80mA @ 3.7V (~0.30W)
- **Deep sleep**: <1mA @ 3.7V (<4mW)
- **Battery**: 1500mAh LiPo (3.7V) = ~10 hours playback

---

## Architecture

### Crate Structure
```
applications/firmware/
├── src/
│   ├── main.rs              # Embassy executor + hardware initialization
│   ├── config.rs            # Compile-time config (pins, buffer sizes)
│   │
│   ├── drivers/             # Hardware abstraction layer
│   │   ├── mod.rs
│   │   ├── display/         # E-ink display driver
│   │   │   ├── mod.rs       # Display controller
│   │   │   ├── commands.rs  # E-ink command set
│   │   │   └── framebuffer.rs
│   │   ├── audio/           # I2S audio output
│   │   │   ├── mod.rs
│   │   │   └── output.rs    # Implements soul-audio-embedded::AudioOutput
│   │   ├── storage/         # SD card FAT32 filesystem
│   │   │   ├── mod.rs
│   │   │   └── sdmmc.rs
│   │   ├── input/           # Button and encoder input
│   │   │   ├── mod.rs
│   │   │   ├── buttons.rs   # Debounced button driver
│   │   │   └── encoder.rs   # Rotary encoder driver
│   │   ├── power/           # Battery and charging management
│   │   │   ├── mod.rs
│   │   │   ├── charger.rs   # BQ24072 charger IC
│   │   │   └── fuel_gauge.rs # MAX17048 battery monitor
│   │   └── bluetooth/       # BLE stack wrapper
│   │       ├── mod.rs
│   │       ├── gatt.rs      # GATT services (media control, sync, OTA)
│   │       └── ota.rs       # Bluetooth OTA update handler
│   │
│   ├── services/            # Application business logic
│   │   ├── mod.rs
│   │   ├── playback.rs      # Playback state machine (play, pause, seek, queue)
│   │   ├── ui.rs            # UI state management and navigation
│   │   ├── library.rs       # Integration with soul-storage (SQLite on SD)
│   │   └── sync.rs          # BLE sync with phone/server
│   │
│   ├── tasks/               # Embassy async tasks
│   │   ├── mod.rs
│   │   ├── audio_task.rs    # High-priority audio playback loop
│   │   ├── display_task.rs  # UI rendering and refresh management
│   │   ├── input_task.rs    # Button/encoder event processing
│   │   ├── power_task.rs    # Battery monitoring and power management
│   │   └── bluetooth_task.rs # BLE advertising, pairing, data sync
│   │
│   ├── state/               # Shared application state
│   │   ├── mod.rs
│   │   └── app_state.rs     # Global state with Embassy channels/mutexes
│   │
│   └── boot/                # Bootloader and OTA integration
│       ├── mod.rs
│       └── ota.rs           # OTA update verification and boot partition switch
│
├── .cargo/
│   └── config.toml          # Xtensa target configuration
├── Cargo.toml               # Firmware dependencies
├── sdkconfig.defaults       # ESP-IDF configuration
├── partitions.csv           # Flash partition table
└── FIRMWARE.md              # This file
```

### Task Communication Model

```
┌─────────────────┐
│  Input Task     │ → InputEvent → ┐
│  (Buttons/Enc)  │                │
└─────────────────┘                │
                                   ↓
┌─────────────────┐         ┌──────────────┐
│  Bluetooth Task │ ←──────→│  App State   │
│  (BLE Sync)     │         │  (Channels)  │
└─────────────────┘         └──────────────┘
                                   │
┌─────────────────┐                │
│  Power Task     │ ← PowerStatus ←┤
│  (Battery Mon)  │                │
└─────────────────┘                │
                                   │
┌─────────────────┐                │
│  Display Task   │ ← UiUpdate ← ──┤
│  (E-ink Render) │                │
└─────────────────┘                │
                                   │
┌─────────────────┐                │
│  Audio Task     │ ← AudioCmd ← ──┘
│  (Playback)     │
└─────────────────┘
```

**Communication Primitives**:
- `embassy_sync::channel::Channel` - Multi-producer, multi-consumer queues
- `embassy_sync::signal::Signal` - Single-value notification
- `embassy_sync::mutex::Mutex` - Critical section-based mutual exclusion

---

## Flash Partition Layout

**Total Flash**: 16MB

```
┌────────────────────────────────────────┐ 0x000000
│  Bootloader (2nd stage)                │ 32KB
├────────────────────────────────────────┤ 0x008000
│  Partition Table                       │ 4KB
├────────────────────────────────────────┤ 0x009000
│  NVS (Non-Volatile Storage)            │ 24KB (WiFi, BLE, settings)
├────────────────────────────────────────┤ 0x00F000
│  PHY Init Data                         │ 4KB
├────────────────────────────────────────┤ 0x010000
│  Factory App (fallback firmware)       │ 1.5MB
├────────────────────────────────────────┤ 0x190000
│  OTA Partition 0                       │ 1.5MB
├────────────────────────────────────────┤ 0x310000
│  OTA Partition 1                       │ 1.5MB
├────────────────────────────────────────┤ 0x490000
│  SPIFFS (config, cache)                │ 512KB
└────────────────────────────────────────┘ 0x510000
  (Remaining ~11MB unused for future expansion)
```

**OTA Strategy**: Dual-partition ping-pong updates (factory app as recovery).

---

## Dependencies

### Cargo.toml
```toml
[dependencies]
# Embassy async runtime
embassy-executor = { version = "0.6", features = ["arch-xtensa", "executor-thread"] }
embassy-time = { version = "0.3", features = ["generic-queue"] }
embassy-sync = "0.6"
embassy-futures = "0.1"

# ESP32-S3 HAL
esp-hal = { version = "0.20", features = ["esp32s3"] }
esp-hal-embassy = "0.3"
esp-backtrace = { version = "0.14", features = ["esp32s3", "exception-handler", "panic-handler", "println"] }
esp-println = { version = "0.11", features = ["esp32s3", "log"] }
esp-alloc = "0.4"

# Peripherals
esp-wifi = { version = "0.9", features = ["esp32s3", "ble", "embassy-net"] }
embedded-sdmmc = "0.8"          # FAT32 SD card filesystem
embedded-graphics = "0.8"       # 2D graphics primitives
epd-waveshare = "0.6"           # E-ink display driver

# Soul Player library crates
soul-core = { path = "../../libraries/soul-core" }
soul-audio = { path = "../../libraries/soul-audio" }
soul-audio-embedded = { path = "../../libraries/soul-audio-embedded" }
soul-storage = { path = "../../libraries/soul-storage" }
soul-metadata = { path = "../../libraries/soul-metadata" }

# Utilities
heapless = "0.8"                # No-alloc collections (Vec, String, etc.)
static_cell = "2.1"             # Static mutable cell for Embassy
critical-section = "1.1"        # Critical section primitives
log = "0.4"                     # Logging facade
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde-json-core = "0.6"         # No-alloc JSON (for BLE payloads)

# OTA
esp-ota = "0.1"                 # OTA update helper

[profile.release]
opt-level = "z"                  # Optimize for size
lto = true                       # Link-time optimization
codegen-units = 1                # Better optimization
overflow-checks = false          # Reduce binary size
strip = true                     # Strip debug symbols
```

---

## Implementation Roadmap

### Phase 1: Core Drivers (Weeks 1-4)
**Goal**: Basic hardware functionality verified

- [ ] **Week 1: Embassy Setup**
  - Embassy executor with timer interrupt
  - UART logging via `esp-println`
  - LED blink task (sanity check)
  - Heap allocation setup (128KB)
  - Panic handler with stack trace

- [ ] **Week 2: SD Card Driver**
  - SDMMC 4-bit interface initialization
  - FAT32 filesystem mount (`embedded-sdmmc`)
  - File read/write tests (create test.txt)
  - Directory listing
  - Integration with soul-storage (SQLite on SD card)

- [ ] **Week 3: I2S Audio Output**
  - I2S peripheral configuration (PCM5102A DAC)
  - DMA buffer setup (double buffering)
  - Test WAV playback (440Hz sine wave)
  - Volume control (digital attenuation)
  - Integration with soul-audio-embedded

- [ ] **Week 4: E-ink Display**
  - SPI interface to e-ink controller
  - Full refresh test (display logo)
  - Partial refresh for dynamic content
  - `embedded-graphics` integration (text, shapes)
  - Framebuffer double buffering

### Phase 2: Input & Power (Weeks 5-6)
**Goal**: User interaction and battery management

- [ ] **Week 5: Button Input System**
  - GPIO interrupt-based button driver
  - Software debouncing (50ms)
  - Long-press detection (500ms threshold)
  - Event queue with `embassy-sync::channel`
  - Rotary encoder support (quadrature decoding)

- [ ] **Week 6: Power Management**
  - I2C communication with BQ24072 charger
  - Battery percentage reading (MAX17048 fuel gauge)
  - USB-C charge detection
  - Low battery warning (< 20%)
  - Deep sleep mode with GPIO wake-up
  - Watchdog timer configuration

### Phase 3: Application Logic (Weeks 7-9)
**Goal**: Functional music player

- [ ] **Week 7: Playback Service**
  - Playback state machine (stopped, playing, paused)
  - Queue management (next, previous, shuffle)
  - Track seeking (timestamp-based)
  - Integration with soul-audio decoder (Symphonia)
  - Gapless playback support

- [ ] **Week 8: UI Service**
  - Navigation state machine (home, library, now playing, settings)
  - Menu rendering on e-ink
  - Input event → UI action mapping
  - Track info display (artist, album, time)
  - Battery/charging status indicator

- [ ] **Week 9: Library Integration**
  - SQLite database on SD card (`/soul-player/library.db`)
  - Track browsing (artists, albums, playlists)
  - Metadata reading from audio files
  - Playlist management
  - Search functionality (limited by e-ink UI)

### Phase 4: Connectivity (Weeks 10-12)
**Goal**: Bluetooth pairing and updates

- [ ] **Week 10: Bluetooth LE Stack**
  - BLE advertising (device name: "Soul Player")
  - Pairing and bonding
  - GATT service: Media Control (play, pause, skip)
  - GATT service: Now Playing info (track, artist, album)
  - Battery level characteristic

- [ ] **Week 11: Sync Protocol**
  - BLE sync with phone app (playlist sync)
  - Integration with soul-sync library
  - Conflict resolution (last-write-wins)
  - Background sync task

- [ ] **Week 12: OTA Updates**
  - BLE OTA service (firmware upload in chunks)
  - SD card OTA (place `firmware.bin` on SD)
  - Firmware verification (SHA256 checksum)
  - Dual-partition bootloader switching
  - Rollback on boot failure (factory app)

### Phase 5: Polish & Optimization (Weeks 13-14)
**Goal**: Production-ready firmware

- [ ] **Week 13: Power Optimization**
  - CPU frequency scaling (idle: 80MHz, playback: 240MHz)
  - E-ink partial refresh optimization
  - Deep sleep when idle (>30s no input)
  - Bluetooth low-power modes
  - Audio buffer tuning (minimize wake-ups)

- [ ] **Week 14: Testing & Debugging**
  - Unit tests for drivers (mock hardware)
  - Integration tests with real hardware
  - Stress testing (24h continuous playback)
  - Battery endurance test (discharge curve)
  - Memory leak detection (heap profiling)
  - Crash logging (persist panic info to NVS)

---

## Best Practices

### 1. Memory Management

**Heap Allocation**:
- Use `esp-alloc` with 128KB heap (sufficient for most tasks)
- Avoid allocations in audio callback (use pre-allocated buffers)
- Prefer `heapless::Vec` and `heapless::String` for bounded data

**Static Allocation**:
```rust
use static_cell::StaticCell;

static AUDIO_BUFFER: StaticCell<[i16; 4096]> = StaticCell::new();

fn init() {
    let buffer = AUDIO_BUFFER.init([0i16; 4096]);
    // Use buffer in audio task
}
```

**Stack Sizes**:
```rust
#[embassy_executor::task(pool_size = 1, stack_size = 8192)]
async fn audio_task() { /* High stack for decoder */ }

#[embassy_executor::task(pool_size = 1, stack_size = 4096)]
async fn display_task() { /* Medium stack */ }
```

### 2. Task Priorities

Embassy doesn't have built-in priorities, but you can use interrupt priorities:

```rust
// Audio I2S interrupt = highest priority
esp_hal::interrupt::enable(
    esp_hal::peripherals::Interrupt::I2S0,
    esp_hal::interrupt::Priority::Priority15
);

// Button GPIO interrupt = medium priority
esp_hal::interrupt::enable(
    esp_hal::peripherals::Interrupt::GPIO,
    esp_hal::interrupt::Priority::Priority5
);
```

**Task Design**:
- **Audio task**: Lock-free, real-time critical
- **Display task**: Low priority, throttled to 1 FPS
- **Input task**: Event-driven, debounced
- **Bluetooth task**: Background, low priority

### 3. Error Handling

**Library Code** (use `Result`):
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("I2S DMA error: {0}")]
    DmaError(String),
    #[error("Decoder error: {0}")]
    DecoderError(#[from] soul_audio::DecoderError),
}

pub fn play_track(path: &str) -> Result<(), AudioError> {
    // ...
}
```

**Application Code** (log and recover):
```rust
if let Err(e) = play_track(path) {
    log::error!("Playback failed: {}", e);
    // Show error on display, skip to next track
}
```

**Panic Handler** (persist crash info):
```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Write to NVS or RTC RAM
    nvs_write("last_panic", format!("{}", info));
    esp_hal::reset::software_reset();
}
```

### 4. Power Optimization

**CPU Frequency Scaling**:
```rust
use esp_hal::clock::CpuClock;

// Idle: 80MHz
esp_hal::clock::set_cpu_clock(CpuClock::Clock80MHz);

// Playback: 240MHz (for decoding)
esp_hal::clock::set_cpu_clock(CpuClock::Clock240MHz);
```

**Deep Sleep**:
```rust
use esp_hal::rtc_cntl::sleep::{deep_sleep, WakeupSource};

// Sleep until button press
deep_sleep(&[
    WakeupSource::Gpio(GPIO_PLAY_BUTTON, WakeupLevel::Low)
]);
```

**E-ink Optimization**:
- Use partial refresh for playback time (< 300ms)
- Full refresh only every 15 partial updates (clear ghosting)
- Deep sleep display when idle (ultra-low power)

### 5. Testing Strategy

**Unit Tests** (with mocks):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_debounce() {
        let mut driver = ButtonDriver::new_mock();
        // Simulate button bouncing
        assert_eq!(driver.read_event(), None);
    }
}
```

**Integration Tests** (on hardware):
```rust
#[embassy_executor::test]
async fn test_audio_playback() {
    let mut audio = I2sAudioOutput::new(/* ... */);
    let samples = [0i16; 1024];
    assert!(audio.write_samples(&samples).is_ok());
}
```

**Hardware-in-Loop Tests**:
- Use `probe-rs` for debugging and RTT logging
- Automated test rig with GPIO control (simulate button presses)
- Audio output verification (capture DAC output, analyze waveform)

### 6. Logging and Debugging

**Logging Levels**:
```rust
log::trace!("Audio buffer refilled");      // Verbose, disabled in release
log::debug!("Track changed: {}", title);   // Development only
log::info!("Bluetooth connected");         // Important events
log::warn!("Battery low: {}%", percent);   // Warnings
log::error!("I2S DMA underrun!");          // Errors
```

**Real-Time Transfer (RTT)**:
```rust
// Use probe-rs for zero-overhead logging
rtt_target::rtt_init_print!();
rprintln!("Audio task started");
```

**Crash Dumps**:
- Store panic info in RTC RAM (survives reboot)
- On next boot, upload crash log via BLE
- Implement watchdog reset detection

### 7. Security Considerations

**BLE Pairing**:
- Use "Just Works" pairing for UX simplicity
- OR use passkey display on e-ink (6-digit PIN)
- Bonding: Store pairing keys in NVS (encrypted)

**OTA Security**:
- Firmware must be signed (ESP32 secure boot)
- Verify SHA256 checksum before flashing
- Rollback on boot failure (3 failed boots → factory app)

**SD Card**:
- FAT32 is inherently insecure (no encryption)
- Future: Encrypt SQLite database with user PIN

---

## Development Workflow

### Initial Setup
```bash
# Install ESP toolchain
espup install

# Install tools
cargo install cargo-espflash espflash probe-rs

# Set environment
. $HOME/export-esp.sh
```

### Build and Flash
```bash
# Build firmware
cd applications/firmware
cargo build --release

# Flash to device (USB-C connected)
espflash flash target/xtensa-esp32s3-espidf/release/soul-player-esp32

# Monitor serial output
espflash monitor
```

### Debug with probe-rs
```bash
# Requires JTAG debugger (ESP-Prog or built-in USB-JTAG)
probe-rs run --chip esp32s3 target/xtensa-esp32s3-espidf/release/soul-player-esp32

# RTT logging
probe-rs attach --chip esp32s3 --protocol swd
```

### OTA Update (Development)
```bash
# Generate OTA binary
esptool.py --chip esp32s3 merge_bin -o firmware.bin \
    --flash_mode dio --flash_size 16MB \
    0x0 bootloader.bin \
    0x8000 partition-table.bin \
    0x10000 soul-player-esp32.bin

# Copy to SD card
cp firmware.bin /media/sdcard/soul-player/ota/firmware.bin

# Or upload via BLE (use companion app)
```

---

## Hardware Bring-Up Checklist

### Phase 1: Power-On
- [ ] 3.3V rail stable (measure with multimeter)
- [ ] ESP32-S3 boots (LED blink via GPIO)
- [ ] USB-C enumeration (appears as USB-JTAG device)
- [ ] UART output visible (115200 baud)

### Phase 2: Peripherals
- [ ] E-ink display initializes (full refresh shows test pattern)
- [ ] SD card detected (read/write test file)
- [ ] I2S DAC outputs audio (1kHz test tone)
- [ ] All buttons readable (GPIO states)
- [ ] Battery charger IC responds on I2C
- [ ] Fuel gauge IC responds on I2C

### Phase 3: Validation
- [ ] Continuous playback (MP3 file, 30+ minutes)
- [ ] Battery charge cycle (0% → 100%)
- [ ] Deep sleep current draw (< 1mA)
- [ ] Bluetooth pairing with phone
- [ ] OTA update successful (SD card method)

---

## Troubleshooting

### Issue: ESP32 won't boot
- Check 3.3V power rail
- Verify GPIO0 is HIGH at boot (not in download mode)
- Flash factory app with `esptool.py`

### Issue: SD card not detected
- Verify SDMMC pins (CMD, CLK, D0-D3)
- Check SD card format (must be FAT32)
- Try lower clock speed (400kHz for init)

### Issue: I2S audio distorted
- Check sample rate matches decoder output
- Verify I2S bit depth (16-bit vs 24-bit)
- Increase DMA buffer size (reduce underruns)

### Issue: E-ink ghosting
- Force full refresh every 15 partial updates
- Check temperature compensation (e-ink spec)
- Verify waveform LUT matches display model

### Issue: High power consumption
- Profile with ammeter on battery line
- Check for busy-wait loops (use `Timer::after()`)
- Ensure peripherals enter sleep mode

---

## Future Enhancements

### Phase 6 (Post-MVP)
- [ ] WiFi streaming (Spotify Connect, AirPlay)
- [ ] Equalizer (3-band → 10-band parametric)
- [ ] Gapless playback (crossfade)
- [ ] Lyrics display (synced LRC files)
- [ ] Firmware A/B testing (canary releases)
- [ ] USB DAC mode (use ESP32 as USB audio device)
- [ ] DSD support (if Symphonia adds support)
- [ ] Custom e-ink UI framework (vs embedded-graphics)

---

## References

### Documentation
- [ESP32-S3 TRM](https://www.espressif.com/sites/default/files/documentation/esp32-s3_technical_reference_manual_en.pdf)
- [Embassy Book](https://embassy.dev/book/)
- [esp-hal Documentation](https://docs.esp-rs.org/esp-hal/)
- [embedded-graphics](https://docs.rs/embedded-graphics/)

### Example Projects
- [esp-hal examples](https://github.com/esp-rs/esp-hal/tree/main/examples)
- [Embassy ESP32 examples](https://github.com/embassy-rs/embassy/tree/main/examples/esp32s3)
- [Awesome ESP Rust](https://github.com/esp-rs/awesome-esp-rust)

### Hardware Datasheets
- PCM5102A DAC: [TI Datasheet](https://www.ti.com/lit/ds/symlink/pcm5102a.pdf)
- BQ24072 Charger: [TI Datasheet](https://www.ti.com/lit/ds/symlink/bq24072.pdf)
- MAX17048 Fuel Gauge: [Analog Devices](https://www.analog.com/media/en/technical-documentation/data-sheets/MAX17048-MAX17049.pdf)
- 2.13" E-ink Display: [Waveshare Wiki](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT)

---

## Contributing

When adding firmware features:
1. Update this FIRMWARE.md with new drivers/tasks
2. Document pin allocations in `config.rs`
3. Add hardware test procedures
4. Update power budget calculations
5. Verify cross-platform compatibility (soul-audio, soul-storage)

---

**Last Updated**: 2026-01-05
**Firmware Version**: 0.1.0-dev
**Target Hardware**: ESP32-S3-WROOM-1 (16MB/8MB)
