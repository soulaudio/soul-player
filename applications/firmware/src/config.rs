//! Hardware configuration and pin definitions
//!
//! This module contains all hardware-specific configuration:
//! - GPIO pin assignments
//! - I2C/SPI/I2S bus configurations
//! - Audio buffer sizes
//! - Display refresh settings
//! - Power management thresholds

// ============================================================================
// GPIO Pin Assignments
// ============================================================================

/// E-ink Display Pins (SPI)
pub mod display {
    /// SPI MOSI (Master Out Slave In)
    pub const PIN_MOSI: u8 = 11;

    /// SPI Clock
    pub const PIN_SCK: u8 = 12;

    /// Chip Select (active low)
    pub const PIN_CS: u8 = 10;

    /// Data/Command select (0 = command, 1 = data)
    pub const PIN_DC: u8 = 9;

    /// Reset (active low)
    pub const PIN_RST: u8 = 8;

    /// Busy indicator from display (high = busy)
    pub const PIN_BUSY: u8 = 7;

    /// SPI clock frequency (Hz) - E-ink can typically handle 10-20MHz
    pub const SPI_FREQUENCY_HZ: u32 = 10_000_000;
}

/// I2S Audio DAC Pins (PCM5102A or similar)
pub mod audio {
    /// Bit Clock (BCK)
    pub const PIN_BCK: u8 = 4;

    /// Word Select / LRCLK (WS)
    pub const PIN_WS: u8 = 5;

    /// Data Output
    pub const PIN_DATA: u8 = 6;

    /// I2S sample rate (Hz)
    pub const DEFAULT_SAMPLE_RATE: u32 = 44100;

    /// I2S bit depth
    pub const BIT_DEPTH: u8 = 16;

    /// DMA buffer size in samples (per channel)
    /// Larger buffers reduce CPU wake-ups but increase latency
    pub const DMA_BUFFER_SIZE: usize = 4096;

    /// Number of DMA buffers (double buffering recommended)
    pub const DMA_BUFFER_COUNT: usize = 2;
}

/// SD Card Pins (SDMMC 4-bit mode)
pub mod sdcard {
    /// Command pin
    pub const PIN_CMD: u8 = 15;

    /// Clock pin
    pub const PIN_CLK: u8 = 14;

    /// Data pin 0
    pub const PIN_D0: u8 = 2;

    /// Data pin 1
    pub const PIN_D1: u8 = 4;

    /// Data pin 2
    pub const PIN_D2: u8 = 12;

    /// Data pin 3
    pub const PIN_D3: u8 = 13;

    /// SDMMC clock frequency (Hz) - 20MHz is a good balance
    pub const SDMMC_FREQUENCY_HZ: u32 = 20_000_000;

    /// Mount point for SD card filesystem
    pub const MOUNT_POINT: &str = "/sdcard";

    /// Database path on SD card
    pub const DATABASE_PATH: &str = "/sdcard/soul-player/library.db";
}

/// Button Input Pins
pub mod buttons {
    /// Play/Pause button
    pub const PIN_PLAY_PAUSE: u8 = 18;

    /// Next track button
    pub const PIN_NEXT: u8 = 19;

    /// Previous track button
    pub const PIN_PREV: u8 = 20;

    /// Volume up button
    pub const PIN_VOL_UP: u8 = 21;

    /// Volume down button
    pub const PIN_VOL_DOWN: u8 = 22;

    /// Menu/Select button
    pub const PIN_MENU: u8 = 23;

    /// Debounce time in milliseconds
    pub const DEBOUNCE_MS: u64 = 50;

    /// Long press threshold in milliseconds
    pub const LONG_PRESS_MS: u64 = 500;
}

/// Rotary Encoder Pins (optional)
pub mod encoder {
    /// Encoder pin A (quadrature)
    pub const PIN_A: u8 = 24;

    /// Encoder pin B (quadrature)
    pub const PIN_B: u8 = 25;

    /// Encoder button (push to select)
    pub const PIN_BUTTON: u8 = 26;
}

/// Power Management Pins (I2C)
pub mod power {
    /// I2C SDA (shared with charger and fuel gauge)
    pub const PIN_SDA: u8 = 26;

    /// I2C SCL (shared with charger and fuel gauge)
    pub const PIN_SCL: u8 = 27;

    /// I2C clock frequency (Hz) - 100kHz is standard for power ICs
    pub const I2C_FREQUENCY_HZ: u32 = 100_000;

    /// BQ24072 Charger I2C address (check datasheet)
    pub const CHARGER_I2C_ADDR: u8 = 0x6B;

    /// MAX17048 Fuel Gauge I2C address
    pub const FUEL_GAUGE_I2C_ADDR: u8 = 0x36;
}

/// Onboard LED (for debugging/heartbeat)
pub mod led {
    /// Built-in LED on ESP32-S3-DevKitC-1
    pub const PIN_LED: u8 = 48;
}

// ============================================================================
// Application Configuration
// ============================================================================

/// Audio playback configuration
pub mod playback {
    /// Default volume (0-100)
    pub const DEFAULT_VOLUME: u8 = 70;

    /// Minimum volume
    pub const MIN_VOLUME: u8 = 0;

    /// Maximum volume
    pub const MAX_VOLUME: u8 = 100;

    /// Volume step for up/down buttons
    pub const VOLUME_STEP: u8 = 5;

    /// Enable gapless playback (requires more buffering)
    pub const GAPLESS_PLAYBACK: bool = true;
}

/// Display configuration
pub mod display_config {
    /// Display width in pixels
    pub const WIDTH: u32 = 250;

    /// Display height in pixels
    pub const HEIGHT: u32 = 122;

    /// Maximum partial refreshes before full refresh (prevents ghosting)
    pub const PARTIAL_REFRESH_LIMIT: u8 = 15;

    /// UI refresh rate (Hz) - E-ink is slow, 1 FPS is sufficient
    pub const REFRESH_RATE_HZ: u8 = 1;

    /// Screen timeout (seconds) - enter deep sleep after this period of inactivity
    pub const SCREEN_TIMEOUT_SECS: u64 = 30;
}

/// Power management configuration
pub mod power_config {
    /// Battery low warning threshold (%)
    pub const BATTERY_LOW_PERCENT: u8 = 20;

    /// Battery critical threshold (%) - force shutdown
    pub const BATTERY_CRITICAL_PERCENT: u8 = 5;

    /// Deep sleep timeout after inactivity (seconds)
    pub const DEEP_SLEEP_TIMEOUT_SECS: u64 = 60;

    /// CPU frequency scaling: idle frequency (MHz)
    pub const CPU_FREQ_IDLE_MHZ: u32 = 80;

    /// CPU frequency scaling: active playback frequency (MHz)
    pub const CPU_FREQ_PLAYBACK_MHZ: u32 = 240;
}

/// Bluetooth configuration
pub mod bluetooth_config {
    /// BLE device name (advertised name)
    pub const DEVICE_NAME: &str = "Soul Player";

    /// BLE connection timeout (seconds)
    pub const CONNECTION_TIMEOUT_SECS: u64 = 30;

    /// Enable BLE pairing with passkey (vs "Just Works")
    pub const USE_PASSKEY_PAIRING: bool = false;

    /// OTA update chunk size (bytes) - balance between speed and memory
    pub const OTA_CHUNK_SIZE: usize = 512;
}

/// System configuration
pub mod system {
    /// Heap size (bytes) - defined in main.rs init_heap()
    pub const HEAP_SIZE: usize = 128 * 1024; // 128KB

    /// Watchdog timeout (seconds)
    pub const WATCHDOG_TIMEOUT_SECS: u8 = 10;

    /// Enable crash logging to NVS
    pub const ENABLE_CRASH_LOGGING: bool = true;

    /// NVS namespace for application settings
    pub const NVS_NAMESPACE: &str = "soul_player";
}

// ============================================================================
// Feature Flags (compile-time configuration)
// ============================================================================

/// Enable verbose logging (increases binary size)
pub const VERBOSE_LOGGING: bool = cfg!(debug_assertions);

/// Enable audio debugging (logs buffer states, underruns, etc.)
pub const DEBUG_AUDIO: bool = cfg!(debug_assertions);

/// Enable performance profiling
pub const ENABLE_PROFILING: bool = false;

// ============================================================================
// Hardware Validation
// ============================================================================

/// Validates that pin assignments don't conflict
///
/// This is called at compile time via const evaluation.
/// If pins conflict, the build will fail with a panic.
#[allow(dead_code)]
const fn validate_pins() {
    // Add validation logic here if needed
    // For now, this is a placeholder for future pin conflict detection
}

// Run validation at compile time
#[allow(dead_code)]
const _: () = validate_pins();
