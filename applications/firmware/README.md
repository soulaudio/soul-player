# Soul Player ESP32-S3 Firmware

Embedded firmware for a standalone Digital Audio Player (DAP) running on ESP32-S3 with Embassy async runtime.

## Quick Start

### Prerequisites

1. **Install Rust and ESP toolchain**:
```bash
# Install rustup if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install ESP Rust toolchain
cargo install espup
espup install

# Source the environment (add to your shell rc file)
. $HOME/export-esp.sh
```

2. **Install flashing tools**:
```bash
cargo install cargo-espflash espflash ldproxy
```

3. **Verify installation**:
```bash
rustc --version
espflash --version
```

### Building

```bash
# Navigate to firmware directory
cd applications/firmware

# Build the firmware (debug mode)
cargo build

# Build the firmware (release mode, optimized for size)
cargo build --release
```

### Flashing to Device

**Note**: Without hardware, you can only build the firmware. When you have the hardware:

```bash
# Flash and monitor (auto-detects USB port)
cargo run --release

# Or use espflash directly
espflash flash target/xtensa-esp32s3-espidf/release/soul-player-esp32 --monitor

# Flash to specific port
espflash flash --port /dev/ttyUSB0 target/xtensa-esp32s3-espidf/release/soul-player-esp32
```

### Monitoring Serial Output

```bash
# Monitor serial output (UART)
espflash monitor

# Or use screen/minicom
screen /dev/ttyUSB0 115200
```

## Project Structure

```
applications/firmware/
├── src/
│   ├── main.rs              # Embassy executor + initialization
│   ├── config.rs            # Hardware pin definitions
│   ├── drivers/             # Hardware abstraction layer
│   ├── services/            # Application business logic
│   ├── tasks/               # Embassy async tasks
│   ├── state/               # Shared state and channels
│   └── boot/                # OTA and bootloader support
├── .cargo/
│   └── config.toml          # Xtensa target configuration
├── Cargo.toml               # Dependencies
├── build.rs                 # Build script
├── sdkconfig.defaults       # ESP-IDF configuration
├── partitions.csv           # Flash partition table
├── FIRMWARE.md              # Detailed architecture documentation
└── README.md                # This file
```

## Current Status

### Implemented
- ✅ Embassy async executor setup
- ✅ Heap allocation (128KB)
- ✅ Heartbeat task (LED blink)
- ✅ UART logging
- ✅ Module structure (drivers, services, tasks, state, boot)
- ✅ Pin configuration (config.rs)
- ✅ Flash partition table
- ✅ ESP-IDF configuration

### To Be Implemented (see FIRMWARE.md for roadmap)
- ⏳ SD card driver (SDMMC 4-bit)
- ⏳ I2S audio output (PCM5102A DAC)
- ⏳ E-ink display driver (2.13" SPI)
- ⏳ Button input handling
- ⏳ Power management (battery, charging)
- ⏳ Bluetooth LE stack
- ⏳ Audio playback service
- ⏳ UI navigation
- ⏳ OTA updates

## Hardware Configuration

**Target MCU**: ESP32-S3-WROOM-1 (16MB Flash, 8MB PSRAM)

### Pin Assignments

See `src/config.rs` for complete pin definitions. Key peripherals:

| Peripheral | Interface | Pins |
|------------|-----------|------|
| E-ink Display | SPI | MOSI=11, SCK=12, CS=10, DC=9, RST=8, BUSY=7 |
| I2S DAC | I2S | BCK=4, WS=5, DATA=6 |
| SD Card | SDMMC | CMD=15, CLK=14, D0-D3=2,4,12,13 |
| Buttons | GPIO | 18-23 |
| Power (I2C) | I2C | SDA=26, SCL=27 |
| Onboard LED | GPIO | 48 |

### Flash Partitions

| Partition | Size | Purpose |
|-----------|------|---------|
| NVS | 24KB | Settings, BLE keys |
| PHY Init | 4KB | WiFi/BLE calibration |
| Factory | 1.5MB | Fallback firmware |
| OTA 0 | 1.5MB | Primary app |
| OTA 1 | 1.5MB | Secondary app (updates) |
| SPIFFS | 512KB | Config files |

## Development Workflow

### Without Hardware (Current Status)

```bash
# Build to verify code compiles
cargo build --release

# Check for errors
cargo clippy --all-targets

# Format code
cargo fmt --all
```

### With Hardware (Future)

```bash
# Flash and run
cargo run --release

# View logs
espflash monitor

# Debug with probe-rs (requires JTAG debugger)
probe-rs run --chip esp32s3 target/xtensa-esp32s3-espidf/release/soul-player-esp32
```

## Adding New Features

### Adding a Driver

1. Create module in `src/drivers/`:
```bash
touch src/drivers/my_driver.rs
```

2. Add to `src/drivers/mod.rs`:
```rust
pub mod my_driver;
```

3. Implement the driver:
```rust
// src/drivers/my_driver.rs
pub struct MyDriver {
    // ...
}

impl MyDriver {
    pub fn new() -> Self {
        // ...
    }
}
```

4. Update `main.rs` to initialize and use the driver.

### Adding a Task

1. Create module in `src/tasks/`:
```bash
touch src/tasks/my_task.rs
```

2. Add to `src/tasks/mod.rs`:
```rust
pub mod my_task;
```

3. Implement the task:
```rust
// src/tasks/my_task.rs
use embassy_executor;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn my_task() {
    loop {
        // Task logic here
        Timer::after(Duration::from_secs(1)).await;
    }
}
```

4. Spawn task in `main.rs`:
```rust
spawner.spawn(tasks::my_task::my_task()).unwrap();
```

## Troubleshooting

### Build Errors

**Error: `espup` not found**
```bash
cargo install espup
espup install
. $HOME/export-esp.sh
```

**Error: `xtensa-esp32s3-espidf` target not found**
```bash
# Re-install ESP toolchain
espup install
```

**Error: Linker errors**
```bash
# Clean build artifacts
cargo clean
# Rebuild
cargo build --release
```

### Runtime Issues (with hardware)

**LED not blinking**:
- Check GPIO48 is correct for your board (some use GPIO2 or GPIO13)
- Modify `config::led::PIN_LED` in `src/config.rs`

**Serial output not visible**:
- Check baud rate (should be 115200)
- Try different USB port
- Verify ESP32-S3 is in run mode (not boot mode)

**Flash fails**:
- Press and hold BOOT button while connecting USB
- Release BOOT button after flash starts
- Some boards auto-enter flash mode

## Resources

- **ESP32-S3 Documentation**: https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/
- **Embassy Book**: https://embassy.dev/book/
- **esp-hal Documentation**: https://docs.esp-rs.org/esp-hal/
- **Firmware Architecture**: See `FIRMWARE.md`
- **Project Roadmap**: See `../../ROADMAP.md`

## Contributing

When adding firmware features:
1. Update `FIRMWARE.md` with new drivers/tasks
2. Document pin allocations in `config.rs`
3. Test on real hardware (when available)
4. Add comments explaining hardware-specific quirks

## License

See project root LICENSE file.
